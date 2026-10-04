#ifndef FORGE_EV_CUH
#define FORGE_EV_CUH

#include "forge_core.cuh"

static __device__ __forceinline__ int d_pool(const G *g, signed char *pool) {
    int cnt[64];
    for (int i = 0; i < 64; i++) cnt[i] = 0;
    int nh = 0;
    for (int i = 0; i < g->n; i++) {
        if (g->loc[i] != 0) { int f = g->face[i]; if (f >= 0) cnt[f]++; }
        else if (g->face[i] >= 0) cnt[g->face[i]]++;
        else nh++;
    }
    int nf = 0;
    for (int f = 0; f < 64; f++) if (cnt[f] > 0) nf++;
    if (nf == 0 || g->n % nf != 0) return -1;
    int per = g->n / nf;
    if (per < 3) return -1;
    int np = 0;
    for (int f = 0; f < 64; f++) {
        if (cnt[f] <= 0) continue;
        int left = per - cnt[f];
        if (left < 0) return -1;
        for (int k = 0; k < left && np < MAXN; k++) pool[np++] = (signed char)f;
    }
    if (np != nh) return -1;
    return np;
}

static __device__ int d_force(G *g, int rm, int rf, int rb, signed char *pool, int np,
                              int depth, long long *budget) {
    if (g_won(g)) return 1;
    if (g_over(g) || depth <= 0) return 0;
    if (--(*budget) <= 0) return 0;
    int j = -1;
    for (int i = 0; i < g->n; i++)
        if (g->loc[i] == 0 && g->face[i] < 0 && g_free(g, i)) { j = i; break; }
    if (j >= 0) {
        signed char seen[MAXN];
        int ns = 0;
        for (int k = 0; k < np; k++) {
            int dup = 0;
            for (int q = 0; q < ns; q++) if (seen[q] == pool[k]) { dup = 1; break; }
            if (!dup) seen[ns++] = pool[k];
        }
        for (int s = 0; s < ns; s++) {
            G c = *g;
            signed char p2[MAXN];
            int n2 = 0, taken = 0;
            for (int k = 0; k < np; k++) {
                if (!taken && pool[k] == seen[s]) { taken = 1; continue; }
                p2[n2++] = pool[k];
            }
            c.face[j] = seen[s];
            int rm2 = rm, rf2 = rf, rb2 = rb;
            if (!d_force(&c, rm2, rf2, rb2, p2, n2, depth, budget)) return 0;
        }
        return 1;
    }
    int mv;
    if (!g_choose(g, rm, rf, rb, &mv)) return 0;
    G c = *g;
    int rm2 = rm, rf2 = rf, rb2 = rb;
    if (!g_apply(&c, mv, &rm2, &rf2, &rb2)) return 0;
    g_reveal(&c);
    if (g_over(&c)) return 0;
    return d_force(&c, rm2, rf2, rb2, pool, np, depth - 1, budget);
}

#endif
