# ham-radio-sniffer

Headless serial bridge for clone-protocol traffic. Sit it between the computer and the radio: a debug cable toward the computer, and a programming cable toward the radio.

Control it from HamBench (**Radio → Sniffer**) or any HTTP client. The CLI remains available for terminal use.

Docs: [Sniffer user guide](https://springfield-ham-radio.github.io/ham-radio-docs/guide/sniffer.html) · [HTTP API](docs/api.md)

## Requirements

- Node.js 24 (see `.nvmrc`)
- Two serial devices: debug cable (computer ↔ sniffer) and programming cable (sniffer ↔ radio)

```bash
yarn install
yarn dev
```

The API listens on [http://127.0.0.1:3010](http://127.0.0.1:3010). There is no web UI in this project.

```bash
yarn sniff --list-ports
yarn sniff /dev/tty.usbserial-A /dev/tty.usbserial-B
```
