# Firmware repris

Source : [marksweiss/tastywaves](https://github.com/marksweiss/tastywaves), commit `12d3499d056ae4c93908783748ca3ddb5d4d61fc`, licence GPL-3.0 (voir `LICENSE` et `LICENSING.md`).

Repris sans modification : `firmware/src/` et `firmware/gen/`.
Remplacé : `tests/host.c`, la couche hôte du firmware (adaptation de celle de Dinghy, voir `../../README.md` du dépôt Fm1 : `tools/worklets/tastywaves_host.c`), qui fournit l'audio par blocs de 256 images, les notes, le MIDI, les boutons, les encodeurs, l'écran et l'état conservé.
