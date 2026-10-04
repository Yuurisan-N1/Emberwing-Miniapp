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
for l in libforge_mc.so forge_mc.dll libforge_mc.dll libforge_mc.dylib forge_mc.dylib; do
  if [ -f "$ROOT/cuda/$l" ]; then cp "$ROOT/cuda/$l" "$STAGE/cuda/$l"; fi
done
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
printf 'paste one initData per line into data.txt\none proxy per line into proxy.txt, an empty file means direct\ngpu: optional. the bot picks gpu or cpu by itself, no config needed.\nthe prebuilt accelerator is per-os: cuda/libforge_mc.so (linux), cuda/forge_mc.dll (windows), cuda/libforge_mc.dylib (macos).\ndrop in only the one matching your os, or none at all: without it the bot runs on the cpu.\nit links cudart statically, so no LD_LIBRARY_PATH and no cuda install is required.\nto build it for your own machine/card: scripts/build_cuda.sh (needs a cuda toolkit with nvcc).\n' > "$STAGE/HOWTO.txt"

OUT="${TMPDIR:-/tmp}/${NAME}.zip"
rm -f "$OUT"
( cd "$STAGE/.." && zip -qr "$OUT" "$NAME" )

echo "$OUT"
unzip -l "$OUT" | tail -3
