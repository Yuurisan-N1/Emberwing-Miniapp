#ifndef FORGE_CORE_CUH
#define FORGE_CORE_CUH

#include <cuda_runtime.h>

#ifndef MAXN
#define MAXN 256
#endif
#ifndef MAXSLOT
#define MAXSLOT 8
#endif
#ifndef MAXMOVES
#define MAXMOVES 400
#endif

typedef struct {
    int n;
    int slots;
    int stash;
    signed char loc[MAXN];
    signed char face[MAXN];
    unsigned long long up[MAXN];
    signed char tray[MAXSLOT];
    int ntray;
    signed char side[MAXSLOT];
    int nside;
    signed char truth[MAXN];
} G;

static __device__ __forceinline__ int g_free(const G *g, int i) {
    unsigned long long m = g->up[i];
    while (m) {
        int j = __ffsll((long long)m) - 1;
        m &= m - 1;
        if (g->loc[j] == 0) return 0;
    }
    return 1;
}

static __device__ __forceinline__ void g_judge(G *g) {
    int on = 0;
    for (int i = 0; i < g->n; i++) if (g->loc[i] == 0) on++;
    if (on == 0 && g->ntray == 0 && g->nside == 0) return;
    if (g->ntray >= g->slots) { g->nside = -1; }
}

static __device__ __forceinline__ int g_over(const G *g) { return g->nside < 0; }

static __device__ __forceinline__ int g_won(const G *g) {
    if (g->nside < 0) return 0;
    for (int i = 0; i < g->n; i++) if (g->loc[i] == 0) return 0;
    return (g->ntray == 0 && g->nside == 0) ? 1 : 0;
}

static __device__ __forceinline__ int g_tap(G *g, int id) {
    if (g_over(g)) return 0;
    int from = g->loc[id];
    if (from == 0) {
        if (!g_free(g, id)) return 0;
    } else if (from == 2) {
        int p = -1;
        for (int i = 0; i < g->nside; i++) if (g->side[i] == id) { p = i; break; }
        if (p < 0) return 0;
        for (int i = p; i + 1 < g->nside; i++) g->side[i] = g->side[i + 1];
        g->nside--;
    } else {
        return 0;
    }
    int at = -1;
    for (int i = 0; i < g->ntray; i++) if (g->face[g->tray[i]] == g->face[id]) at = i;
    int pos = (at < 0) ? g->ntray : at + 1;
    if (g->ntray >= MAXSLOT) return 0;
    for (int i = g->ntray; i > pos; i--) g->tray[i] = g->tray[i - 1];
    g->tray[pos] = (signed char)id;
    g->ntray++;
    g->loc[id] = 1;
    int cnt = 0;
    for (int i = 0; i < g->ntray; i++) if (g->face[g->tray[i]] == g->face[id]) cnt++;
    if (cnt >= 3) {
        signed char keep[MAXSLOT];
        int nk = 0, drop = 0;
        for (int i = 0; i < g->ntray; i++) {
            signed char t = g->tray[i];
            if (g->face[t] == g->face[id] && drop < 3) { drop++; g->loc[t] = 3; }
            else keep[nk++] = t;
        }
        for (int i = 0; i < nk; i++) g->tray[i] = keep[i];
        g->ntray = nk;
    }
    g_judge(g);
    return 1;
}

static __device__ __forceinline__ int g_remove(G *g) {
    if (g_over(g) || g->ntray == 0) return 0;
    int k = g->ntray < 3 ? g->ntray : 3;
    if (g->stash - g->nside < k) return 0;
    for (int i = 0; i < k; i++) {
        signed char t = g->tray[i];
        if (g->nside < MAXSLOT) g->side[g->nside++] = t;
        g->loc[t] = 2;
    }
    for (int i = 0; i < g->ntray - k; i++) g->tray[i] = g->tray[i + k];
    g->ntray -= k;
    return 1;
}

static __device__ __forceinline__ int g_refresh_ok(const G *g) {
    if (g->ntray == g->slots - 1) {
        int dup = 0;
        for (int i = 0; i < g->ntray && !dup; i++)
            for (int j = i + 1; j < g->ntray; j++)
                if (g->face[g->tray[i]] == g->face[g->tray[j]]) { dup = 1; break; }
        if (!dup) return 0;
    }
    int on = 0;
    for (int i = 0; i < g->n; i++) if (g->loc[i] == 0) on++;
    return on >= 2;
}

