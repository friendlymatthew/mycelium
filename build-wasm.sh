#!/bin/bash
set -e

echo "Building WASM module..."
cd renderer
wasm-pack build --target web --out-dir pkg --no-default-features

echo ""
echo "WASM module built successfully!"
echo "Location: renderer/pkg/"
echo ""
echo "To run the web app:"
echo "  cd web"
echo "  npm install  # First time only"
echo "  npm run dev"
