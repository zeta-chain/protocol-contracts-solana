#!/bin/bash

cargo +nightly rustdoc -p gateway --lib -- -Z unstable-options --output-format json

mkdir -p docs

rustdoc-md --path target/doc/gateway.json --output docs/gateway.md && sed -i '' 's/^# Arguments$/\nArguments:\n/' docs/gateway.md