# ham-radio-sniffer

Headless serial bridge for clone-protocol traffic. Sit it between the computer and the radio: a debug cable toward the computer, and a programming cable toward the radio.

The process is one Rust binary. It opens serial ports with the [`serialport`](https://crates.io/crates/serialport) crate (4.10.1). Control it from HamBench (**Radio → Sniffer**) or any HTTP client. The CLI remains available for terminal use.

Docs: [Sniffer user guide](https://springfield-ham-radio.github.io/ham-radio-docs/guide/sniffer.html) · [HTTP API](docs/api.md)

## Requirements

- Rust 1.85 or newer
- Two serial devices: debug cable (computer ↔ sniffer) and programming cable (sniffer ↔ radio)
- On Linux, `libudev` and `pkg-config` to build (`libudev-dev` on Debian)

```bash
cargo build --release
./target/release/ham-radio-sniffer
```

The API listens on [http://127.0.0.1:3010](http://127.0.0.1:3010). `HOST` and `PORT` override that. Loopback binds `127.0.0.1`; any other host binds `0.0.0.0`. There is no web UI in this project.

```bash
./target/release/ham-radio-sniffer --list-ports
./target/release/ham-radio-sniffer /dev/tty.usbserial-A /dev/tty.usbserial-B
```

`/api/health` reports the crate version from `Cargo.toml`.

## Linux ARM64

CI builds `ham-radio-sniffer-linux-aarch64` on `ubuntu-22.04-arm` and uploads it as an artifact. HamBench copies that binary when it installs the sniffer onto a Raspberry Pi.

## License

MIT. Copyright (c) 2026 Bryan Hunt. See [LICENSE](LICENSE).
