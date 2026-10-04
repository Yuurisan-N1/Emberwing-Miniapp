#include <cuda_runtime.h>
#include <stdio.h>

#include "forge_ev.cuh"

__global__ void ev_kernel(const G *base, const int *cand, int ncand, int depth,
                          long long budget, int *out) {
    int ci = blockIdx.x * blockDim.x + threadIdx.x;
    if (ci >= ncand) return;
    G g = base[ci];
    signed char pool[MAXN];
    int np = d_pool(&g, pool);
    if (np < 0) { out[ci * 2] = 0; out[ci * 2 + 1] = 0; return; }
    int code = cand[ci];
    int rm = 0, rf = 0, rb = 0;
    if (code >= 0) {
        if (!g_tap(&g, code)) { out[ci * 2] = 0; out[ci * 2 + 1] = 0; return; }
        g_reveal(&g);
    } else if (code == -1) {
        rm = 1;
        if (!g_apply(&g, -1, &rm, &rf, &rb)) { out[ci * 2] = 0; out[ci * 2 + 1] = 0; return; }
        rm = 0;
    } else if (code == -2) {
        rf = 1;
        if (!g_apply(&g, -2, &rm, &rf, &rb)) { out[ci * 2] = 0; out[ci * 2 + 1] = 0; return; }
        rf = 0;
        g_reveal(&g);
    } else if (code == -3) {
        rb = 1;
        if (!g_apply(&g, -3, &rm, &rf, &rb)) { out[ci * 2] = 0; out[ci * 2 + 1] = 0; return; }
        rb = 0;
    }
    if (g_over(&g)) { out[ci * 2] = 0; out[ci * 2 + 1] = 0; return; }
    long long b = budget;
    int ok = d_force(&g, rm, rf, rb, pool, np, depth, &b);
    out[ci * 2] = ok;
    out[ci * 2 + 1] = (int)(budget - b);
}

extern "C" int forge_ev_prove(const void *boards, const int *cand, int ncand, int depth,
                             long long budget, int *out) {
    int devs = 0;
    if (cudaGetDeviceCount(&devs) != cudaSuccess || devs <= 0) return -1;
    if (cudaSetDevice(0) != cudaSuccess) return -2;
    if (ncand <= 0 || depth <= 0 || budget <= 0) return -5;
    G *db = NULL;
    int *dc = NULL, *do_ = NULL;
    size_t bsz = (size_t)ncand * sizeof(G);
    if (cudaMalloc(&db, bsz) != cudaSuccess) return -3;
    if (cudaMalloc(&dc, ncand * sizeof(int)) != cudaSuccess) { cudaFree(db); return -3; }
    if (cudaMalloc(&do_, ncand * 2 * sizeof(int)) != cudaSuccess) {
        cudaFree(db);
        cudaFree(dc);
        return -3;
    }
    cudaMemcpy(db, boards, bsz, cudaMemcpyHostToDevice);
    cudaMemcpy(dc, cand, ncand * sizeof(int), cudaMemcpyHostToDevice);
    cudaMemset(do_, 0, ncand * 2 * sizeof(int));
    int block = 64;
    int grid = (ncand + block - 1) / block;
    ev_kernel<<<grid, block>>>(db, dc, ncand, depth, budget, do_);
    cudaError_t e = cudaDeviceSynchronize();
    if (e != cudaSuccess) {
        cudaFree(db);
        cudaFree(dc);
        cudaFree(do_);
        return -4;
    }
    cudaMemcpy(out, do_, ncand * 2 * sizeof(int), cudaMemcpyDeviceToHost);
    cudaFree(db);
    cudaFree(dc);
    cudaFree(do_);
    return 0;
}
