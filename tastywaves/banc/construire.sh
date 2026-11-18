#!/usr/bin/env bash
# Tout le banc : prend le firmware repris dans vendor/, compile le noyau natif et le module WebAssembly, les compare.
# Outils : gcc, zig (0.17), wasm-opt, node.   banc/construire.sh [dossier de travail]
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../.." && pwd)
W=${1:-$ROOT/_banc}; V=$ROOT/tastywaves/vendor
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) NOYAU=tastywaves.dll; LIENS=""; EXE=.exe ;; *) NOYAU=tastywaves.so; LIENS="-ldl"; EXE="" ;; esac
mkdir -p "$W"
DEF=(-DDINGHY_HOST '-DAPP_VER="1.0"' -DDH_CV_ROWS=64u -Ifirmware/src -Ifirmware/gen firmware/src/tastywaves.c)
( cd "$V"
  gcc -O2 -shared -fPIC -w "${DEF[@]}" -I"$ROOT/tastywaves/core/shim" "$ROOT/tastywaves/core/runtime.c" -o "$W/$NOYAU"
  "$HERE/asyncify.sh" "$W/ref.wasm" "${DEF[@]}" )
gcc -O2 "$ROOT/tastywaves/spike/drive.c" $LIENS -o "$W/drive$EXE"
"$HERE/comparer.sh" "$W/$NOYAU" "$W/ref.wasm" "$W/drive$EXE"
