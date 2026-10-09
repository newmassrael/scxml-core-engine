/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The reference implementation of the `sce_atomic_*` symbols the queue runtime
 * uses (sce/forge/atomics.h; SCE Protocol-Synthesis RFC §synth-5-I), for a hosted
 * target: each onto the compiler's `__atomic` builtins with the ordering its name
 * states. A bare-metal target supplies its own (`sce_intrinsics_runtime`); this
 * one is what the queue's tests, and ThreadSanitizer, run against.
 *
 * The builtins and not <stdatomic.h>: the queue's words are plain `uint32_t` and
 * `uint64_t` objects, which the builtins operate on legitimately and an
 * `_Atomic`-qualified cast would not.
 */

#include <stdint.h>

#include <sce/forge/atomics.h>

#define SCE_HOST_ATOMICS(WIDTH, TYPE)                                                                                  \
    TYPE sce_atomic_load_acquire_##WIDTH(const TYPE *p0) {                                                             \
        return __atomic_load_n(p0, __ATOMIC_ACQUIRE);                                                                  \
    }                                                                                                                  \
    TYPE sce_atomic_load_seq_cst_##WIDTH(const TYPE *p0) {                                                             \
        return __atomic_load_n(p0, __ATOMIC_SEQ_CST);                                                                  \
    }                                                                                                                  \
    void sce_atomic_store_release_##WIDTH(TYPE *p0, TYPE p1) {                                                         \
        __atomic_store_n(p0, p1, __ATOMIC_RELEASE);                                                                    \
    }                                                                                                                  \
    void sce_atomic_store_seq_cst_##WIDTH(TYPE *p0, TYPE p1) {                                                         \
        __atomic_store_n(p0, p1, __ATOMIC_SEQ_CST);                                                                    \
    }                                                                                                                  \
    TYPE sce_atomic_cas_strong_acq_rel_##WIDTH(TYPE *p0, TYPE p1, TYPE p2) {                                           \
        TYPE expected = p1;                                                                                            \
        (void)__atomic_compare_exchange_n(p0, &expected, p2, 0, __ATOMIC_ACQ_REL, __ATOMIC_ACQUIRE);                   \
        return expected;                                                                                               \
    }                                                                                                                  \
    TYPE sce_atomic_fetch_add_acq_rel_##WIDTH(TYPE *p0, TYPE p1) {                                                     \
        return __atomic_fetch_add(p0, p1, __ATOMIC_ACQ_REL);                                                           \
    }                                                                                                                  \
    TYPE sce_atomic_fetch_sub_acq_rel_##WIDTH(TYPE *p0, TYPE p1) {                                                     \
        return __atomic_fetch_sub(p0, p1, __ATOMIC_ACQ_REL);                                                           \
    }                                                                                                                  \
    TYPE sce_atomic_fetch_or_acq_rel_##WIDTH(TYPE *p0, TYPE p1) {                                                      \
        return __atomic_fetch_or(p0, p1, __ATOMIC_ACQ_REL);                                                            \
    }

SCE_HOST_ATOMICS(u32, uint32_t)
SCE_HOST_ATOMICS(u64, uint64_t)
