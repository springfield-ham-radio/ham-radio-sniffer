#!/bin/sh
set -eu

cd "$(dirname "$0")"

cargo fetch
cargo build
