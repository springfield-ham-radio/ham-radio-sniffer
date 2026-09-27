use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequest {
  pub computer_port: String,
  pub radio_port: String,
  pub baud_rate: Option<u32>,
  pub log_file: Option<String>,
  pub rts: Option<bool>,
  pub dtr: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequestError {
  pub message: String,
}

impl StartRequestError {
  fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

impl std::fmt::Display for StartRequestError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.message)
  }
}

/// Validates a JSON body for `POST /api/sniffer/start`.
///
/// Ports are required and must be distinct so the sniffer does not open the
/// same device twice.
pub fn parse_start_request(body: &Value) -> Result<StartRequest, StartRequestError> {
  let record = body
    .as_object()
    .ok_or_else(|| StartRequestError::new("Request body must be a JSON object"))?;
  let computer_port = read_required_string(record.get("computerPort"), "computerPort")?;
  let radio_port = read_required_string(record.get("radioPort"), "radioPort")?;

  if computer_port == radio_port {
    return Err(StartRequestError::new(
      "computerPort and radioPort must be different",
    ));
  }

  Ok(StartRequest {
    computer_port,
    radio_port,
    baud_rate: read_optional_baud(record.get("baudRate"))?,
    log_file: read_optional_string(record.get("logFile"), "logFile")?,
    rts: read_optional_bool(record.get("rts"), "rts")?,
    dtr: read_optional_bool(record.get("dtr"), "dtr")?,
  })
}

fn read_optional_string(
  value: Option<&Value>,
  field_name: &str,
) -> Result<Option<String>, StartRequestError> {
  match value {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(text)) => {
      let trimmed = text.trim();
      if trimmed.is_empty() {
        Ok(None)
      } else {
        Ok(Some(trimmed.to_string()))
      }
    }
    Some(_) => Err(StartRequestError::new(format!(
      "{field_name} must be a string"
    ))),
  }
}

fn read_required_string(
  value: Option<&Value>,
  field_name: &str,
) -> Result<String, StartRequestError> {
  read_optional_string(value, field_name)?
    .ok_or_else(|| StartRequestError::new(format!("{field_name} is required")))
}

fn read_optional_baud(value: Option<&Value>) -> Result<Option<u32>, StartRequestError> {
  match value {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(text)) if text.trim().is_empty() => Ok(None),
    Some(Value::Number(number)) => {
      let baud = number
        .as_u64()
        .filter(|baud| *baud > 0 && *baud <= u64::from(u32::MAX) && number.is_u64());
      match baud {
        Some(baud) => Ok(Some(baud as u32)),
        None => Err(StartRequestError::new(
          "baudRate must be a positive integer",
        )),
      }
    }
    Some(_) => Err(StartRequestError::new(
      "baudRate must be a positive integer",
    )),
  }
}

fn read_optional_bool(
  value: Option<&Value>,
  field_name: &str,
) -> Result<Option<bool>, StartRequestError> {
  match value {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(text)) if text.trim().is_empty() => Ok(None),
    Some(Value::Bool(flag)) => Ok(Some(*flag)),
    Some(_) => Err(StartRequestError::new(format!(
      "{field_name} must be a boolean"
    ))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn accepts_a_valid_start_request() {
    let request = parse_start_request(&json!({
      "computerPort": "/dev/tty.usbserial-A",
      "radioPort": "/dev/tty.usbserial-B",
      "baudRate": 9600,
      "logFile": "capture.json",
    }))
    .unwrap();

    assert_eq!(
      request,
      StartRequest {
        computer_port: "/dev/tty.usbserial-A".into(),
        radio_port: "/dev/tty.usbserial-B".into(),
        baud_rate: Some(9600),
        log_file: Some("capture.json".into()),
        rts: None,
        dtr: None,
      }
    );
  }

  #[test]
  fn rejects_a_missing_computer_port() {
    let error = parse_start_request(&json!({ "radioPort": "/dev/ttyUSB0" })).unwrap_err();
    assert_eq!(error.message, "computerPort is required");
  }

  #[test]
  fn rejects_the_same_path_for_both_ports() {
    let error = parse_start_request(&json!({
      "computerPort": "/dev/ttyUSB0",
      "radioPort": "/dev/ttyUSB0",
    }))
    .unwrap_err();
    assert_eq!(
      error.message,
      "computerPort and radioPort must be different"
    );
  }

  #[test]
  fn rejects_a_non_positive_baud_rate() {
    let error = parse_start_request(&json!({
      "computerPort": "/dev/ttyUSB0",
      "radioPort": "/dev/ttyUSB1",
      "baudRate": 0,
    }))
    .unwrap_err();
    assert_eq!(error.message, "baudRate must be a positive integer");
  }

  #[test]
  fn rejects_a_non_object_body() {
    let error = parse_start_request(&Value::Null).unwrap_err();
    assert_eq!(error.message, "Request body must be a JSON object");
  }

  #[test]
  fn rejects_non_boolean_control_lines() {
    let error = parse_start_request(&json!({
      "computerPort": "/dev/ttyUSB0",
      "radioPort": "/dev/ttyUSB1",
      "rts": "yes",
    }))
    .unwrap_err();
    assert_eq!(error.message, "rts must be a boolean");
  }
}
