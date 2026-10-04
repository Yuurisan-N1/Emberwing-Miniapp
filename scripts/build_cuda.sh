#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NVCC="${NVCC:-}"
if [ -z "$NVCC" ]; then
  for c in nvcc nvcc.exe /usr/local/cuda/bin/nvcc \
           "/c/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v13.4/bin/nvcc.exe"; do
    if command -v "$c" >/dev/null 2>&1; then NVCC="$c"; break; fi
  done
fi
if [ -z "$NVCC" ]; then
  echo "nvcc not found; forge stays on the cpu backend" >&2
  exit 1
fi

case "$(uname -s 2>/dev/null)" in
  MINGW*|MSYS*|CYGWIN*|Windows*) LIB="forge_mc.dll"; XC="" ;;
  *) LIB="libforge_mc.so"; XC="-Xcompiler -fPIC" ;;
esac

CUDA_HOME="$(dirname "$(dirname "$(command -v "$NVCC")")")"
LIBS="$CUDA_HOME/lib64"
[ -d "$LIBS" ] || LIBS="$CUDA_HOME/lib/x64"
[ -d "$LIBS" ] || LIBS="$CUDA_HOME/lib"
OUT="$ROOT/cuda/$LIB"

ARCH="${EMB_CUDA_ARCH:-}"
if [ -z "$ARCH" ] && command -v nvidia-smi >/dev/null 2>&1; then
  CAP="$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null | head -1 | tr -d ' .' || true)"
  [ -n "$CAP" ] && ARCH="sm_$CAP"
fi
if [ -z "$ARCH" ]; then
  ARCH="all-major"
fi

echo "nvcc  : $NVCC"
echo "libs  : $LIBS"
echo "arch  : $ARCH"

"$NVCC" -O3 -std=c++14 --shared $XC "-arch=$ARCH" \
  -I"$CUDA_HOME/include" -I"$ROOT/cuda" -L"$LIBS" \
  -o "$OUT" "$ROOT/cuda/forge_mc.cu" "$ROOT/cuda/forge_ev.cu" -lcudart

ls -l "$OUT"
MC="$(nm -D --defined-only "$OUT" 2>/dev/null | grep -c forge_mc || echo '?')"
EV="$(nm -D --defined-only "$OUT" 2>/dev/null | grep -c forge_ev || echo '?')"
echo "OK: $MC exported forge_mc symbols, $EV exported forge_ev symbols"
