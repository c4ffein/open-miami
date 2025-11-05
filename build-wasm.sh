#!/bin/bash

echo "🔥 Building Open Miami for the web... 🔥"

# Check if wasm32 target is installed
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build the project
echo "Building WASM binary..."
cargo build --release --target wasm32-unknown-unknown

# Copy the WASM file to the root directory for easier serving
echo "Copying WASM file..."
cp target/wasm32-unknown-unknown/release/open_miami.wasm .

# Verify that the Macroquad JS loader exists
if [ ! -f "open_miami.js" ]; then
    echo "ERROR: open_miami.js (Macroquad JS loader) not found!"
    echo "This file should be committed in the repository."
    exit 1
fi
echo "✓ open_miami.js found"

echo ""
echo "✅ Build complete!"
echo ""
echo "To run the game:"
echo "  1. Start a local server:"
echo "     python3 -m http.server 8000"
echo "  2. Open http://localhost:8000 in your browser"
echo ""
echo "Files ready:"
echo "  - open_miami.wasm (generated)"
echo "  - open_miami.js (Macroquad JS loader)"
echo "  - index.html"
