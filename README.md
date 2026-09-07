# ham-radio-sniffer

Headless serial bridge for clone-protocol traffic. Sit the sniffer between the computer and the radio: a debug cable toward the computer, and a programming cable toward the radio.

Control it from ham-radio-ui (Sniffer tab) or any HTTP client. The CLI remains available for terminal use.

## Requirements

- Node.js 24 (see `.nvmrc`)
- Two serial devices: debug cable (computer ↔ sniffer) and programming cable (sniffer ↔ radio)

## Develop

```bash
yarn install
yarn dev
```

The API listens on [http://127.0.0.1:3010](http://127.0.0.1:3010). There is no web UI in this project.

## HTTP API

See [docs/api.md](docs/api.md).

## CLI

```bash
yarn sniff --list-ports
yarn sniff /dev/tty.usbserial-A /dev/tty.usbserial-B
yarn sniff /dev/tty.usbserial-A /dev/tty.usbserial-B 9600 --log-file capture.json
```

Set `SNIFFER_LOG_LEVEL=debug` (or `LOG_LEVEL=debug`) to print every serial chunk as hex. `info` (default) logs port open/close and the first bytes seen on each port.

## Production

```bash
yarn build
yarn start
```

`yarn start` defaults to `127.0.0.1:3010`. Override with `HOST` and `PORT` (for example `HOST=0.0.0.0 PORT=3010 yarn start` to accept LAN clients). `GET /api/health` includes the package version.

## Tests

```bash
yarn test
```