static __device__ __forceinline__ int g_rollback(G *g) {
    if (g_over(g) || g->ntray == 0) return 0;
    int t = g->tray[0];
    for (int i = 0; i < g->ntray; i++) if (g->tray[i] > t) t = g->tray[i];
    int p = 0;
    for (int i = 0; i < g->ntray; i++) if (g->tray[i] == t) { p = i; break; }
    for (int i = p; i + 1 < g->ntray; i++) g->tray[i] = g->tray[i + 1];
    g->ntray--;
    g->loc[t] = 0;
    return 1;
}

static __device__ __forceinline__ void g_reveal(G *g) {
    for (int i = 0; i < g->n; i++)
        if (g->loc[i] == 0 && g->face[i] < 0 && g_free(g, i)) g->face[i] = g->truth[i];
}

static __device__ __forceinline__ void g_redeal(G *g) {
    int on[MAXN], no = 0;
    for (int i = 0; i < g->n; i++) if (g->loc[i] == 0) on[no++] = i;
    if (no < 2) return;
    signed char first = g->truth[on[0]];
    for (int w = 0; w + 1 < no; w++) g->truth[on[w]] = g->truth[on[w + 1]];
    g->truth[on[no - 1]] = first;
    for (int i = 0; i < no; i++) g->face[on[i]] = -1;
}

static __device__ __forceinline__ int g_ceil3(int c) { return ((c + 2) / 3) * 3; }

// Candidate per-face totals for the hidden pool. Mirrors the host model: every
// face's total is a multiple of three (each colour clears in triples), a face
// holds two to four triples, and at most 24 colours are in play. `idx` selects
// which candidate to materialise; the count of candidates is returned.
static __device__ __forceinline__ int g_cands(const int *cnt, int obs, const int *keys,
                                              int n, int want, signed char *tot) {
    for (int i = 0; i < 64; i++) tot[i] = 0;
    if (n % 3 != 0) return 0;
    int triples = n / 3;
    int ncand = 0;
    for (int f = obs; f <= triples; f++) {
        int base = triples / f, rem = triples % f;
        int lo = base * 3, hi = (base + 1) * 3;
        if (base < 2 || base > 3 || f > 24) continue;
        if (lo < 3) continue;
        int need = 0, ok = 1;
        for (int i = 0; i < obs; i++) {
            int mn = g_ceil3(cnt[keys[i]]);
            if (mn > hi) { ok = 0; break; }
            if (mn > lo) need++;
        }
        if (!ok || need > rem) continue;
        if (ncand == want) {
            signed char t[64];
            for (int i = 0; i < 64; i++) t[i] = 0;
            for (int i = 0; i < obs; i++) {
                int mn = g_ceil3(cnt[keys[i]]);
                int v = (i < rem) ? hi : lo;
                if (v < mn) v = mn;
                t[keys[i]] = (signed char)v;
            }
            int sum = 0;
            for (int i = 0; i < obs; i++) sum += t[keys[i]];
            int used = obs;
            for (int lbl = 0; used < f && lbl < 64; lbl++) {
                if (t[lbl] != 0) continue;
                t[lbl] = (signed char)lo;
                sum += lo;
                used++;
            }
            if (sum == n) {
                for (int i = 0; i < 64; i++) tot[i] = t[i];
                return 1;
            }
        } else {
            int sum = 0;
            for (int i = 0; i < obs; i++) {
                int mn = g_ceil3(cnt[keys[i]]);
                int v = (i < rem) ? hi : lo;
                if (v < mn) v = mn;
                sum += v;
            }
            sum += lo * (f - obs);
            if (sum == n) ncand++;
        }
    }
    return ncand;
}

static __device__ __forceinline__ int g_ncand(const int *cnt, int obs, const int *keys, int n) {
    signed char dummy[64];
    return g_cands(cnt, obs, keys, n, -1, dummy);
}

