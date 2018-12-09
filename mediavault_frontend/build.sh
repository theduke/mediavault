#! /bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null && pwd )"
cd $DIR

cargo build --target wasm32-unknown-unknown && \
mkdir -p ../target/web && \
cp assets/* ../target/web && \
cp ./index.html ../target/web/ && \
wasm-bindgen ../target/wasm32-unknown-unknown/debug/mediavault_frontend.wasm \
  --out-dir ../target/web \
  --no-modules \
  --no-modules-global MediaVault
