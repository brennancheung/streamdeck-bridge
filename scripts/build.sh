#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_DIR"
cargo build --release

BINARY="./target/release/streamdeck-bridge"
echo ""
echo "Built: $BINARY"
echo ""
echo "Quick start:"
echo "  $BINARY --test     Verify your Stream Deck works"
echo "  $BINARY            Start the bridge server"
echo "  $BINARY --help     Show all options"
