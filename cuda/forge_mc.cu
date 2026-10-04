#include <cuda_runtime.h>
#include <stdio.h>

#include "forge_core.cuh"

__global__ void mc_kernel(const G *base, const int *cand, int ncand, int samples,
                          unsigned long long seed, int *out) {
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)ncand * samples;
    if (idx >= total) return;
    int ci = (int)(idx / samples);
    G g = base[ci];
    unsigned long long s = seed ^ ((unsigned long long)idx * 0x9E3779B97F4A7C15ULL + 0x2545F4914F6CDD1DULL);
    g_sample(&g, &s);
    g_reveal(&g);
    int rm = 0, rf = 0, rb = 0;
    int code = cand[ci];
    if (code >= 0) {
        if (!g_tap(&g, code)) return;
        g_reveal(&g);
    } else if (code == -1) {
        rm = 1;
        if (!g_apply(&g, -1, &rm, &rf, &rb)) return;
        rm = 0;
    } else if (code == -2) {
        rf = 1;
        if (!g_apply(&g, -2, &rm, &rf, &rb)) return;
        rf = 0;
        g_reveal(&g);
    } else if (code == -3) {
        rb = 1;
        if (!g_apply(&g, -3, &rm, &rf, &rb)) return;
        rb = 0;
    }
    if (g_rollout(&g, rm, rf, rb, &s)) atomicAdd(&out[ci], 1);
}

extern "C" int forge_mc_devices(void) {
    int n = 0;
    if (cudaGetDeviceCount(&n) != cudaSuccess) return -1;
    return n;
}

extern "C" int forge_mc_eval(const void *boards, const int *cand, int ncand, int samples,
                             unsigned long long seed, int *out_wins) {
    int devs = 0;
    if (cudaGetDeviceCount(&devs) != cudaSuccess || devs <= 0) return -1;
    if (cudaSetDevice(0) != cudaSuccess) return -2;
    G *db = NULL;
    int *dc = NULL, *do_ = NULL;
    size_t bsz = (size_t)ncand * sizeof(G);
    if (cudaMalloc(&db, bsz) != cudaSuccess) return -3;
    if (cudaMalloc(&dc, ncand * sizeof(int)) != cudaSuccess) { cudaFree(db); return -3; }
    if (cudaMalloc(&do_, ncand * sizeof(int)) != cudaSuccess) { cudaFree(db); cudaFree(dc); return -3; }
    cudaMemcpy(db, boards, bsz, cudaMemcpyHostToDevice);
    cudaMemcpy(dc, cand, ncand * sizeof(int), cudaMemcpyHostToDevice);
    cudaMemset(do_, 0, ncand * sizeof(int));
    long total = (long)ncand * samples;
    int block = 128;
    int grid = (int)((total + block - 1) / block);
    mc_kernel<<<grid, block>>>(db, dc, ncand, samples, seed, do_);
    cudaError_t e = cudaDeviceSynchronize();
    if (e != cudaSuccess) {
        cudaFree(db); cudaFree(dc); cudaFree(do_);
        return -4;
    }
    cudaMemcpy(out_wins, do_, ncand * sizeof(int), cudaMemcpyDeviceToHost);
    cudaFree(db); cudaFree(dc); cudaFree(do_);
    return 0;
}

extern "C" int forge_mc_pack(void *blob, int slot, int n, int slots, int stash,
                             const signed char *loc, const signed char *face,
                             const unsigned long long *up,
                             const signed char *tray, int ntray,
                             const signed char *side, int nside) {
    G *g = (G *)blob + slot;
    if (n > MAXN) return -1;
    g->n = n;
    g->slots = slots;
    g->stash = stash;
    for (int i = 0; i < MAXN; i++) { g->loc[i] = 3; g->face[i] = -1; g->up[i] = 0; }
    for (int i = 0; i < n; i++) { g->loc[i] = loc[i]; g->face[i] = face[i]; g->up[i] = up[i]; }
    g->ntray = ntray > MAXSLOT ? MAXSLOT : ntray;
    for (int i = 0; i < g->ntray; i++) g->tray[i] = tray[i];
    g->nside = nside > MAXSLOT ? MAXSLOT : nside;
    for (int i = 0; i < g->nside; i++) g->side[i] = side[i];
    return 0;
}

extern "C" int forge_mc_size(void) { return (int)sizeof(G); }
