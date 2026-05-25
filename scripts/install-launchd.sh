#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

PORT=9001
BINARY="$REPO_DIR/target/release/streamdeck-bridge"
LABEL="com.streamdeck.bridge"
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST="$PLIST_DIR/$LABEL.plist"
LOG_DIR="$HOME/Library/Logs/streamdeck-bridge"

while [[ $# -gt 0 ]]; do
    case $1 in
        --port) PORT="$2"; shift 2 ;;
        --binary) BINARY="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: install-launchd.sh [OPTIONS]"
            echo ""
            echo "Install streamdeck-bridge as a macOS launchd user agent."
            echo "The service starts automatically on login and restarts on crash."
            echo ""
            echo "Options:"
            echo "  --port PORT     WebSocket port (default: 9001)"
            echo "  --binary PATH   Path to binary (default: target/release/streamdeck-bridge)"
            echo "  -h, --help      Show this help"
            echo ""
            echo "Examples:"
            echo "  ./scripts/install-launchd.sh"
            echo "  ./scripts/install-launchd.sh --port 8080"
            echo "  ./scripts/install-launchd.sh --binary /usr/local/bin/streamdeck-bridge"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

BINARY="$(cd "$(dirname "$BINARY")" 2>/dev/null && pwd)/$(basename "$BINARY")"

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    echo "Build first: cargo build --release  (or: scripts/build.sh)"
    exit 1
fi

mkdir -p "$PLIST_DIR" "$LOG_DIR"

if launchctl list 2>/dev/null | grep -q "$LABEL"; then
    echo "Stopping existing service..."
    launchctl unload "$PLIST" 2>/dev/null || true
fi

cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$LABEL</string>
    <key>ProgramArguments</key>
    <array>
        <string>$BINARY</string>
        <string>--port</string>
        <string>$PORT</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/stderr.log</string>
    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>
EOF

launchctl load "$PLIST"

echo "Installed and started streamdeck-bridge"
echo "  Port:      $PORT"
echo "  WebSocket: ws://localhost:$PORT"
echo "  Binary:    $BINARY"
echo "  Plist:     $PLIST"
echo "  Logs:      $LOG_DIR/"
echo ""
echo "The service starts automatically on login and restarts on crash."
echo "To stop:      launchctl unload $PLIST"
echo "To uninstall: $SCRIPT_DIR/uninstall-launchd.sh"
