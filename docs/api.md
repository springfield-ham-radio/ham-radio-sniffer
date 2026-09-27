# Sniffer HTTP API

The sniffer is one Rust binary. ham-radio-ui (or any HTTP client) starts, stops, and observes a serial bridge.

Default URL: `http://127.0.0.1:3010`

`HOST` and `PORT` override the listen address. Loopback (`127.0.0.1`, `localhost`, `::1`) binds `127.0.0.1`. Any other host binds `0.0.0.0`. CORS is enabled so a separately hosted UI can call the API.

## Endpoints

### `GET /` and `GET /api/health`

Liveness check. `version` is the crate version that binary was built from.

```json
{ "ok": true, "service": "ham-radio-sniffer", "version": "0.1.0" }
```

### `GET /api/ports`

Lists serial ports available on the machine running the sniffer.

```json
{
  "ports": [
    {
      "path": "/dev/tty.usbserial-A",
      "manufacturer": "FTDI",
      "vendorId": "0403",
      "productId": "6001"
    }
  ]
}
```

### `GET /api/sniffer`

Current session status. `running` is `false` until `POST /api/sniffer/start` succeeds. While a bridge is running, status also includes live diagnostics:

- `computerPortOpen` / `radioPortOpen`
- `bytesComputerToRadio` / `bytesRadioToComputer` (bytes the UART delivered, even if the other port is closed)
- `writeErrors` (dropped or failed forwards)
- `rts` / `dtr` (the lines applied after open; both default to true)

Zero bytes with both ports open means the process never received data on those devices.

### Logging

Set `SNIFFER_LOG_LEVEL` or `LOG_LEVEL` to `debug`, `info` (default), `warn`, or `error`.

```bash
SNIFFER_LOG_LEVEL=debug ./ham-radio-sniffer
```

`info` logs port open and close, the first bytes on each port, and each bridge write (including when it finishes or is dropped). `debug` logs every chunk as hex.

### `POST /api/sniffer/start`

Starts a single bridge between two ports. Returns `409` if a session is already running, or `400` if the body is invalid. Error bodies include `statusMessage`.

```json
{
  "computerPort": "/dev/tty.usbserial-A",
  "radioPort": "/dev/tty.usbserial-B",
  "baudRate": 9600,
  "logFile": "optional-capture.json",
  "rts": true,
  "dtr": true
}
```

`computerPort` is the debug-cable side (computer ↔ sniffer); `radioPort` is the programming-cable side (sniffer ↔ radio). They must be different paths. `baudRate` defaults to 9600. Ports open at 8N1 with RTS/CTS off. RTS and DTR are asserted after open unless the request sets them false.

Bytes are counted as soon as the UART delivers them and forwarded to the other port immediately. UI packets coalesce on direction change or after 15 ms idle. The on-disk `SEND` / `RECV` log groups on direction change and is written when the bridge stops.

### `POST /api/sniffer/stop`

Stops the running bridge and closes both serial ports. Safe to call when nothing is running. Packets and the serial log remain readable afterward.

### `GET /api/sniffer/log`

Returns status, coalesced packets captured in this session, and the serial log.

The `file.data` payload uses the same SerialLogger JSON shape as ham-radio-driver (`metadata` plus `entries` of `SEND` / `RECV`). ham-radio-ui wraps this into a `springfield-ham-radio-sniffer-capture` document when you click **Save capture**.

Log data remains available after `POST /api/sniffer/stop` so captures can be saved once traffic finishes. `file.data` is omitted when nothing has been logged.

### `GET /api/sniffer/events`

Server-sent events stream. Each `message` is JSON:

- `{ "type": "status", "status": { ... } }`
- `{ "type": "packet", "packet": { "id", "timestamp", "elapsedMs", "direction", "data" } }`
- `{ "type": "error", "message": "...", "source": "computer" | "radio" }`

`data` is an array of byte values `0-255`. Direction is `COMPUTER->RADIO` or `RADIO->COMPUTER`. A new subscriber receives the current status first.

## CLI

```bash
ham-radio-sniffer --list-ports
ham-radio-sniffer <computer-port> <radio-port> [baud-rate] [--log-file <filename>] [--no-rts] [--no-dtr]
ham-radio-sniffer --version
```

With no arguments the process serves the HTTP API.

## ham-radio-ui

The Sniffer screen talks to this process at `http://127.0.0.1:3010` by default. Change the host and port under Preferences → Sniffer.
