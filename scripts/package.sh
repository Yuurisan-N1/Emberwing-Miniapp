#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAMP="$(date +%Y%m%d_%H%M)"
NAME="emberwing-bot-${STAMP}"
STAGE="${TMPDIR:-/tmp}/${NAME}"

rm -rf "$STAGE"
mkdir -p "$STAGE/cuda" "$STAGE/engine" "$STAGE/scripts"

cp -r "$ROOT/src" "$STAGE/src"
cp "$ROOT"/cuda/*.cu "$ROOT"/cuda/*.cuh "$STAGE/cuda/"
cp "$ROOT"/engine/*.h "$STAGE/engine/"
cp "$ROOT/scripts/build_cuda.sh" "$STAGE/scripts/build_cuda.sh"
cp "$ROOT/scripts/package.sh" "$STAGE/scripts/package.sh"
cp "$ROOT/Cargo.toml" "$STAGE/Cargo.toml"
cp "$ROOT/build.rs" "$STAGE/build.rs"

for f in Cargo.lock README.md LICENSE Makefile run.sh config.json .gitignore; do
  if [ -f "$ROOT/$f" ]; then cp "$ROOT/$f" "$STAGE/$f"; fi
done
for d in assets .github; do
  if [ -d "$ROOT/$d" ]; then cp -r "$ROOT/$d" "$STAGE/$d"; fi
done

printf '' > "$STAGE/proxy.txt"
printf 'paste one initData per line into data.txt\none proxy per line into proxy.txt, an empty file means direct\n' > "$STAGE/HOWTO.txt"

OUT="${TMPDIR:-/tmp}/${NAME}.zip"
rm -f "$OUT"
( cd "$STAGE/.." && zip -qr "$OUT" "$NAME" )

echo "$OUT"
unzip -l "$OUT" | tail -3
