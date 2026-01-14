#!/bin/bash
set -e

echo "Building for GitHub Pages..."
echo ""

cd renderer
wasm-pack build --target web --out-dir pkg --no-default-features
cd ..

cd web
npm run build

