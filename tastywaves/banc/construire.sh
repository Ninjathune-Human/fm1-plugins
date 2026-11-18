#!/usr/bin/env bash
# Tout le banc : récupère le firmware au commit fixé, compile le noyau natif et le module WebAssembly, les compare.
# Outils : gcc, zig (0.17), wasm-opt, node.   banc/construire.sh [dossier de travail]
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../.." && pwd)
W=${1:-$ROOT/_banc}; COMMIT=12d3499d056ae4c93908783748ca3ddb5d4d61fc
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) NOYAU=tastywaves.dll; LIENS=""; EXE=.exe ;; *) NOYAU=tastywaves.so; LIENS="-ldl"; EXE="" ;; esac
mkdir -p "$W"; cd "$W"
if [ ! -d tw ]; then git clone -q --filter=blob:none https://github.com/marksweiss/tastywaves tw; fi
git -C tw checkout -q "$COMMIT"; git -C tw clean -fdq
cp "$ROOT/tastywaves/core/host.c" tw/tests/host.c
DEF=(-DDINGHY_HOST '-DAPP_VER="1.0"' -DDH_CV_ROWS=64u -Ifirmware/src -Ifirmware/gen firmware/src/tastywaves.c)
( cd tw
  gcc -O2 -shared -fPIC -w "${DEF[@]}" -I"$ROOT/tastywaves/core/shim" "$ROOT/tastywaves/core/runtime.c" -o "$W/$NOYAU"
  "$HERE/asyncify.sh" "$W/ref.wasm" "${DEF[@]}" )
gcc -O2 "$ROOT/tastywaves/spike/drive.c" $LIENS -o "$W/drive$EXE"
"$HERE/comparer.sh" "$W/$NOYAU" "$W/ref.wasm" "$W/drive$EXE"
