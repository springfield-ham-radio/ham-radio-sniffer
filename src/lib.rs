//! Headless serial bridge. The HTTP API stays compatible with the HamBench sniffer client.

mod bind;
mod bridge;
mod cli;
mod http;
mod logging;
mod parse;
mod ports;
mod serial_log;
mod session;

pub use bind::listen_addr;
pub use cli::{run_cli, CliCommand};
pub use http::{serve, serve_from_env};
pub use logging::{resolve_log_level, Level, Logger};
pub use parse::{parse_start_request, StartRequest, StartRequestError};
pub use ports::{list_ports, SerialOpener};
pub use session::{Session, SessionError};

pub const SERVICE_NAME: &str = "ham-radio-sniffer";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `{ "ok": true, "service": "ham-radio-sniffer", "version": "<crate version>" }`.
pub fn health_body() -> serde_json::Value {
  serde_json::json!({
    "ok": true,
    "service": SERVICE_NAME,
    "version": CRATE_VERSION,
  })
}
