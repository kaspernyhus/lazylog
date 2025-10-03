#!/bin/bash
set -e

echo "Building lazylog..."
cargo build --release

echo "Installing to /usr/local/bin..."
sudo cp target/release/lazylog /usr/local/bin/

echo "Installation complete! Run 'lazylog --help' to get started."
