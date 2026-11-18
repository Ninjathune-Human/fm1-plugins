# Essai de faisabilité du noyau natif (phase 1)

Le firmware de TastyWaves est compilé tel quel en bibliothèque dynamique pour PC, avec un `emscripten.h` de remplacement (`../core/shim`) et une coroutine à temps virtuel (`../core/runtime.c`). `drive.c` charge une copie de la bibliothèque par instance et joue une note.

Reproduire (Linux) : sources du firmware dans `tastywaves/vendor` (commit 12d3499), puis

    gcc -O2 -shared -fPIC -w -DDINGHY_HOST '-DAPP_VER="1.0"' -DDH_CV_ROWS=64u -Itastywaves/core/shim \
        -Itastywaves/vendor/firmware/src -Itastywaves/vendor/firmware/gen tastywaves/vendor/firmware/src/tastywaves.c tastywaves/core/runtime.c -o tastywaves.so
    gcc -O2 tastywaves/spike/drive.c -ldl -o drive
    ./drive tastywaves.so 60 64          # deux instances, deux notes

Résultats (18 novembre 2026, Linux x86-64, Xeon 2,1 GHz) :
- compilation sans modification du firmware, 64 bits ;
- do central : pic à 261,9 Hz (attendu 261,6), mi : 329,5 Hz (attendu 329,6) ;
- deux instances dans le même processus : sorties identiques, bit à bit, à celles de chaque instance seule ;
- 12 voix tenues : 100 s d'audio rendues en 1,0 s, soit environ 1 % d'un cœur.

Pas encore testé : Windows (fibres au lieu d'`ucontext`, `LoadLibrary` d'une copie), comparaison avec le module WebAssembly du site (phase 2).