static __device__ __forceinline__ void g_sample(G *g, unsigned long long *s) {
    int hid[MAXN], nh = 0;
    int cnt[64];
    for (int i = 0; i < 64; i++) cnt[i] = 0;
    for (int i = 0; i < g->n; i++) {
        g->truth[i] = g->face[i];
        if (g->face[i] >= 0) cnt[g->face[i]]++;
        else if (g->loc[i] == 0) hid[nh++] = i;
    }
    if (nh == 0) return;
    int keys[64], obs = 0;
    for (int f = 0; f < 64; f++) if (cnt[f] > 0) keys[obs++] = f;
    for (int i = 1; i < obs; i++) {
        int k = keys[i], c = cnt[k], j = i - 1;
        while (j >= 0 && cnt[keys[j]] < c) { keys[j + 1] = keys[j]; j--; }
        keys[j + 1] = k;
    }
    int ncand = g_ncand(cnt, obs, keys, g->n);
    signed char tot[64];
    int have = 0;
    if (ncand > 0) {
        *s = *s * 6364136223846793005ULL + 1442695040888963407ULL;
        int want = (int)((*s >> 33) % (unsigned long long)ncand);
        have = g_cands(cnt, obs, keys, g->n, want, tot);
    }
    signed char pool[MAXN];
    int np = 0;
    if (have) {
        for (int f = 0; f < 64; f++) {
            int left = (int)tot[f] - cnt[f];
            for (int k = 0; k < left && np < MAXN; k++) pool[np++] = (signed char)f;
        }
    }
    if (np < nh) {
        int filler = obs > 0 ? keys[0] : 0;
        while (np < nh && np < MAXN) pool[np++] = (signed char)filler;
    }
    np = nh;
    for (int i = np - 1; i > 0; i--) {
        *s = *s * 6364136223846793005ULL + 1442695040888963407ULL;
        int j = (int)((*s >> 33) % (unsigned long long)(i + 1));
        signed char t = pool[i]; pool[i] = pool[j]; pool[j] = t;
    }
    for (int i = 0; i < nh; i++) g->truth[hid[i]] = pool[i];
}

static __device__ __forceinline__ int g_tally(const G *g, int i) {
    int c = 0;
    for (int k = 0; k < g->ntray; k++) if (g->face[g->tray[k]] == g->face[i]) c++;
    return c;
}

static __device__ __forceinline__ int g_choose(G *g, int rm, int rf, int rb, int *out) {
    int best = -1, bt = -1;
    for (int i = 0; i < g->n; i++) {
        if (g->loc[i] != 0 || g->face[i] < 0 || !g_free(g, i)) continue;
        int t = g_tally(g, i);
        if (t >= 2) { if (t > bt) { bt = t; best = i; } }
    }
    if (best >= 0) { out[0] = best; return 1; }
    bt = -1;
    for (int i = 0; i < g->n; i++) {
        if (g->loc[i] != 0 || g->face[i] < 0 || !g_free(g, i)) continue;
        if (g_tally(g, i) == 1) { if (bt < 1) { bt = 1; best = i; } }
    }
    if (best >= 0) { out[0] = best; return 1; }
    if (g->ntray + 4 <= g->slots) {
        int bk = -1;
        for (int i = 0; i < g->n; i++) {
            if (g->loc[i] != 0 || g->face[i] < 0 || !g_free(g, i)) continue;
            if (g_tally(g, i) != 0) continue;
            int k = 0;
            for (int x = 0; x < g->n; x++) if (g->loc[x] == 0 && g->face[x] == g->face[i]) k++;
            if (k > bk) { bk = k; best = i; }
        }
        if (best >= 0) { out[0] = best; return 1; }
    }
    if (g->ntray >= 3 && rm > 0) { out[0] = -1; return 1; }
    if (g->ntray >= 4 && rb > 0) { out[0] = -3; return 1; }
    if (rf > 0 && g_refresh_ok(g) && g->ntray >= 5) { out[0] = -2; return 1; }
    for (int i = 0; i < g->n; i++) {
        if (g->loc[i] == 0 && g->face[i] >= 0 && g_free(g, i)) { out[0] = i; return 1; }
    }
    (void)bt;
    return 0;
}

static __device__ __forceinline__ int g_apply(G *g, int code, int *rm, int *rf, int *rb) {
    if (code >= 0) return g_tap(g, code);
    if (code == -1) { if (*rm <= 0) return 0; if (!g_remove(g)) return 0; (*rm)--; return 1; }
    if (code == -2) {
        if (*rf <= 0 || !g_refresh_ok(g)) return 0;
        for (int i = 0; i < g->n; i++) if (g->loc[i] == 0) g->face[i] = -1;
        g_redeal(g);
        (*rf)--;
        return 1;
    }
    if (code == -3) { if (*rb <= 0) return 0; if (!g_rollback(g)) return 0; (*rb)--; return 1; }
    return 0;
}

static __device__ int g_rollout(G *g, int rm, int rf, int rb, unsigned long long *s) {
    for (int m = 0; m < MAXMOVES; m++) {
        if (g_over(g)) return 0;
        if (g_won(g)) return 1;
        int mv;
        if (!g_choose(g, rm, rf, rb, &mv)) return 0;
        if (!g_apply(g, mv, &rm, &rf, &rb)) return 0;
        g_reveal(g);
    }
    return 0;
}

#endif
