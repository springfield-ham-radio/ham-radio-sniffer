use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
  Debug = 0,
  Info = 1,
  Warn = 2,
  Error = 3,
}

/// `SNIFFER_LOG_LEVEL` or `LOG_LEVEL`: `debug`, `info` (default), `warn`, or `error`.
pub fn resolve_log_level(value: Option<&str>) -> Level {
  match value
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(|value| value.to_ascii_lowercase())
  {
    Some(ref level) if level == "debug" || level == "trace" => Level::Debug,
    Some(ref level) if level == "warn" || level == "warning" => Level::Warn,
    Some(ref level) if level == "error" || level == "fatal" => Level::Error,
    Some(ref level) if level == "info" => Level::Info,
    _ => Level::Info,
  }
}

pub fn level_from_env() -> Level {
  let value = std::env::var("SNIFFER_LOG_LEVEL")
    .ok()
    .or_else(|| std::env::var("LOG_LEVEL").ok());
  resolve_log_level(value.as_deref())
}

#[derive(Clone, Debug)]
pub struct Logger {
  level: Level,
  quiet: bool,
}

impl Logger {
  pub fn new(level: Level) -> Self {
    Self {
      level,
      quiet: false,
    }
  }

  pub fn from_env() -> Self {
    Self::new(level_from_env())
  }

  pub fn quiet() -> Self {
    Self {
      level: Level::Error,
      quiet: true,
    }
  }

  pub fn log(&self, level: Level, message: &str) {
    if self.quiet || level < self.level {
      return;
    }

    let label = match level {
      Level::Debug => "DEBUG",
      Level::Info => "INFO",
      Level::Warn => "WARN",
      Level::Error => "ERROR",
    };
    eprintln!("{label} {message}");
  }

  pub fn debug(&self, message: &str) {
    self.log(Level::Debug, message);
  }

  pub fn info(&self, message: &str) {
    self.log(Level::Info, message);
  }

  pub fn warn(&self, message: &str) {
    self.log(Level::Warn, message);
  }

  pub fn error(&self, message: &str) {
    self.log(Level::Error, message);
  }

  pub fn level_name(&self) -> &'static str {
    match self.level {
      Level::Debug => "debug",
      Level::Info => "info",
      Level::Warn => "warn",
      Level::Error => "error",
    }
  }
}

/// Remembers whether info already logged the first chunk on a port.
#[derive(Debug, Default)]
pub struct FirstSeen {
  seen: AtomicBool,
}

impl FirstSeen {
  pub fn already_seen(&self) -> bool {
    self.seen.swap(true, Ordering::Relaxed)
  }
}

pub fn format_hex(bytes: &[u8]) -> String {
  bytes
    .iter()
    .map(|byte| format!("{byte:02X}"))
    .collect::<Vec<_>>()
    .join(" ")
}

pub fn format_hex_preview(bytes: &[u8]) -> String {
  let preview = format_hex(&bytes[..bytes.len().min(32)]);
  if bytes.len() <= 32 {
    preview
  } else {
    format!("{preview} …")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn log_level_defaults_and_aliases() {
    assert_eq!(resolve_log_level(None), Level::Info);
    assert_eq!(resolve_log_level(Some("")), Level::Info);
    assert_eq!(resolve_log_level(Some("nope")), Level::Info);
    assert_eq!(resolve_log_level(Some("debug")), Level::Debug);
    assert_eq!(resolve_log_level(Some("TRACE")), Level::Debug);
    assert_eq!(resolve_log_level(Some("warn")), Level::Warn);
    assert_eq!(resolve_log_level(Some("warning")), Level::Warn);
    assert_eq!(resolve_log_level(Some("error")), Level::Error);
    assert_eq!(resolve_log_level(Some("fatal")), Level::Error);
    assert_eq!(resolve_log_level(Some("info")), Level::Info);
  }

  #[test]
  fn hex_preview_truncates_after_32_bytes() {
    let bytes: Vec<u8> = (0..40).collect();
    let preview = format_hex_preview(&bytes);
    assert!(preview.ends_with(" …"));
    assert_eq!(preview.split_whitespace().count(), 33);
  }
}
