/* Essai de faisabilité : charge N copies du noyau (une par instance), joue une note sur chacune, écrit un WAV par
 * instance et affiche une somme de contrôle. gcc drive.c -ldl -o drive ; ./drive noyau.so note1 [note2 ...] */
#define _GNU_SOURCE
#include <dlfcn.h>
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

static Inst load(const char *so, int k)
{
    char tmp[256]; char cmd[600]; Inst in;
    snprintf(tmp, sizeof tmp, "/tmp/tw_inst_%d.so", k);
    snprintf(cmd, sizeof cmd, "cp '%s' '%s'", so, tmp);
    if (system(cmd)) exit(2);
    in.h = dlopen(tmp, RTLD_NOW | RTLD_LOCAL);
    if (!in.h) { fprintf(stderr, "%s\n", dlerror()); exit(2); }
    in.advance = (void (*)(double))dlsym(in.h, "tw_advance");
    in.audio = (float *(*)(void))dlsym(in.h, "web_audio");
    in.midi = (void (*)(uint32_t, uint32_t, uint32_t))dlsym(in.h, "web_midi");
    in.enc = (void (*)(uint32_t, int32_t))dlsym(in.h, "web_enc");
    in.btn = (void (*)(uint32_t, uint32_t))dlsym(in.h, "web_btn");
    in.fb = (uint16_t *(*)(void))dlsym(in.h, "web_fb");
    return in;
}

int main(int argc, char **argv)
{
    int n = argc - 2, blocks = (getenv("BLOCS") ? atoi(getenv("BLOCS")) : 44100 * 2 / 256), boot = 100, b, k;
    Inst in[8]; FILE *wav[8]; uint64_t sum[8] = {0}; double peak[8] = {0};
    for (k = 0; k < n; k++) {
        char name[64]; in[k] = load(argv[1], k);
        snprintf(name, sizeof name, "/tmp/tw_%d.raw", k); wav[k] = fopen(name, "wb");
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
