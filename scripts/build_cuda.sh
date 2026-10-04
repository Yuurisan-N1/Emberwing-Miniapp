#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CAP=""
if command -v nvidia-smi >/dev/null 2>&1; then
  CAP="$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null | head -1 | tr -d ' .' || true)"
fi
if [ -z "$CAP" ]; then CAP="75"; fi
if [ -n "${EMB_CUDA_CAP:-}" ]; then CAP="$EMB_CUDA_CAP"; fi

nv_major() {
  local v
  v="$("$1" --version 2>/dev/null | sed -n 's/.*release \([0-9][0-9]*\)\..*/\1/p' | head -1)"
  if [ -z "$v" ]; then v=0; fi
  echo "$v"
}

pick_nvcc() {
  if [ -n "${NVCC:-}" ]; then echo "$NVCC"; return; fi
  local list=()
  local c g
  for c in nvcc nvcc.exe; do
    if command -v "$c" >/dev/null 2>&1; then list+=("$(command -v "$c")"); fi
  done
  for g in /opt/*/bin/nvcc /usr/local/cuda-12*/bin/nvcc /usr/local/cuda-11*/bin/nvcc; do
    if [ -x "$g" ]; then list+=("$g"); fi
  done
  list+=(
    /opt/cu12/bin/nvcc
    /usr/local/cuda/bin/nvcc
    "/c/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/bin/nvcc.exe"
    "/c/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v13.4/bin/nvcc.exe"
  )
  local first=""
  for c in "${list[@]}"; do
    if [ ! -x "$c" ]; then continue; fi
    if [ -z "$first" ]; then first="$c"; fi
    local v
    v="$(nv_major "$c")"
    if [ "$CAP" -lt 75 ]; then
      if [ "$v" -lt 13 ] && [ "$v" -gt 0 ]; then echo "$c"; return; fi
    else
      echo "$c"; return
    fi
  done
  echo "$first"
}

NVCC="$(pick_nvcc)"

if [ -z "$NVCC" ]; then
  echo "nvcc not found; forge stays on the cpu backend" >&2
  echo "need CUDA 12.x (sm_60..sm_90):" >&2
  echo "  conda install -c nvidia cuda-nvcc=12.6 cuda-cudart-dev=12.6 cuda-cudart-static=12.6" >&2
  echo "  NVCC=/path/to/cuda-12.6/bin/nvcc scripts/build_cuda.sh" >&2
  exit 1
fi

NVV="$(nv_major "$NVCC")"

if [ "$NVV" -ge 13 ] && [ "$CAP" -lt 75 ]; then
  echo "nvcc $NVV cannot target sm_$CAP: CUDA 13 dropped Maxwell/Pascal/Volta." >&2
  echo "install CUDA 12.x nvcc and rerun:" >&2
  echo "  conda install -c nvidia cuda-nvcc=12.6 cuda-cudart-dev=12.6 cuda-cudart-static=12.6" >&2
  echo "  NVCC=/path/to/cuda-12.6/bin/nvcc scripts/build_cuda.sh" >&2
  exit 1
fi

case "$(uname -s 2>/dev/null)" in
  MINGW*|MSYS*|CYGWIN*|Windows*) LIB="forge_mc.dll"; XC=""; EXTRA="" ;;
  *) LIB="libforge_mc.so"; XC="-Xcompiler -fPIC"; EXTRA="-lpthread -ldl -lrt" ;;
esac

CUDA_HOME="$(dirname "$(dirname "$NVCC")")"
LIBS="$CUDA_HOME/lib64"
if [ ! -d "$LIBS" ]; then LIBS="$CUDA_HOME/lib/x64"; fi
if [ ! -d "$LIBS" ]; then LIBS="$CUDA_HOME/lib"; fi
if [ ! -d "$LIBS" ]; then LIBS="$CUDA_HOME/targets/x86_64-linux/lib"; fi
INC="$CUDA_HOME/include"
if [ ! -f "$INC/cuda_runtime.h" ]; then INC="$CUDA_HOME/targets/x86_64-linux/include"; fi
OUT="$ROOT/cuda/$LIB"

echo "nvcc  : $NVCC ($NVV)"
echo "libs  : $LIBS"
echo "arch  : sm_$CAP (EMB_CUDA_ARCH / EMB_CUDA_CAP override)"

GEN=()
if [ -n "${EMB_CUDA_ARCH:-}" ]; then
  GEN=("-arch=$EMB_CUDA_ARCH")
else
  for a in 60 61 75 86 89 90; do
    GEN+=("-gencode" "arch=compute_$a,code=sm_$a")
  done
  if [ "$NVV" -ge 13 ]; then
    GEN+=("-gencode" "arch=compute_120,code=sm_120")
  fi
  GEN+=("-gencode" "arch=compute_$CAP,code=compute_$CAP")
fi

LINK="-lcudart"
if [ -f "$LIBS/libcudart_static.a" ] || [ -f "$LIBS/cudart_static.lib" ]; then
  LINK="-lcudart_static $EXTRA"
fi

build() {
  "$NVCC" -O3 -std=c++14 --shared $XC "${GEN[@]}" \
    -I"$INC" -I"$ROOT/cuda" -L"$LIBS" \
    -o "$OUT" "$ROOT/cuda/forge_mc.cu" "$ROOT/cuda/forge_ev.cu" $LINK
}

if ! build 2>/tmp/forge_cuda_build.log; then
  echo "fat build failed, retrying native arch only" >&2
  tail -3 /tmp/forge_cuda_build.log >&2 || true
  GEN=("-gencode" "arch=compute_$CAP,code=sm_$CAP" "-gencode" "arch=compute_$CAP,code=compute_$CAP")
  build
fi

ls -l "$OUT"
MC="$(nm -D --defined-only "$OUT" 2>/dev/null | grep -c forge_mc || echo '?')"
EV="$(nm -D --defined-only "$OUT" 2>/dev/null | grep -c forge_ev || echo '?')"
echo "OK: $MC exported forge_mc symbols, $EV exported forge_ev symbols"
