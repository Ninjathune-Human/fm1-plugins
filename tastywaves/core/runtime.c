/* Coroutine et temps virtuel du firmware : tw_advance(ms) fait avancer l'horloge de ms millisecondes et laisse la
 * boucle principale du firmware (main) tourner jusqu'à sa prochaine attente. Déterministe, sans fil ni verrou. */
#define _GNU_SOURCE
#include <stdlib.h>
#include "emscripten.h"
#ifdef _WIN32
#include <windows.h>
#else
#include <ucontext.h>
#endif

extern int main(void);
static double now_ms;
static int started, finished;
#ifdef _WIN32                                   /* Windows : des fibres */
static void *host_fiber, *app_fiber;
static void CALLBACK entry(void *arg) { (void)arg; main(); finished = 1; SwitchToFiber(host_fiber); }
static void to_app(void)
{
    if (!started) {
        host_fiber = IsThreadAFiber() ? GetCurrentFiber() : ConvertThreadToFiber(0);
        app_fiber = CreateFiber(1u << 20, entry, 0);
        started = 1;
    }
    SwitchToFiber(app_fiber);
}
static void to_host(void) { SwitchToFiber(host_fiber); }
#else                                           /* ailleurs : ucontext */
static ucontext_t host_ctx, app_ctx;
static char *app_stack;
static void entry(void) { main(); finished = 1; }
static void to_app(void)
{
    if (!started) {
        const size_t n = 1u << 20;
        app_stack = malloc(n);
        getcontext(&app_ctx);
        app_ctx.uc_stack.ss_sp = app_stack; app_ctx.uc_stack.ss_size = n; app_ctx.uc_link = &host_ctx;
        makecontext(&app_ctx, entry, 0);
        started = 1;
    }
    swapcontext(&host_ctx, &app_ctx);
}
static void to_host(void) { swapcontext(&app_ctx, &host_ctx); }
#endif

double emscripten_get_now(void) { return now_ms; }
void emscripten_sleep(unsigned int ms) { (void)ms; to_host(); }

EMSCRIPTEN_KEEPALIVE void tw_advance(double ms)
{
    now_ms += ms;
    if (!finished) to_app();
}
