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
echo "Copying WASM files..."
cp target/wasm32-unknown-unknown/release/open_miami.wasm .

# Create a simple JS loader if it doesn't exist
if [ ! -f "open_miami.js" ]; then
    echo "Creating JS loader..."
    cat > open_miami.js << 'EOF'
async function wasm_run() {
    const wasmSupported = (() => {
        try {
            if (typeof WebAssembly === "object"
                && typeof WebAssembly.instantiate === "function") {
                const module = new WebAssembly.Module(Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00));
                if (module instanceof WebAssembly.Module)
                    return new WebAssembly.Instance(module) instanceof WebAssembly.Instance;
            }
        } catch (e) {
        }
        return false;
    })();

    if (!wasmSupported) {
        console.error("WebAssembly is not supported in this browser");
        return;
    }

    // Load and instantiate the WASM module
    const response = await fetch('open_miami.wasm');
    const bytes = await response.arrayBuffer();
    const module = await WebAssembly.instantiate(bytes, {});

    console.log("WASM module loaded successfully!");
}
EOF
fi

echo ""
echo "✅ Build complete!"
echo ""
echo "To run the game:"
echo "  1. Start a local server:"
echo "     python3 -m http.server 8000"
echo "  2. Open http://localhost:8000 in your browser"
echo ""
echo "Files generated:"
echo "  - open_miami.wasm"
echo "  - open_miami.js"
echo "  - index.html (already exists)"
