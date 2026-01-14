#!/bin/bash
set -e

echo "Exporting GPS data to JSON for web viewer..."
echo ""

cargo run -p storage --bin export_json

echo ""
echo "✓ Export complete!"
echo ""
echo "To view in browser:"
echo "  cd web"
echo "  npm run dev"
