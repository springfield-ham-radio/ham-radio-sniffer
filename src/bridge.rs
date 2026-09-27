use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use crate::logging::{format_hex, format_hex_preview, FirstSeen, Logger};
use crate::serial_log::{format_elapsed, SerialLog};

const DEFAULT_BAUD: u32 = 9600;
pub const DEFAULT_PACKET_IDLE: Duration = Duration::from_millis(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
  Computer,
  Radio,
}

impl Side {
  pub(crate) fn as_str(self) -> &'static str {
    match self {
      Self::Computer => "computer",
      Self::Radio => "radio",
    }
  }

  fn peer(self) -> Self {
    match self {
      Self::Computer => Self::Radio,
      Self::Radio => Self::Computer,
    }
  }

  fn direction(self) -> &'static str {
    match self {
      Self::Computer => "COMPUTER->RADIO",
      Self::Radio => "RADIO->COMPUTER",
    }
  }

  fn description(self) -> &'static str {
    match self {
      Self::Computer => "Computer to Radio",
      Self::Radio => "Radio to Computer",
    }
  }

  fn is_send(self) -> bool {
    matches!(self, Self::Computer)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketEvent {
  pub timestamp: String,
  pub elapsed_ms: u128,
  pub direction: String,
  pub data: Vec<u8>,
  pub description: String,
}

#[derive(Debug, Clone)]
pub enum BridgeEvent {
  Packet(PacketEvent),
  Error { message: String, source: Side },
  Stats,
}

pub trait PortReader: Send {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

pub trait PortWriter: Send {
  fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
}

pub struct PortEnds {
  pub reader: Box<dyn PortReader>,
  pub writer: Box<dyn PortWriter>,
}

pub trait PortOpener: Send + Sync {
  fn open(&self, path: &str, baud_rate: u32, rts: bool, dtr: bool) -> Result<PortEnds, String>;
}

#[derive(Debug, Clone)]
pub struct BridgeConfig {
  pub computer_port: String,
  pub radio_port: String,
  pub baud_rate: u32,
  pub log_file: String,
  pub rts: bool,
  pub dtr: bool,
  pub packet_idle: Duration,
}

impl BridgeConfig {
  pub fn from_request(
    computer_port: String,
    radio_port: String,
    baud_rate: Option<u32>,
    log_file: Option<String>,
    rts: Option<bool>,
    dtr: Option<bool>,
  ) -> Self {
    Self {
      computer_port,
      radio_port,
      baud_rate: baud_rate.unwrap_or(DEFAULT_BAUD),
      log_file: log_file.unwrap_or_else(|| crate::serial_log::default_log_path(SystemTime::now())),
      rts: rts.unwrap_or(true),
      dtr: dtr.unwrap_or(true),
      packet_idle: DEFAULT_PACKET_IDLE,
    }
  }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BridgeStats {
  pub bytes_computer_to_radio: u64,
  pub bytes_radio_to_computer: u64,
  pub write_errors: u64,
  pub computer_port_open: bool,
  pub radio_port_open: bool,
}

struct Shared {
  logger: Logger,
  started: Instant,
  packet_idle: Duration,
  computer_port: String,
  radio_port: String,
  bytes_computer_to_radio: AtomicU64,
  bytes_radio_to_computer: AtomicU64,
  write_errors: AtomicU64,
  computer_open: AtomicBool,
  radio_open: AtomicBool,
  seen_computer: FirstSeen,
  seen_radio: FirstSeen,
  shutdown: AtomicBool,
  serial_log: Mutex<SerialLog>,
  coalescer: Mutex<Coalescer>,
  coalesce_wake: Condvar,
}

struct Coalescer {
  direction: Option<Side>,
  bytes: Vec<u8>,
  started_at: Instant,
  description: String,
  deadline: Option<Instant>,
  stopped: bool,
}

pub struct RunningBridge {
  shared: Arc<Shared>,
  readers: Vec<JoinHandle<()>>,
  coalescer: Option<JoinHandle<()>>,
}

impl RunningBridge {
  pub fn spawn(
    config: &BridgeConfig,
    computer: Option<PortEnds>,
    radio: Option<PortEnds>,
    logger: Logger,
    on_event: Arc<dyn Fn(BridgeEvent) + Send + Sync>,
  ) -> Self {
    let now = SystemTime::now();
    let shared = Arc::new(Shared {
      logger: logger.clone(),
      started: Instant::now(),
      packet_idle: config.packet_idle,
      computer_port: config.computer_port.clone(),
      radio_port: config.radio_port.clone(),
      bytes_computer_to_radio: AtomicU64::new(0),
      bytes_radio_to_computer: AtomicU64::new(0),
      write_errors: AtomicU64::new(0),
      computer_open: AtomicBool::new(false),
      radio_open: AtomicBool::new(false),
      seen_computer: FirstSeen::default(),
      seen_radio: FirstSeen::default(),
      shutdown: AtomicBool::new(false),
      serial_log: Mutex::new(SerialLog::create(config.log_file.clone(), now)),
      coalescer: Mutex::new(Coalescer {
        direction: None,
        bytes: Vec::new(),
        started_at: Instant::now(),
        description: String::new(),
        deadline: None,
        stopped: false,
      }),
      coalesce_wake: Condvar::new(),
    });

    logger.info(&format!(
      "Sniffer started computerPort={} radioPort={} baudRate={} rts={} dtr={} logFile={}",
      config.computer_port,
      config.radio_port,
      config.baud_rate,
      config.rts,
      config.dtr,
      config.log_file
    ));

    let mut readers = Vec::new();
    let (computer_reader, computer_writer) = split_ends(computer);
    let (radio_reader, radio_writer) = split_ends(radio);

    if let Some(reader) = computer_reader {
      shared.computer_open.store(true, Ordering::SeqCst);
      readers.push(spawn_reader(
        Arc::clone(&shared),
        Side::Computer,
        reader,
        radio_writer,
        Arc::clone(&on_event),
      ));
    }

    if let Some(reader) = radio_reader {
      shared.radio_open.store(true, Ordering::SeqCst);
      readers.push(spawn_reader(
        Arc::clone(&shared),
        Side::Radio,
        reader,
        computer_writer,
        Arc::clone(&on_event),
      ));
    }

    let coalescer_shared = Arc::clone(&shared);
    let coalesce_events = Arc::clone(&on_event);
    let coalescer = thread::Builder::new()
      .name("sniffer-coalesce".into())
      .spawn(move || coalesce_loop(coalescer_shared, coalesce_events))
      .expect("coalesce thread");

    Self {
      shared,
      readers,
      coalescer: Some(coalescer),
    }
  }

  pub fn stats(&self) -> BridgeStats {
    self.shared.stats()
  }

  pub fn log_path(&self) -> String {
    self
      .shared
      .serial_log
      .lock()
      .expect("log")
      .path()
      .to_string()
  }

  pub fn log_snapshot(&self) -> Option<serde_json::Value> {
    self.shared.serial_log.lock().expect("log").snapshot()
  }

  pub fn stop(&mut self) -> Option<serde_json::Value> {
    self.shared.shutdown.store(true, Ordering::SeqCst);

    for thread in self.readers.drain(..) {
      let _ = thread.join();
    }

    {
      let mut coalescer = self.shared.coalescer.lock().expect("coalescer");
      coalescer.stopped = true;
      self.shared.coalesce_wake.notify_all();
    }

    if let Some(thread) = self.coalescer.take() {
      let _ = thread.join();
    }

    self.shared.computer_open.store(false, Ordering::SeqCst);
    self.shared.radio_open.store(false, Ordering::SeqCst);

    let mut log = self.shared.serial_log.lock().expect("log");
    log.close();
    log.snapshot()
  }
}

impl Drop for RunningBridge {
  fn drop(&mut self) {
    if !self.readers.is_empty() || self.coalescer.is_some() {
      self.stop();
    }
  }
}

type SplitEnds = (Option<Box<dyn PortReader>>, Option<Box<dyn PortWriter>>);

fn split_ends(ends: Option<PortEnds>) -> SplitEnds {
  match ends {
    Some(ends) => (Some(ends.reader), Some(ends.writer)),
    None => (None, None),
  }
}

fn spawn_reader(
  shared: Arc<Shared>,
  side: Side,
  mut reader: Box<dyn PortReader>,
  writer: Option<Box<dyn PortWriter>>,
  on_event: Arc<dyn Fn(BridgeEvent) + Send + Sync>,
) -> JoinHandle<()> {
  thread::Builder::new()
    .name(format!("sniffer-{}", side.as_str()))
    .spawn(move || {
      let mut writer = writer;
      let mut buf = [0_u8; 4096];
      // Block in read. serialport's read polls the fd and reads on that
      // wake-up, so the kernel having a byte is what returns here. A timeout
      // only means the port was idle, which lets stop() finish.
      while !shared.shutdown.load(Ordering::SeqCst) {
        match reader.read(&mut buf) {
          Ok(0) => continue,
          Ok(count) => {
            let chunk = buf[..count].to_vec();
            handle_chunk(&shared, side, &chunk, writer.as_mut(), &on_event);
          }
          Err(error)
            if error.kind() == io::ErrorKind::TimedOut
              || error.kind() == io::ErrorKind::WouldBlock =>
          {
            continue
          }
          Err(error) => {
            if !shared.shutdown.load(Ordering::SeqCst) {
              shared
                .logger
                .error(&format!("{} port read failed: {error}", side.as_str()));
              on_event(BridgeEvent::Error {
                message: error.to_string(),
                source: side,
              });
            }
            break;
          }
        }
      }

      shared.set_open(side, false);
      shared.logger.warn(&format!(
        "{} port closed port={}",
        side.as_str(),
        shared.path(side)
      ));
      on_event(BridgeEvent::Stats);
    })
    .expect("reader thread")
}

fn handle_chunk(
  shared: &Shared,
  side: Side,
  chunk: &[u8],
  writer: Option<&mut Box<dyn PortWriter>>,
  on_event: &Arc<dyn Fn(BridgeEvent) + Send + Sync>,
) {
  let hex = format_hex(chunk);
  let already_seen = shared.seen(side).already_seen();
  if !already_seen {
    shared.logger.info(&format!(
      "First {}-port data bytes={} hex={hex}",
      side.as_str(),
      chunk.len()
    ));
  }
  shared.logger.debug(&format!(
    "{} data bytes={} hex={hex}",
    side.as_str(),
    chunk.len()
  ));

  shared.add_bytes(side, chunk.len() as u64);

  let peer = side.peer();
  let destination = shared.path(peer);
  let metadata = format!(
    "from={} to={} port={} bytes={} hex={}",
    side.as_str(),
    peer.as_str(),
    destination,
    chunk.len(),
    format_hex_preview(chunk)
  );

  match writer {
    Some(writer) if shared.is_open(peer) => {
      shared.logger.info(&format!("Bridge write {metadata}"));
      match writer.write_all(chunk) {
        Ok(()) => shared
          .logger
          .info(&format!("Bridge write finished {metadata}")),
        Err(error) => {
          shared.write_errors.fetch_add(1, Ordering::SeqCst);
          shared
            .logger
            .error(&format!("Bridge write failed {metadata} error={error}"));
          on_event(BridgeEvent::Error {
            message: error.to_string(),
            source: peer,
          });
        }
      }
    }
    _ => {
      shared.write_errors.fetch_add(1, Ordering::SeqCst);
      shared.logger.warn(&format!(
        "Dropping bridge write; {} port not open {metadata}",
        peer.as_str()
      ));
    }
  }

  {
    let mut log = shared.serial_log.lock().expect("log");
    log.push(side.is_send(), chunk, side.description(), SystemTime::now());
  }

  for packet in push_coalesced(shared, side, chunk) {
    on_event(BridgeEvent::Packet(packet));
    on_event(BridgeEvent::Stats);
  }
}

fn push_coalesced(shared: &Shared, side: Side, chunk: &[u8]) -> Vec<PacketEvent> {
  let mut coalescer = shared.coalescer.lock().expect("coalescer");
  let mut packets = Vec::new();

  if coalescer
    .direction
    .is_some_and(|direction| direction != side)
  {
    if let Some(packet) = coalescer.take(shared.started) {
      packets.push(packet);
    }
  }

  if coalescer.direction.is_none() {
    coalescer.direction = Some(side);
    coalescer.started_at = Instant::now();
    coalescer.description = side.description().to_string();
  }

  coalescer.bytes.extend_from_slice(chunk);

  if shared.packet_idle.is_zero() {
    if let Some(packet) = coalescer.take(shared.started) {
      packets.push(packet);
    }
    return packets;
  }

  coalescer.deadline = Some(Instant::now() + shared.packet_idle);
  shared.coalesce_wake.notify_all();
  packets
}

fn coalesce_loop(shared: Arc<Shared>, on_event: Arc<dyn Fn(BridgeEvent) + Send + Sync>) {
  loop {
    let packet = {
      let mut coalescer = shared.coalescer.lock().expect("coalescer");
      loop {
        if coalescer.stopped {
          let packet = coalescer.take(shared.started);
          drop(coalescer);
          if let Some(packet) = packet {
            on_event(BridgeEvent::Packet(packet));
            on_event(BridgeEvent::Stats);
          }
          return;
        }

        let now = Instant::now();
        if coalescer.deadline.is_some_and(|deadline| now >= deadline) {
          break coalescer.take(shared.started);
        }

        if let Some(deadline) = coalescer.deadline {
          let wait = deadline.saturating_duration_since(now);
          let (guard, _) = shared
            .coalesce_wake
            .wait_timeout(coalescer, wait)
            .expect("coalescer");
          coalescer = guard;
        } else {
          coalescer = shared.coalesce_wake.wait(coalescer).expect("coalescer");
        }
      }
    };

    if let Some(packet) = packet {
      on_event(BridgeEvent::Packet(packet));
      on_event(BridgeEvent::Stats);
    }
  }
}

impl Coalescer {
  fn take(&mut self, started: Instant) -> Option<PacketEvent> {
    self.deadline = None;
    let direction = self.direction.take()?;
    if self.bytes.is_empty() {
      return None;
    }

    let elapsed = self.started_at.saturating_duration_since(started);
    let elapsed_ms = elapsed.as_millis();
    let packet = PacketEvent {
      timestamp: format_elapsed(elapsed_ms),
      elapsed_ms,
      direction: direction.direction().to_string(),
      data: std::mem::take(&mut self.bytes),
      description: std::mem::take(&mut self.description),
    };
    Some(packet)
  }
}

impl Shared {
  fn stats(&self) -> BridgeStats {
    BridgeStats {
      bytes_computer_to_radio: self.bytes_computer_to_radio.load(Ordering::SeqCst),
      bytes_radio_to_computer: self.bytes_radio_to_computer.load(Ordering::SeqCst),
      write_errors: self.write_errors.load(Ordering::SeqCst),
      computer_port_open: self.computer_open.load(Ordering::SeqCst),
      radio_port_open: self.radio_open.load(Ordering::SeqCst),
    }
  }

  fn path(&self, side: Side) -> &str {
    match side {
      Side::Computer => &self.computer_port,
      Side::Radio => &self.radio_port,
    }
  }

  fn seen(&self, side: Side) -> &FirstSeen {
    match side {
      Side::Computer => &self.seen_computer,
      Side::Radio => &self.seen_radio,
    }
  }

  fn add_bytes(&self, side: Side, count: u64) {
    match side {
      Side::Computer => self
        .bytes_computer_to_radio
        .fetch_add(count, Ordering::SeqCst),
      Side::Radio => self
        .bytes_radio_to_computer
        .fetch_add(count, Ordering::SeqCst),
    };
  }

  fn is_open(&self, side: Side) -> bool {
    match side {
      Side::Computer => self.computer_open.load(Ordering::SeqCst),
      Side::Radio => self.radio_open.load(Ordering::SeqCst),
    }
  }

  fn set_open(&self, side: Side, open: bool) {
    match side {
      Side::Computer => self.computer_open.store(open, Ordering::SeqCst),
      Side::Radio => self.radio_open.store(open, Ordering::SeqCst),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::mpsc::{self, Receiver, Sender};

  struct ChanReader {
    rx: Receiver<Vec<u8>>,
    pending: std::collections::VecDeque<u8>,
  }

  impl PortReader for ChanReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
      while self.pending.is_empty() {
        // Wake when a chunk is sent. The timeout only fires while idle so
        // stop() can return; it is not what delivers the bytes.
        match self.rx.recv_timeout(Duration::from_millis(50)) {
          Ok(chunk) => self.pending.extend(chunk),
          Err(mpsc::RecvTimeoutError::Timeout) => {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "idle"))
          }
          Err(mpsc::RecvTimeoutError::Disconnected) => {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "closed"));
          }
        }
      }

      let count = buf.len().min(self.pending.len());
      for slot in buf.iter_mut().take(count) {
        *slot = self.pending.pop_front().expect("pending");
      }
      Ok(count)
    }
  }

  struct ChanWriter {
    tx: Sender<Vec<u8>>,
  }

  impl PortWriter for ChanWriter {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
      self
        .tx
        .send(buf.to_vec())
        .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }
  }

  struct Ends {
    read_tx: Sender<Vec<u8>>,
    write_rx: Receiver<Vec<u8>>,
    ends: Option<PortEnds>,
  }

  fn ends() -> Ends {
    let (read_tx, read_rx) = mpsc::channel();
    let (write_tx, write_rx) = mpsc::channel();
    Ends {
      read_tx,
      write_rx,
      ends: Some(PortEnds {
        reader: Box::new(ChanReader {
          rx: read_rx,
          pending: std::collections::VecDeque::new(),
        }),
        writer: Box::new(ChanWriter { tx: write_tx }),
      }),
    }
  }

  fn config(idle: Duration, log_file: &str) -> BridgeConfig {
    BridgeConfig {
      computer_port: "/dev/computer".into(),
      radio_port: "/dev/radio".into(),
      baud_rate: 9600,
      log_file: log_file.into(),
      rts: true,
      dtr: true,
      packet_idle: idle,
    }
  }

  #[test]
  fn forwards_a_computer_chunk_as_soon_as_read_returns() {
    let mut computer = ends();
    let mut radio = ends();
    let packets = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&packets);
    let mut bridge = RunningBridge::spawn(
      &config(Duration::ZERO, "sniffer-forward-test.json"),
      computer.ends.take(),
      radio.ends.take(),
      Logger::quiet(),
      Arc::new(move |event| {
        if let BridgeEvent::Packet(packet) = event {
          collected.lock().expect("packets").push(packet);
        }
      }),
    );

    let started = Instant::now();
    computer.read_tx.send(vec![0x50, 0xbb]).unwrap();
    let written = radio
      .write_rx
      .recv_timeout(Duration::from_millis(200))
      .unwrap();
    assert!(started.elapsed() < Duration::from_millis(200));
    assert_eq!(written, vec![0x50, 0xbb]);

    let deadline = Instant::now() + Duration::from_millis(200);
    while packets.lock().expect("packets").is_empty() && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(5));
    }

    let recorded = packets.lock().expect("packets").clone();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].direction, "COMPUTER->RADIO");
    assert_eq!(recorded[0].data, vec![0x50, 0xbb]);
    let stats = bridge.stats();
    assert_eq!(stats.bytes_computer_to_radio, 2);
    assert_eq!(stats.write_errors, 0);
    assert!(stats.computer_port_open);
    assert!(stats.radio_port_open);
    bridge.stop();
    let _ = std::fs::remove_file("sniffer-forward-test.json");
  }

  #[test]
  fn coalesces_one_direction_after_idle_and_splits_on_direction_change() {
    let mut computer = ends();
    let mut radio = ends();
    let packets = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&packets);
    let mut bridge = RunningBridge::spawn(
      &config(Duration::from_millis(15), "sniffer-coalesce-test.json"),
      computer.ends.take(),
      radio.ends.take(),
      Logger::quiet(),
      Arc::new(move |event| {
        if let BridgeEvent::Packet(packet) = event {
          collected.lock().expect("packets").push(packet);
        }
      }),
    );

    computer.read_tx.send(vec![0x50]).unwrap();
    computer.read_tx.send(vec![0xbb]).unwrap();
    let early = Instant::now() + Duration::from_millis(5);
    while Instant::now() < early {
      thread::sleep(Duration::from_millis(1));
    }
    assert!(packets.lock().expect("packets").is_empty());

    radio.read_tx.send(vec![0x06]).unwrap();
    let deadline = Instant::now() + Duration::from_millis(200);
    while packets.lock().expect("packets").len() < 2 && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(5));
    }

    let recorded = packets.lock().expect("packets").clone();
    assert_eq!(recorded[0].direction, "COMPUTER->RADIO");
    assert_eq!(recorded[0].data, vec![0x50, 0xbb]);
    assert_eq!(recorded[1].direction, "RADIO->COMPUTER");
    assert_eq!(recorded[1].data, vec![0x06]);
    bridge.stop();
    let _ = std::fs::remove_file("sniffer-coalesce-test.json");
  }

  #[test]
  fn counts_bytes_when_the_other_port_is_closed() {
    let mut computer = ends();
    let packets = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&packets);
    let mut bridge = RunningBridge::spawn(
      &config(Duration::ZERO, "sniffer-closed-test.json"),
      computer.ends.take(),
      None,
      Logger::quiet(),
      Arc::new(move |event| {
        if let BridgeEvent::Packet(packet) = event {
          collected.lock().expect("packets").push(packet);
        }
      }),
    );

    computer.read_tx.send(vec![0x0d]).unwrap();
    let deadline = Instant::now() + Duration::from_millis(200);
    while packets.lock().expect("packets").is_empty() && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(packets.lock().expect("packets")[0].data, vec![0x0d]);
    let stats = bridge.stats();
    assert_eq!(stats.bytes_computer_to_radio, 1);
    assert_eq!(stats.write_errors, 1);
    assert!(!stats.radio_port_open);
    bridge.stop();
    let _ = std::fs::remove_file("sniffer-closed-test.json");
  }
}
