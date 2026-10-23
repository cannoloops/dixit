#!/usr/bin/env bash
# Installs the locally built Trascrivi.app into /Applications and starts it with logs.
# Build first with: scripts/release.sh <version> --dry-run
# Logs: ~/Library/Logs/Trascrivi/trascrivi.log
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/target/release/bundle/macos/Trascrivi.app"
DEST="/Applications/Trascrivi.app"
LOG_DIR="$HOME/Library/Logs/Trascrivi"

[[ -d "$APP" ]] || { echo "Build mancante: $APP (lancia scripts/release.sh <versione> --dry-run)"; exit 1; }

if pgrep -f "$DEST/Contents/MacOS/" >/dev/null; then
  echo "==> Chiudo Trascrivi in esecuzione"
  osascript -e 'tell application id "com.vitomanuguerra.trascrivi" to quit' || true
  for _ in $(seq 1 30); do pgrep -f "$DEST/Contents/MacOS/" >/dev/null || break; sleep 1; done
  pgrep -f "$DEST/Contents/MacOS/" >/dev/null && { echo "Trascrivi non si è chiuso: chiudilo dal menu e riprova"; exit 1; }
fi

echo "==> Installo in $DEST"
rm -rf "$DEST"
ditto "$APP" "$DEST"

mkdir -p "$LOG_DIR"
echo "==> Avvio (log: $LOG_DIR/trascrivi.log)"
open -a "$DEST" --env RUST_LOG=info --stdout "$LOG_DIR/trascrivi.log" --stderr "$LOG_DIR/trascrivi.log"
