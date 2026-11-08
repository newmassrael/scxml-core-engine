// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A counting allocator for the C tests (docs/adr/0086, section 8.6 (d)), of the
// shape of `sce_alloc_fn` and of Lua's `lua_Alloc`: it counts every block it
// gives and every block it frees, keeps the bytes it has outstanding, and can be
// told to refuse the next block, so a test asserts that each text a holder owns
// is freed exactly once and that a refused block reaches the reason it raises.
//
// A block freed twice, or freed with a size other than the one it was given
// at, is counted in `misfreed`: the allocator keeps the blocks it has
// outstanding, up to COUNTING_ALLOC_HELD at once, so it can tell. Every block
// comes from the C library and goes back to it.

#ifndef SCE_TESTS_COUNTING_ALLOC_H
#define SCE_TESTS_COUNTING_ALLOC_H

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* The most blocks a case holds at once. */
#define COUNTING_ALLOC_HELD 64

typedef struct {
    void *ptr;
    size_t size;
} counting_alloc_block_t;

typedef struct {
    uint32_t given;
    uint32_t freed;
    /* Frees and resizes of a block this allocator does not hold, or at
       another size than it holds the block at. */
    uint32_t misfreed;
    /* Blocks it refused, and whether it refuses the next one. */
    uint32_t refused;
    int refuse_next;
    size_t outstanding;
    counting_alloc_block_t held[COUNTING_ALLOC_HELD];
} counting_alloc_t;

static inline void counting_alloc_init(counting_alloc_t *self) {
    memset(self, 0, sizeof(*self));
}

/* The index `ptr` is held at, or -1. */
static inline int counting_alloc_find(const counting_alloc_t *self, const void *ptr) {
    for (int i = 0; i < COUNTING_ALLOC_HELD; i++) {
        if (self->held[i].ptr == ptr) {
            return i;
        }
    }
    return -1;
}

/* The allocator: `context` is a `counting_alloc_t`. */
static inline void *counting_alloc(void *context, void *ptr, size_t old_size, size_t new_size) {
    counting_alloc_t *self = (counting_alloc_t *)context;
    const int at = ptr != NULL ? counting_alloc_find(self, ptr) : counting_alloc_find(self, NULL);
    if (ptr != NULL && (at < 0 || self->held[at].size != old_size)) {
        self->misfreed++;
    }
    if (new_size == 0u) {
        if (ptr != NULL) {
            self->freed++;
            /* A block this allocator does not hold is not handed to the C
               library, so a second free is counted rather than crashing. */
            if (at >= 0) {
                self->outstanding -= self->held[at].size;
                self->held[at].ptr = NULL;
                self->held[at].size = 0u;
                free(ptr);
            }
        }
        return NULL;
    }
    /* A new block needs a place to be held at, and a resize of a block this
       allocator does not hold is not made. */
    if (self->refuse_next || at < 0) {
        self->refuse_next = 0;
        self->refused++;
        return NULL;
    }
    void *block = realloc(ptr, new_size);
    if (block == NULL) {
        return NULL;
    }
    if (ptr == NULL) {
        self->given++;
    } else {
        self->outstanding -= self->held[at].size;
    }
    self->held[at].ptr = block;
    self->held[at].size = new_size;
    self->outstanding += new_size;
    return block;
}

#endif /* SCE_TESTS_COUNTING_ALLOC_H */
