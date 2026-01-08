#!/usr/bin/env bash
set -e

# Build release binary
echo "Building release binary..."
cargo build --release --bin morphogenetic-security

BIN="target/release/morphogenetic-security"
CONFIG="docs/examples/intense-defense.yaml" # A reasonably complex scenario

echo "Running performance profile on $CONFIG..."

# Use simple time if hyperfine not available
if command -v hyperfine &> /dev/null; then
    hyperfine --warmup 3 --runs 10 "$BIN --config $CONFIG"
else
    echo "Hyperfine not found, using time..."
    time "$BIN" --config "$CONFIG"
fi

echo "Performance run complete."
