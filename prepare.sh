#!/bin/sh
set -eu

cd "$(dirname "$0")"

yarn install
yarn prepare

