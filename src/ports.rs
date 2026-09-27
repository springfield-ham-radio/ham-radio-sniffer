use std::io::{self, Read, Write};
use std::time::Duration;

use serde::Serialize;
use serialport::{DataBits, FlowControl, Parity, SerialPort, SerialPortType, StopBits};

use crate::bridge::{PortEnds, PortReader, PortWriter};

/// How long `read` may block while idle so `stop` can be noticed.
///
/// `serialport` waits in `poll` and calls `read` on that wake-up. A byte the
/// kernel already has is returned then. This timeout is only the idle bound;
/// it is not what delivers the next byte.
pub const READ_POLL_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
  pub path: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub manufacturer: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub serial_number: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub product_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub vendor_id: Option<String>,
}

pub fn list_ports() -> Result<Vec<PortInfo>, String> {
  let ports = serialport::available_ports().map_err(|error| error.to_string())?;
  Ok(ports.into_iter().map(map_port).collect())
}

fn map_port(port: serialport::SerialPortInfo) -> PortInfo {
  let mut info = PortInfo {
    path: port.port_name,
    manufacturer: None,
    serial_number: None,
    product_id: None,
    vendor_id: None,
  };

  if let SerialPortType::UsbPort(usb) = port.port_type {
    info.manufacturer = usb.manufacturer.filter(|value| !value.is_empty());
    info.serial_number = usb.serial_number.filter(|value| !value.is_empty());
    info.product_id = Some(format!("{:04x}", usb.pid));
    info.vendor_id = Some(format!("{:04x}", usb.vid));
  }

  info
}

pub struct SerialOpener;

impl crate::bridge::PortOpener for SerialOpener {
  fn open(&self, path: &str, baud_rate: u32, rts: bool, dtr: bool) -> Result<PortEnds, String> {
    let mut port = serialport::new(path, baud_rate)
      .data_bits(DataBits::Eight)
      .parity(Parity::None)
      .stop_bits(StopBits::One)
      .flow_control(FlowControl::None)
      .timeout(READ_POLL_TIMEOUT)
      .open()
      .map_err(|error| error.to_string())?;

    port
      .write_request_to_send(rts)
      .map_err(|error| format!("RTS failed: {error}"))?;
    port
      .write_data_terminal_ready(dtr)
      .map_err(|error| format!("DTR failed: {error}"))?;

    let mut writer = port.try_clone().map_err(|error| error.to_string())?;
    writer
      .set_timeout(Duration::from_secs(2))
      .map_err(|error| error.to_string())?;

    Ok(PortEnds {
      reader: Box::new(SerialReader { port }),
      writer: Box::new(SerialWriter { port: writer }),
    })
  }
}

struct SerialReader {
  port: Box<dyn SerialPort>,
}

impl PortReader for SerialReader {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    self.port.read(buf)
  }
}

struct SerialWriter {
  port: Box<dyn SerialPort>,
}

impl PortWriter for SerialWriter {
  fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
    self.port.write_all(buf)?;
    self.port.flush()
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn formats_usb_ids_as_four_hex_digits() {
    assert_eq!(format!("{:04x}", 0x0403u16), "0403");
    assert_eq!(format!("{:04x}", 0x6001u16), "6001");
  }
}
