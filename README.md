# ham-radio-sniffer

Headless serial bridge for clone-protocol traffic between a programming cable and a ham radio.

Control it from ham-radio-ui (Sniffer tab) or any HTTP client. The CLI remains available for terminal use.

## Requirements

- Node.js 24 (see `.nvmrc`)
- Two serial devices: one toward the computer/programming software, one toward the radio

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

## Production

```bash
yarn build
yarn start
```

Override the listen port with `PORT` after `yarn build` / `yarn start` if needed.

## Tests

```bash
yarn test
```
