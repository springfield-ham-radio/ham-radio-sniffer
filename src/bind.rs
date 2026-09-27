use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Listen address for `HOST` and `PORT`.
///
/// Loopback (`127.0.0.1`, `localhost`, `::1`) stays on `127.0.0.1`. Any other
/// host binds `0.0.0.0`, matching the desktop start script.
pub fn listen_addr(host: &str, port: u16) -> SocketAddr {
  let ip = if is_loopback_host(host) {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
  } else {
    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
  };
  SocketAddr::new(ip, port)
}

pub fn is_loopback_host(host: &str) -> bool {
  let host = host.trim();
  host.is_empty()
    || host == "127.0.0.1"
    || host.eq_ignore_ascii_case("localhost")
    || host == "::1"
    || host == "[::1]"
    || host == "0:0:0:0:0:0:0:1"
}

pub fn host_from_env() -> String {
  std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub fn port_from_env() -> Result<u16, String> {
  match std::env::var("PORT") {
    Err(_) => Ok(3010),
    Ok(value) if value.trim().is_empty() => Ok(3010),
    Ok(value) => value
      .trim()
      .parse::<u16>()
      .map_err(|_| format!("PORT must be an integer from 1 to 65535, got {value}"))
      .and_then(|port| {
        if port == 0 {
          Err("PORT must be an integer from 1 to 65535, got 0".to_string())
        } else {
          Ok(port)
        }
      }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn loopback_hosts_bind_localhost() {
    for host in ["", "127.0.0.1", "localhost", "LOCALHOST", "::1", "[::1]"] {
      assert_eq!(
        listen_addr(host, 3010).ip(),
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        "{host}"
      );
    }
  }

  #[test]
  fn other_hosts_bind_all_interfaces() {
    for host in ["0.0.0.0", "192.168.1.10", "pi.local"] {
      assert_eq!(
        listen_addr(host, 3010).ip(),
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        "{host}"
      );
    }
  }
}
