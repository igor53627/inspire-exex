#!/bin/bash
set -e

echo "Building alloy-wasm..."
cd ../alloy-wasm
wasm-pack build --target web --release

echo "Copying to burner-wallet/static/pkg..."
mkdir -p ../burner-wallet/static/pkg
cp -r pkg/* ../burner-wallet/static/pkg/

echo "WASM size:"
ls -lh ../burner-wallet/static/pkg/*.wasm

echo "[OK] WASM packages ready"
