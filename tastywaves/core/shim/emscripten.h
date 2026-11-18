/* emscripten.h natif : le firmware attend par emscripten_sleep (une milliseconde de son temps) ; ici le temps est
 * virtuel, mené par le plugin, et l'attente rend la main par une coroutine (runtime.c). */
#pragma once
#ifdef _WIN32
#define EMSCRIPTEN_KEEPALIVE __declspec(dllexport)
#else
#define EMSCRIPTEN_KEEPALIVE __attribute__((used, visibility("default")))
#endif
double emscripten_get_now(void);
void emscripten_sleep(unsigned int ms);
