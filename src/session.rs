use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::bridge::{BridgeConfig, BridgeEvent, PortOpener, RunningBridge, Side};
use crate::logging::Logger;
use crate::parse::StartRequest;

#[derive(Debug)]
pub enum SessionError {
  Conflict,
  Invalid(String),
}

impl SessionError {
  pub fn status_code(&self) -> u16 {
    match self {
      Self::Conflict => 409,
      Self::Invalid(_) => 400,
    }
  }

  pub fn message(&self) -> &str {
    match self {
      Self::Conflict => "Sniffer is already running",
      Self::Invalid(message) => message,
    }
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Packet {
  id: u64,
  timestamp: String,
  elapsed_ms: u128,
  direction: String,
  data: Vec<u8>,
  description: String,
}

struct Inner {
  running: bool,
  computer_port: Option<String>,
  radio_port: Option<String>,
  baud_rate: Option<u32>,
  log_file: Option<String>,
  started_at: Option<String>,
  packets: Vec<Packet>,
  next_id: u64,
  rts: Option<bool>,
  dtr: Option<bool>,
  bridge: Option<RunningBridge>,
  last_log: Option<Value>,
  last_stats: Option<crate::bridge::BridgeStats>,
}

pub struct Session {
  inner: Mutex<Inner>,
  /// Serializes start so two requests cannot both open the ports.
  start_gate: Mutex<()>,
  events: broadcast::Sender<String>,
  opener: Arc<dyn PortOpener>,
  logger: Logger,
}

impl Session {
  pub fn new(opener: Arc<dyn PortOpener>, logger: Logger) -> Arc<Self> {
    let (events, _) = broadcast::channel(256);
    logger.info(&format!(
      "Sniffer logger ready level={}",
      logger.level_name()
    ));
    Arc::new(Self {
      inner: Mutex::new(Inner {
        running: false,
        computer_port: None,
        radio_port: None,
        baud_rate: None,
        log_file: None,
        started_at: None,
        packets: Vec::new(),
        next_id: 1,
        rts: None,
        dtr: None,
        bridge: None,
        last_log: None,
        last_stats: None,
      }),
      start_gate: Mutex::new(()),
      events,
      opener,
      logger,
    })
  }

  pub fn subscribe(&self) -> broadcast::Receiver<String> {
    self.events.subscribe()
  }

  pub fn status(&self) -> Value {
    let inner = self.inner.lock().expect("session");
    status_json(&inner)
  }

  pub fn packets(&self) -> Value {
    let inner = self.inner.lock().expect("session");
    json!(inner.packets)
  }

  pub fn log_response(&self) -> Value {
    let inner = self.inner.lock().expect("session");
    let path = inner.log_file.clone();
    let data = inner
      .bridge
      .as_ref()
      .and_then(RunningBridge::log_snapshot)
      .or_else(|| inner.last_log.clone());
    let mut file = serde_json::Map::new();
    if let Some(path) = path {
      file.insert("path".into(), json!(path));
    }
    if let Some(data) = data {
      file.insert("data".into(), data);
    }
    json!({
      "status": status_json(&inner),
      "packets": inner.packets,
      "file": file,
    })
  }

  pub fn start(self: &Arc<Self>, request: StartRequest) -> Result<Value, SessionError> {
    let _gate = self.start_gate.lock().expect("start");
    let config = BridgeConfig::from_request(
      request.computer_port,
      request.radio_port,
      request.baud_rate,
      request.log_file,
      request.rts,
      request.dtr,
    );

    {
      let inner = self.inner.lock().expect("session");
      if inner.running {
        return Err(SessionError::Conflict);
      }
    }

    let computer = self.open_side(Side::Computer, &config);
    let radio = self.open_side(Side::Radio, &config);
    let events = Arc::clone(self);
    let bridge = RunningBridge::spawn(
      &config,
      computer,
      radio,
      self.logger.clone(),
      Arc::new(move |event| events.on_bridge_event(event)),
    );

    let started_at = crate::serial_log::format_iso(std::time::SystemTime::now());
    let mut inner = self.inner.lock().expect("session");
    if inner.running {
      drop(inner);
      return Err(SessionError::Conflict);
    }

    inner.running = true;
    inner.computer_port = Some(config.computer_port);
    inner.radio_port = Some(config.radio_port);
    inner.baud_rate = Some(config.baud_rate);
    inner.log_file = Some(bridge.log_path());
    inner.started_at = Some(started_at);
    inner.packets.clear();
    inner.next_id = 1;
    inner.rts = Some(config.rts);
    inner.dtr = Some(config.dtr);
    inner.last_log = None;
    inner.last_stats = None;
    inner.bridge = Some(bridge);

    let status = status_json(&inner);
    drop(inner);
    self.publish(json!({ "type": "status", "status": status.clone() }));
    self.logger.info("Sniffer session started");
    Ok(status)
  }

  pub fn stop(&self) -> Value {
    let mut inner = self.inner.lock().expect("session");
    if !inner.running && inner.bridge.is_none() {
      return status_json(&inner);
    }

    let stats = inner.bridge.as_ref().map(RunningBridge::stats);
    let log_before = inner.bridge.as_ref().and_then(RunningBridge::log_snapshot);
    let mut bridge = inner.bridge.take();
    inner.running = false;
    inner.last_stats = stats;
    drop(inner);

    let log_after = bridge.as_mut().and_then(RunningBridge::stop);
    let mut inner = self.inner.lock().expect("session");
    inner.last_log = log_after.or(log_before);
    inner.bridge = None;
    let status = status_json(&inner);
    drop(inner);

    self.logger.info("Sniffer session stopped");
    self.publish(json!({ "type": "status", "status": status.clone() }));
    status
  }

  fn open_side(
    self: &Arc<Self>,
    side: Side,
    config: &BridgeConfig,
  ) -> Option<crate::bridge::PortEnds> {
    let path = match side {
      Side::Computer => config.computer_port.as_str(),
      Side::Radio => config.radio_port.as_str(),
    };
    self.logger.info(&format!(
      "Opening {} port port={path} baudRate={}",
      side.as_str(),
      config.baud_rate
    ));

    match self
      .opener
      .open(path, config.baud_rate, config.rts, config.dtr)
    {
      Ok(ends) => {
        self.logger.info(&format!(
          "{} port opened port={path} rts={} dtr={}",
          side.as_str(),
          config.rts,
          config.dtr
        ));
        Some(ends)
      }
      Err(error) => {
        self.logger.error(&format!(
          "{} port open failed port={path} error={error}",
          side.as_str()
        ));
        self.publish(json!({
          "type": "error",
          "message": error,
          "source": side.as_str(),
        }));
        None
      }
    }
  }

  fn on_bridge_event(self: &Arc<Self>, event: BridgeEvent) {
    match event {
      BridgeEvent::Packet(packet) => {
        let mut inner = self.inner.lock().expect("session");
        let recorded = Packet {
          id: inner.next_id,
          timestamp: packet.timestamp,
          elapsed_ms: packet.elapsed_ms,
          direction: packet.direction,
          data: packet.data,
          description: packet.description,
        };
        inner.next_id += 1;
        inner.packets.push(recorded.clone());
        let status = status_json(&inner);
        drop(inner);
        self.publish(json!({ "type": "packet", "packet": recorded }));
        self.publish(json!({ "type": "status", "status": status }));
      }
      BridgeEvent::Error { message, source } => {
        self.publish(json!({
          "type": "error",
          "message": message,
          "source": source.as_str(),
        }));
      }
      BridgeEvent::Stats => {
        let inner = self.inner.lock().expect("session");
        let status = status_json(&inner);
        drop(inner);
        self.publish(json!({ "type": "status", "status": status }));
      }
    }
  }

  fn publish(&self, event: Value) {
    let _ = self.events.send(event.to_string());
  }
}

fn status_json(inner: &Inner) -> Value {
  let stats = inner
    .bridge
    .as_ref()
    .map(RunningBridge::stats)
    .or(inner.last_stats);
  let mut status = serde_json::Map::new();
  status.insert("running".into(), json!(inner.running));
  if let Some(port) = &inner.computer_port {
    status.insert("computerPort".into(), json!(port));
  }
  if let Some(port) = &inner.radio_port {
    status.insert("radioPort".into(), json!(port));
  }
  if let Some(baud) = inner.baud_rate {
    status.insert("baudRate".into(), json!(baud));
  }
  if let Some(path) = &inner.log_file {
    status.insert("logFile".into(), json!(path));
  }
  if let Some(started) = &inner.started_at {
    status.insert("startedAt".into(), json!(started));
  }
  status.insert("packetCount".into(), json!(inner.packets.len()));

  if let Some(stats) = stats {
    status.insert(
      "bytesComputerToRadio".into(),
      json!(stats.bytes_computer_to_radio),
    );
    status.insert(
      "bytesRadioToComputer".into(),
      json!(stats.bytes_radio_to_computer),
    );
    status.insert("writeErrors".into(), json!(stats.write_errors));
    let computer_open = if inner.running {
      stats.computer_port_open
    } else {
      false
    };
    let radio_open = if inner.running {
      stats.radio_port_open
    } else {
      false
    };
    status.insert("computerPortOpen".into(), json!(computer_open));
    status.insert("radioPortOpen".into(), json!(radio_open));
  }

  if let Some(rts) = inner.rts {
    status.insert("rts".into(), json!(rts));
  }
  if let Some(dtr) = inner.dtr {
    status.insert("dtr".into(), json!(dtr));
  }

  Value::Object(status)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bridge::{PortEnds, PortReader, PortWriter};

  struct IdleReader;

  impl PortReader for IdleReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
      std::thread::sleep(std::time::Duration::from_millis(20));
      Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "idle"))
    }
  }

  struct Sink;

  impl PortWriter for Sink {
    fn write_all(&mut self, _buf: &[u8]) -> std::io::Result<()> {
      Ok(())
    }
  }

  struct OkOpener;

  impl PortOpener for OkOpener {
    fn open(
      &self,
      _path: &str,
      _baud_rate: u32,
      _rts: bool,
      _dtr: bool,
    ) -> Result<PortEnds, String> {
      Ok(PortEnds {
        reader: Box::new(IdleReader),
        writer: Box::new(Sink),
      })
    }
  }

  #[test]
  fn starts_once_and_rejects_a_second_start() {
    let session = Session::new(Arc::new(OkOpener), Logger::quiet());
    let request = StartRequest {
      computer_port: "/dev/computer".into(),
      radio_port: "/dev/radio".into(),
      baud_rate: Some(19200),
      log_file: Some("session-test.json".into()),
      rts: None,
      dtr: None,
    };

    let status = session.start(request.clone()).unwrap();
    assert_eq!(status["running"], true);
    assert_eq!(status["computerPort"], "/dev/computer");
    assert_eq!(status["radioPort"], "/dev/radio");
    assert_eq!(status["baudRate"], 19200);
    assert_eq!(status["packetCount"], 0);
    assert_eq!(status["rts"], true);
    assert_eq!(status["dtr"], true);

    let error = session.start(request).unwrap_err();
    assert_eq!(error.message(), "Sniffer is already running");
    assert_eq!(error.status_code(), 409);

    let stopped = session.stop();
    assert_eq!(stopped["running"], false);
    assert_eq!(stopped["computerPortOpen"], false);
    assert_eq!(stopped["radioPortOpen"], false);
    let again = session.stop();
    assert_eq!(again["running"], false);
    let _ = std::fs::remove_file("session-test.json");
  }
}
