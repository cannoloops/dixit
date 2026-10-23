#!/usr/bin/env bash
# Installs the locally built Dixit.app into /Applications and starts it with logs.
# Build first with: scripts/release.sh <version> --dry-run
# Logs: ~/Library/Logs/Dixit/dixit.log
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/target/release/bundle/macos/Dixit.app"
DEST="/Applications/Dixit.app"
OLD="/Applications/Trascrivi.app"   # nome precedente: stessa bundle id, va rimossa
LOG_DIR="$HOME/Library/Logs/Dixit"

[[ -d "$APP" ]] || { echo "Build mancante: $APP (lancia scripts/release.sh <versione> --dry-run)"; exit 1; }

# Chiude l'app installata in $1 (se in esecuzione).
quit_app() {
  local name; name="$(basename "$1")"
  if pgrep -f "$1/Contents/MacOS/" >/dev/null; then
    echo "==> Chiudo $name in esecuzione"
    osascript -e 'tell application id "com.vitomanuguerra.trascrivi" to quit' || true
    for _ in $(seq 1 30); do pgrep -f "$1/Contents/MacOS/" >/dev/null || break; sleep 1; done
    if pgrep -f "$1/Contents/MacOS/" >/dev/null; then
      echo "$name non si è chiuso: chiudilo dal menu e riprova"; exit 1
    fi
  fi
}

quit_app "$DEST"
if [[ -d "$OLD" ]]; then
  quit_app "$OLD"
  echo "==> Rimuovo $OLD"
  rm -rf "$OLD"
fi

echo "==> Installo in $DEST"
rm -rf "$DEST"
ditto "$APP" "$DEST"

mkdir -p "$LOG_DIR"
echo "==> Avvio (log: $LOG_DIR/dixit.log)"
open -a "$DEST" --env RUST_LOG=info --stdout "$LOG_DIR/dixit.log" --stderr "$LOG_DIR/dixit.log"
