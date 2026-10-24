#!/usr/bin/env bash
# Fills src-tauri/resources/models/ with the models bundled in the .app (Contents/Resources/models).
# Same layout as the user's models dir, so the app looks there first and then in the bundle.
# Files already in the user's models dir are cloned (APFS clonefile, no extra space), the rest downloaded.
set -euo pipefail

cd "$(dirname "$0")/.."
DEST="src-tauri/resources/models"
USER_MODELS="$HOME/Library/Application Support/com.vitomanuguerra.trascrivi/models"
HF="https://huggingface.co"

# fetch <relative path under models/> <url>
fetch() {
  local rel="$1" url="$2" out="$DEST/$1"
  [ -s "$out" ] && { echo "ok      $rel"; return; }
  mkdir -p "$(dirname "$out")"
  if [ -s "$USER_MODELS/$rel" ]; then
    echo "clone   $rel"
    cp -c "$USER_MODELS/$rel" "$out" 2>/dev/null || cp "$USER_MODELS/$rel" "$out"
  else
    echo "scarico $rel"
    curl -fL --retry 3 -o "$out.part" "$url"
    mv "$out.part" "$out"
  fi
}

# Riassunto: Gemma 4 E4B Q4_K_M (provider interno, llama-helper)
fetch summary/gemma-4-E4B-it-Q4_K_M.gguf "$HF/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf"

# Trascrizione definitiva: Whisper large-v3
fetch ggml-large-v3.bin "$HF/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"

# Trascrizione live: Parakeet TDT 0.6B v3 int8
P=parakeet/parakeet-tdt-0.6b-v3-int8
PURL="https://meetily.towardsgeneralintelligence.com/models/parakeet-tdt-0.6b-v3-onnx"
for f in encoder-model.int8.onnx decoder_joint-model.int8.onnx nemo128.onnx vocab.txt; do
  fetch "$P/$f" "$PURL/$f"
done

# Parlanti: wespeaker
fetch speaker/wespeaker_en_voxceleb_resnet34_LM.onnx \
  "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34_LM.onnx"

# Embedding per la ricerca: models/embedding/<nome>/<file>. Attivo quando il modello è scelto, es.
#   EMBEDDING_FILES="embeddinggemma-300m/model.onnx=<url> embeddinggemma-300m/tokenizer.json=<url>"
for pair in ${EMBEDDING_FILES:-}; do
  fetch "embedding/${pair%%=*}" "${pair#*=}"
done

du -sh "$DEST"
