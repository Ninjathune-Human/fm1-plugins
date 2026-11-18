/* Essai de faisabilité : charge N copies du noyau (une par instance), joue une note sur chacune, écrit un WAV par
 * instance et affiche une somme de contrôle. gcc drive.c -ldl -o drive (Linux ; sous Windows : gcc drive.c -o drive) ; ./drive noyau.so note1 [note2 ...] */
#define _GNU_SOURCE
#ifdef _WIN32
#include <windows.h>
#define OUVRIR(p) ((void *)LoadLibraryA(p))
#define SYM(h, n) ((void *)GetProcAddress((HMODULE)(h), n))
#else
#include <dlfcn.h>
#define OUVRIR(p) dlopen(p, RTLD_NOW | RTLD_LOCAL)
#define SYM(h, n) dlsym(h, n)
#endif
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    void *h;
    void (*advance)(double);
    float *(*audio)(void);
    void (*midi)(uint32_t, uint32_t, uint32_t);
    void (*enc)(uint32_t, int32_t);
    void (*btn)(uint32_t, uint32_t);
    uint16_t *(*fb)(void);
} Inst;

static const char *tmpdir(void)
{
    const char *t = getenv("TMPDIR"); if (!t) t = getenv("TEMP"); return t ? t : "/tmp";
}
static int copier(const char *de, const char *vers)
{
    FILE *a = fopen(de, "rb"), *b = fopen(vers, "wb"); char buf[65536]; size_t n;
    if (!a || !b) return 1;
    while ((n = fread(buf, 1, sizeof buf, a)) > 0) fwrite(buf, 1, n, b);
    fclose(a); fclose(b); return 0;
}
static Inst load(const char *so, int k)
{
    char tmp[512]; Inst in;
#ifdef _WIN32
    snprintf(tmp, sizeof tmp, "%s\\tw_inst_%d.dll", tmpdir(), k);
#else
    snprintf(tmp, sizeof tmp, "%s/tw_inst_%d.so", tmpdir(), k);
#endif
    if (copier(so, tmp)) { fprintf(stderr, "copie impossible\n"); exit(2); }
    in.h = OUVRIR(tmp);
    if (!in.h) { fprintf(stderr, "chargement impossible\n"); exit(2); }
    in.advance = (void (*)(double))SYM(in.h, "tw_advance");
    in.audio = (float *(*)(void))SYM(in.h, "web_audio");
    in.midi = (void (*)(uint32_t, uint32_t, uint32_t))SYM(in.h, "web_midi");
    in.enc = (void (*)(uint32_t, int32_t))SYM(in.h, "web_enc");
    in.btn = (void (*)(uint32_t, uint32_t))SYM(in.h, "web_btn");
    in.fb = (uint16_t *(*)(void))SYM(in.h, "web_fb");
    return in;
}

int main(int argc, char **argv)
{
    int n = argc - 2, blocks = (getenv("BLOCS") ? atoi(getenv("BLOCS")) : 44100 * 2 / 256), boot = 100, b, k;
    Inst in[8]; FILE *wav[8]; uint64_t sum[8] = {0}; double peak[8] = {0};
    for (k = 0; k < n; k++) {
        char name[64]; in[k] = load(argv[1], k);
        snprintf(name, sizeof name, "%s/tw_%d.raw", tmpdir(), k); wav[k] = fopen(name, "wb");
    }
    for (b = 0; b < boot + blocks; b++) {
        for (k = 0; k < n; k++) {
            if (b == boot) {
                in[k].midi(0x90, (uint32_t)atoi(argv[2 + k]), 100);
                if (getenv("ACCORD")) for (int j = 1; j < 12; j++) in[k].midi(0x90, (uint32_t)atoi(argv[2 + k]) + 2u * j, 100);   /* 12 voix */
            }
            if (getenv("SCENE")) {                  /* des encodeurs et un bouton, comme ref.mjs */
                if (b == boot + 50) { in[k].enc(0, 3); in[k].enc(1, -2); }
                if (b == boot + 60) in[k].btn(2, 1);
                if (b == boot + 70) in[k].btn(2, 0);
            }
            in[k].advance(256.0 * 1000.0 / 44100.0);
            const float *o = in[k].audio();
            for (int i = 0; i < 512; i++) {
                int16_t s = (int16_t)(o[i] * 32767.0f); uint32_t u; memcpy(&u, &o[i], 4);
                sum[k] = sum[k] * 1099511628211ull + u;
                if (o[i] > peak[k]) peak[k] = o[i];
                if (b >= boot) fwrite(&s, 2, 1, wav[k]);
            }
        }
    }
    for (k = 0; k < n; k++) {                      /* somme de l'écran */
        uint64_t f = 0; const uint16_t *px = in[k].fb();
        for (int i = 0; i < 240 * 240; i++) f = f * 1099511628211ull + px[i];
        printf("instance %d écran %016llx\n", k, (unsigned long long)f);
    }
    for (k = 0; k < n; k++) printf("instance %d note %s : somme %016llx crête %.3f\n", k, argv[2 + k], (unsigned long long)sum[k], peak[k]);
    return 0;
}
