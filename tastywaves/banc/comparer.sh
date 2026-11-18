#!/usr/bin/env bash
# Compare le noyau natif au module WebAssembly du site sur plusieurs scénarios (sortie audio et écran, bit à bit).
#   banc/comparer.sh <noyau.so> <noyau.wasm> <pilote natif> ; échoue au premier écart
set -u
SO=$1; WASM=$2; DRV=$3; HERE=$(cd "$(dirname "$0")" && pwd); rc=0
for sc in "60:" "64:SCENE=1" "40:ACCORD=1" "48:ACCORD=1 SCENE=1 BLOCS=3000"; do
  note=${sc%%:*}; env=${sc#*:}
  a=$(env $env "$DRV" "$SO" "$note" | tr -d '\r' | sed 's/instance 0 note [0-9]* : //')
  b=$(env $env node "$HERE/ref.mjs" "$WASM" "$note" | tr -d '\r' | sed 's/instance 0 note [0-9]* : //')
  if [ "$a" = "$b" ]; then echo "identique  note $note $env"; else echo "ÉCART      note $note $env"; echo "  natif : $a"; echo "  wasm  : $b"; rc=1; fi
done
exit $rc
