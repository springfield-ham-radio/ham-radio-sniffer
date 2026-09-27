use std::fs;
use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

/// On-disk `SEND` / `RECV` log in the ham-radio-driver SerialLogger shape.
///
/// Groups are split on direction change. The file is written when the bridge
/// stops. A snapshot includes the open group so a capture saved mid-session
/// still has the latest bytes, without starting a new group.
#[derive(Debug, Clone)]
pub struct SerialLog {
  path: String,
  start: SystemTime,
  start_iso: String,
  end_iso: Option<String>,
  entries: Vec<LogEntry>,
  pending_direction: Option<Direction>,
  pending_bytes: Vec<u8>,
  pending_elapsed_ms: u128,
  pending_description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
  Send,
  Recv,
}

impl Direction {
  fn as_str(self) -> &'static str {
    match self {
      Self::Send => "SEND",
      Self::Recv => "RECV",
    }
  }
}

#[derive(Debug, Clone)]
struct LogEntry {
  timestamp: String,
  elapsed_ms: u128,
  direction: Direction,
  data: Vec<u8>,
  description: Option<String>,
}

impl SerialLog {
  pub fn create(path: String, start: SystemTime) -> Self {
    Self {
      path,
      start_iso: format_iso(start),
      start,
      end_iso: None,
      entries: Vec::new(),
      pending_direction: None,
      pending_bytes: Vec::new(),
      pending_elapsed_ms: 0,
      pending_description: None,
    }
  }

  pub fn path(&self) -> &str {
    &self.path
  }

  pub fn push(&mut self, send: bool, bytes: &[u8], description: &str, at: SystemTime) {
    let direction = if send {
      Direction::Send
    } else {
      Direction::Recv
    };

    if self
      .pending_direction
      .is_some_and(|current| current != direction)
    {
      self.flush_pending();
    }

    if self.pending_direction.is_none() {
      self.pending_direction = Some(direction);
      self.pending_elapsed_ms = elapsed_ms(self.start, at);
      self.pending_description = Some(description.to_string());
    }

    self.pending_bytes.extend_from_slice(bytes);
  }

  /// JSON object, or `None` when nothing has been logged.
  pub fn snapshot(&self) -> Option<Value> {
    let mut entries = self.entries.clone();

    if let Some(direction) = self.pending_direction {
      if !self.pending_bytes.is_empty() {
        entries.push(LogEntry {
          timestamp: format_elapsed(self.pending_elapsed_ms),
          elapsed_ms: self.pending_elapsed_ms,
          direction,
          data: self.pending_bytes.clone(),
          description: self.pending_description.clone(),
        });
      }
    }

    if entries.is_empty() {
      return None;
    }

    Some(self.to_json(&entries))
  }

  pub fn close(&mut self) {
    self.flush_pending();
    self.end_iso = Some(format_iso(SystemTime::now()));
    self.write_file();
  }

  fn flush_pending(&mut self) {
    let Some(direction) = self.pending_direction.take() else {
      return;
    };

    if self.pending_bytes.is_empty() {
      self.pending_description = None;
      return;
    }

    self.entries.push(LogEntry {
      timestamp: format_elapsed(self.pending_elapsed_ms),
      elapsed_ms: self.pending_elapsed_ms,
      direction,
      data: std::mem::take(&mut self.pending_bytes),
      description: self.pending_description.take(),
    });
  }

  fn to_json(&self, entries: &[LogEntry]) -> Value {
    let mut metadata = json!({
      "startTime": self.start_iso,
      "totalEntries": entries.len(),
      "version": "1.0.0",
    });

    if let Some(end) = &self.end_iso {
      metadata["endTime"] = json!(end);
    }

    json!({
      "metadata": metadata,
      "entries": entries.iter().map(LogEntry::to_json).collect::<Vec<_>>(),
    })
  }

  fn write_file(&self) {
    let payload = self.to_json(&self.entries);
    if let Err(error) = fs::write(
      &self.path,
      serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into()),
    ) {
      eprintln!("ERROR Failed to write log file {}: {error}", self.path);
    }
  }
}

impl LogEntry {
  fn to_json(&self) -> Value {
    let mut entry = json!({
      "timestamp": self.timestamp,
      "elapsedMs": self.elapsed_ms,
      "direction": self.direction.as_str(),
      "data": self.data,
    });

    if let Some(description) = &self.description {
      entry["description"] = json!(description);
    }

    entry
  }
}

pub fn default_log_path(now: SystemTime) -> String {
  let stamp = format_iso(now).replace([':', '.'], "-");
  format!("radio-sniffer-{stamp}.json")
}

pub fn format_elapsed(elapsed_ms: u128) -> String {
  let seconds = elapsed_ms / 1000;
  let millis = elapsed_ms % 1000;
  format!("{seconds:03}.{millis:03}")
}

pub fn format_iso(time: SystemTime) -> String {
  DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn elapsed_ms(start: SystemTime, at: SystemTime) -> u128 {
  at.duration_since(start)
    .map(|duration| duration.as_millis())
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;

  #[test]
  fn groups_send_and_recv_on_direction_change() {
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let mut log = SerialLog::create("capture.json".into(), start);
    log.push(true, &[0x50], "Computer to Radio", start);
    log.push(
      true,
      &[0xbb],
      "Computer to Radio",
      start + Duration::from_millis(5),
    );
    log.push(
      false,
      &[0x06],
      "Radio to Computer",
      start + Duration::from_millis(40),
    );

    let snapshot = log.snapshot().unwrap();
    let entries = snapshot["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["direction"], "SEND");
    assert_eq!(entries[0]["data"], serde_json::json!([0x50, 0xbb]));
    assert_eq!(entries[1]["direction"], "RECV");
    assert_eq!(entries[1]["data"], serde_json::json!([0x06]));
    assert_eq!(snapshot["metadata"]["version"], "1.0.0");
    assert_eq!(snapshot["metadata"]["totalEntries"], 2);
    assert!(snapshot["metadata"].get("endTime").is_none());
  }

  #[test]
  fn empty_log_has_no_snapshot() {
    let log = SerialLog::create("capture.json".into(), SystemTime::UNIX_EPOCH);
    assert!(log.snapshot().is_none());
  }
}
