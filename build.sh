#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo build --release

install_dir="${TXTWRK_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$install_dir"
install -m 0755 target/release/txtwrk "$install_dir/txtwrk"

echo "Installed txtwrk to $install_dir/txtwrk"
if [[ ":$PATH:" != *":$install_dir:"* ]]; then
    echo "Note: $install_dir is not in your PATH."
    echo "Add it with:  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
