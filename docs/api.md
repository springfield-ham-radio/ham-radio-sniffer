# Sniffer HTTP API

The sniffer is a headless Nuxt/Nitro server. ham-radio-ui (or any HTTP client) starts, stops, and observes a serial bridge.

Default development URL: `http://127.0.0.1:3010`

CORS is enabled so a separately hosted UI can call the API.

## Endpoints

### `GET /` and `GET /api/health`

Liveness check.

```json
{ "ok": true, "service": "ham-radio-sniffer" }
```

### `GET /api/ports`

Lists serial ports available on the machine running the sniffer.

```json
{
  "ports": [
    {
      "path": "/dev/tty.usbserial-A",
      "manufacturer": "FTDI"
    }
  ]
}
```

### `GET /api/sniffer`

Current session status. `running` is `false` until `POST /api/sniffer/start` succeeds.

### `POST /api/sniffer/start`

Starts a single bridge between two ports. Returns `409` if a session is already running, or `400` if the body is invalid.

```json
{
  "computerPort": "/dev/tty.usbserial-A",
  "radioPort": "/dev/tty.usbserial-B",
  "baudRate": 9600,
  "logFile": "optional-capture.json"
}
```

`computerPort` is the programming-cable side; `radioPort` is the radio side. They must be different paths.

### `POST /api/sniffer/stop`

Stops the running bridge and closes both serial ports. Safe to call when nothing is running.

### `GET /api/sniffer/log`

Returns status, coalesced packets captured in this session, and the on-disk serial log (when present).

The `file.data` payload uses the same SerialLogger JSON shape as ham-radio-driver (`SEND` / `RECV` entries). ham-radio-ui wraps this into a `springfield-ham-radio-sniffer-capture` document when you click **Save capture**.

Log data remains available after `POST /api/sniffer/stop` so captures can be saved once traffic finishes.

### `GET /api/sniffer/events`

Server-sent events stream. Each `message` is JSON:

- `{ "type": "status", "status": { ... } }`
- `{ "type": "packet", "packet": { "id", "timestamp", "elapsedMs", "direction", "data" } }`
- `{ "type": "error", "message": "...", "source": "computer" | "radio" }`

`data` is an array of byte values `0-255`. Direction is `COMPUTER->RADIO` or `RADIO->COMPUTER`.

## ham-radio-ui

The Sniffer tab talks to this server at `http://127.0.0.1:3010` by default. Change the URL under Preferences → Sniffer.
