#!/usr/bin/env bash
set -euo pipefail

LABEL="com.streamdeck.bridge"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
LOG_DIR="$HOME/Library/Logs/streamdeck-bridge"

if [ ! -f "$PLIST" ]; then
    echo "Not installed (no plist at $PLIST)"
    exit 0
fi

launchctl unload "$PLIST" 2>/dev/null || true
rm -f "$PLIST"

echo "Uninstalled streamdeck-bridge launchd service"
echo "  Removed: $PLIST"

if [ -d "$LOG_DIR" ]; then
    echo ""
    echo "Logs are still at $LOG_DIR/ (remove manually if desired)"
fi
