use std::sync::Arc;
use std::time::Duration;

use crate::logging::{level_from_env, Logger};
use crate::ports::{list_ports, SerialOpener};
use crate::session::Session;
use crate::CRATE_VERSION;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
  Version,
  ListPorts,
  Bridge(BridgeArgs),
  Usage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeArgs {
  pub computer_port: String,
  pub radio_port: String,
  pub baud_rate: u32,
  pub log_file: Option<String>,
  pub rts: bool,
  pub dtr: bool,
}

pub fn parse_args(args: &[String]) -> Result<CliCommand, String> {
  if args.iter().any(|arg| arg == "--version" || arg == "-v") {
    return Ok(CliCommand::Version);
  }

  if args.iter().any(|arg| arg == "--list-ports") {
    return Ok(CliCommand::ListPorts);
  }

  if args.len() < 2 {
    return Ok(CliCommand::Usage);
  }

  let computer_port = args[0].clone();
  let radio_port = args[1].clone();

  if computer_port.starts_with('-') || radio_port.starts_with('-') {
    return Ok(CliCommand::Usage);
  }

  let mut baud_rate = 9600_u32;
  let mut log_file = None;
  let mut rts = true;
  let mut dtr = true;
  let mut index = 2;

  while index < args.len() {
    match args[index].as_str() {
      "--log-file" => {
        let Some(path) = args.get(index + 1) else {
          return Err("--log-file requires a filename".into());
        };
        log_file = Some(path.clone());
        index += 2;
      }
      "--no-rts" => {
        rts = false;
        index += 1;
      }
      "--no-dtr" => {
        dtr = false;
        index += 1;
      }
      other => {
        if let Ok(baud) = other.parse::<u32>() {
          if baud == 0 {
            return Err("baud rate must be a positive integer".into());
          }
          baud_rate = baud;
          index += 1;
        } else {
          return Err(format!("Unknown argument {other}"));
        }
      }
    }
  }

  Ok(CliCommand::Bridge(BridgeArgs {
    computer_port,
    radio_port,
    baud_rate,
    log_file,
    rts,
    dtr,
  }))
}

pub fn print_usage() {
  println!("Usage: ham-radio-sniffer <computer-port> <radio-port> [baud-rate] [--log-file <filename>] [--no-rts] [--no-dtr]");
  println!("       ham-radio-sniffer --list-ports");
  println!("       ham-radio-sniffer --version");
  println!();
  println!("With no arguments, the HTTP API listens on 127.0.0.1:3010.");
  println!("HOST and PORT override that. Loopback binds 127.0.0.1; any other host binds 0.0.0.0.");
  println!();
  println!("RTS and DTR default on (typical USB clone cable). Use --no-rts for TH-F6 CAT.");
  println!();
  println!("Examples:");
  println!("  ham-radio-sniffer /dev/ttyS0 /dev/ttyUSB0");
  println!("  ham-radio-sniffer /dev/ttyS0 /dev/ttyUSB0 9600");
  println!("  ham-radio-sniffer /dev/ttyS0 /dev/ttyUSB0 9600 --log-file my-sniffer.json");
  println!("  ham-radio-sniffer /dev/ttyS0 /dev/ttyUSB0 9600 --no-rts");
}

/// Run a CLI command. Returns the process exit code.
pub fn run_cli(args: &[String]) -> i32 {
  match parse_args(args) {
    Ok(CliCommand::Version) => {
      println!("{CRATE_VERSION}");
      0
    }
    Ok(CliCommand::ListPorts) => match list_ports() {
      Ok(ports) => {
        for port in ports {
          println!("{}", port.path);
        }
        0
      }
      Err(error) => {
        eprintln!("Sniffer error: {error}");
        1
      }
    },
    Ok(CliCommand::Usage) => {
      print_usage();
      1
    }
    Ok(CliCommand::Bridge(bridge)) => run_bridge(bridge),
    Err(error) => {
      eprintln!("Sniffer error: {error}");
      print_usage();
      1
    }
  }
}

fn run_bridge(args: BridgeArgs) -> i32 {
  let logger = Logger::new(level_from_env());
  let session = Session::new(Arc::new(SerialOpener), logger.clone());
  let request = crate::parse::StartRequest {
    computer_port: args.computer_port,
    radio_port: args.radio_port,
    baud_rate: Some(args.baud_rate),
    log_file: args.log_file,
    rts: Some(args.rts),
    dtr: Some(args.dtr),
  };

  if let Err(error) = session.start(request) {
    eprintln!("Sniffer error: {}", error.message());
    return 1;
  }

  logger.info("Waiting for data transfer - press Ctrl+C to stop");
  let (tx, rx) = std::sync::mpsc::channel();
  if let Err(error) = ctrlc::set_handler(move || {
    let _ = tx.send(());
  }) {
    eprintln!("Sniffer error: {error}");
    session.stop();
    return 1;
  }

  let _ = rx.recv();
  session.stop();
  // Give the reader threads one poll interval to notice shutdown.
  std::thread::sleep(Duration::from_millis(50));
  0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_bridge_flags() {
    let args = vec![
      "/dev/ttyS0".into(),
      "/dev/ttyUSB0".into(),
      "19200".into(),
      "--log-file".into(),
      "cap.json".into(),
      "--no-rts".into(),
      "--no-dtr".into(),
    ];
    assert_eq!(
      parse_args(&args).unwrap(),
      CliCommand::Bridge(BridgeArgs {
        computer_port: "/dev/ttyS0".into(),
        radio_port: "/dev/ttyUSB0".into(),
        baud_rate: 19200,
        log_file: Some("cap.json".into()),
        rts: false,
        dtr: false,
      })
    );
  }

  #[test]
  fn version_list_and_usage() {
    assert_eq!(
      parse_args(&["--version".into()]).unwrap(),
      CliCommand::Version
    );
    assert_eq!(
      parse_args(&["--list-ports".into()]).unwrap(),
      CliCommand::ListPorts
    );
    assert_eq!(parse_args(&[]).unwrap(), CliCommand::Usage);
    assert_eq!(parse_args(&["--no-rts".into()]).unwrap(), CliCommand::Usage);
  }
}
