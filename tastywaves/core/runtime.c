/* Coroutine et temps virtuel du firmware : tw_advance(ms) fait avancer l'horloge de ms millisecondes et laisse la
 * boucle principale du firmware (main) tourner jusqu'à sa prochaine attente. Déterministe, sans fil ni verrou. */
#define _GNU_SOURCE
#include <stdlib.h>
#include <ucontext.h>
#include "emscripten.h"

extern int main(void);
static ucontext_t host_ctx, app_ctx;
static char *app_stack;
static double now_ms;
static int started, finished;

double emscripten_get_now(void) { return now_ms; }
void emscripten_sleep(unsigned int ms) { (void)ms; swapcontext(&app_ctx, &host_ctx); }
static void entry(void) { main(); finished = 1; }

EMSCRIPTEN_KEEPALIVE void tw_advance(double ms)
{
    now_ms += ms;
    if (!started) {
        const size_t n = 1u << 20;
        app_stack = malloc(n);
        getcontext(&app_ctx);
        app_ctx.uc_stack.ss_sp = app_stack; app_ctx.uc_stack.ss_size = n; app_ctx.uc_link = &host_ctx;
        makecontext(&app_ctx, entry, 0);
        started = 1;
    }
    if (!finished) swapcontext(&host_ctx, &app_ctx);
}
