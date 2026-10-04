#ifndef EMBERWING_FORGE_H
#define EMBERWING_FORGE_H

#ifdef __cplusplus
extern "C" {
#endif

#define FORGE_MOVE_TAP 0
#define FORGE_MOVE_REMOVE 1
#define FORGE_MOVE_REFRESH 2
#define FORGE_MOVE_ROLLBACK 3

#define FORGE_OK 0
#define FORGE_ERR_ARG 1
#define FORGE_ERR_NOMOVE 2

typedef struct {
    const int *geo;
    int n;
    int slots;
    int stash;
    const unsigned long long *on;
    int on_words;
    const signed char *face;
    const int *tray;
    int tray_n;
    const int *side;
    int side_n;
    int rm;
    int rf;
    int rb;
} ForgeState;

typedef struct {
    int kind;
    int tile;
} ForgeMove;

typedef struct {
    int samples;
    int worlds;
    int threads;
    int deadline_ms;
    long long nodes;
    unsigned long long seed;
    int exact;
} ForgeBudget;

int forge_version(void);
const char *forge_desc(void);

int forge_plan(const ForgeState *st, const ForgeBudget *bud, ForgeMove *out, int out_cap);
int forge_win(const ForgeState *st, const ForgeBudget *bud, int depth);
int forge_rollouts(const ForgeState *st, const ForgeBudget *bud, int tile, int *wins, int *plays);
void forge_free_moves(ForgeMove *p);

#ifdef __cplusplus
}
#endif

#endif
