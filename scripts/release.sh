#!/usr/bin/env bash
# Dixit release: build, sign, notarize if the Apple ID is set, publish on cannoloops/dixit.
# Usage: scripts/release.sh <version> [--dry-run]     (see docs/RELEASE.md)
set -euo pipefail

VERSION="${1:?usage: scripts/release.sh <version> [--dry-run]}"
DRY_RUN=false
[[ "${2:-}" == "--dry-run" ]] && DRY_RUN=true
RELEASES_REPO="cannoloops/dixit"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KEY="$HOME/.tauri/trascrivi.key"
TRIPLE="aarch64-apple-darwin"

# Ogni release ha il suo changelog breve in testa a docs/release-notes.md («**Novità in <versione>**»).
grep -q "Novità in $VERSION\*\*" "$ROOT/docs/release-notes.md" || { echo "Manca il changelog di $VERSION in docs/release-notes.md"; exit 1; }
[[ -f "$KEY" ]] || { echo "Chiave updater mancante: $KEY"; exit 1; }
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""

# Developer ID + APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID: firmata e notarizzata.
# Altrimenti firma con APPLE_SIGNING_IDENTITY (es. «Dixit Local Signing») o ad hoc, senza notarizzazione.
SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
[[ -n "${APPLE_ID:-}" ]] || echo "Nessun APPLE_ID: release non notarizzata"
$DRY_RUN && unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID

echo "==> Versione $VERSION"
python3 - "$ROOT" "$VERSION" <<'PY'
import json, re, sys
root, version = sys.argv[1], sys.argv[2]
for path in (f"{root}/frontend/src-tauri/tauri.conf.json", f"{root}/frontend/package.json"):
    data = json.load(open(path))
    data["version"] = version
    with open(path, "w") as f:
        json.dump(data, f, indent=4, ensure_ascii=False)
        f.write("\n")
cargo = f"{root}/frontend/src-tauri/Cargo.toml"
text = open(cargo).read()
text = re.sub(r'(?m)^version = "[^"]+"', f'version = "{version}"', text, count=1)
open(cargo, "w").write(text)
PY

echo "==> Sidecar llama-helper"
(cd "$ROOT/llama-helper" && cargo build --release --features metal)
cp "$ROOT/target/release/llama-helper" "$ROOT/frontend/src-tauri/binaries/llama-helper-$TRIPLE"

echo "==> Build app + dmg (firma: $SIGNING_IDENTITY)"
(cd "$ROOT/frontend" && pnpm tauri build --bundles app,dmg \
  --config "{\"bundle\":{\"macOS\":{\"signingIdentity\":\"$SIGNING_IDENTITY\"}}}")

BUNDLE="$ROOT/target/release/bundle"
DMG="$(ls "$BUNDLE"/dmg/Dixit_"${VERSION}"_*.dmg)"
TARBALL="$BUNDLE/macos/Dixit.app.tar.gz"
[[ -f "$TARBALL.sig" ]] || { echo "Firma updater mancante: $TARBALL.sig"; exit 1; }

echo "==> latest.json"
python3 - "$VERSION" "$TARBALL.sig" "https://github.com/$RELEASES_REPO/releases/download/v$VERSION/Dixit.app.tar.gz" > "$BUNDLE/latest.json" <<'PY'
import datetime, json, sys
version, sig_path, url = sys.argv[1:4]
print(json.dumps({
    "version": version,
    "notes": f"Dixit {version}",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "platforms": {"darwin-aarch64": {"signature": open(sig_path).read().strip(), "url": url}},
}, indent=2))
PY

if $DRY_RUN; then
  echo "Dry run completato: $DMG"
  exit 0
fi

echo "==> Pubblicazione su $RELEASES_REPO"
gh release create "v$VERSION" --repo "$RELEASES_REPO" --title "Dixit $VERSION" --notes-file "$ROOT/docs/release-notes.md" \
  "$DMG" "$TARBALL" "$TARBALL.sig" "$BUNDLE/latest.json"
