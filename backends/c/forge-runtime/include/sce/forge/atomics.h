/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge -- the intrinsics the queue runtime (sce/forge/queue.h) is built on.
 *
 * Every symbol here is one of the SCE Protocol-Synthesis RFC §synth-5-I
 * whitelist (the `sce_atomic_*` family, `sce_intrinsics_runtime`), declared
 * with the prototype the generator writes for it (forge/extern_emit.rs:
 * `*const T` is `const T*`, `*mut T` is `T*`), so a declaration here and one in
 * a generated `<sce:extern>` sidecar agree. The runtime does not implement
 * them: the compiler never sees an implementation, which is the point of the
 * family. The platform supplies one, as it does for the worker inbox
 * (templates/forge/c/worker.c.jinja2): a hosted target maps each onto
 * `__atomic_*` or `<stdatomic.h>` (the conformance tests' sce_atomic_host.c is
 * the reference), a bare-metal target onto its core's load-exclusive /
 * store-exclusive pair, or onto an interrupt-masked critical section where it
 * has no read-modify-write.
 *
 * Only the orderings the queue algorithms use are declared:
 *
 *   load    acquire, seq_cst        a published index or entry; the SCQ
 *                                   emptiness threshold (seq_cst, as in the
 *                                   Rust and C++ runtimes)
 *   store   release, seq_cst        publishing an index; the threshold
 *   cas     strong, acq_rel         returns the old value; it took the swap
 *                                   exactly when that equals `expected`
 *   fetch   add, sub, or, acq_rel   return the old value
 *   xchg    acq_rel                 returns the old value; u32 only
 *   load    relaxed                 u32 only
 *   store   relaxed                 u32 only
 *
 * for the two widths the queue uses: u32 (the Lamport row's indices, the
 * place counters, and the entries of an SCQ ring on a 32-bit target) and u64
 * (the entries of an SCQ ring on a 64-bit one). The exchange and the relaxed
 * pair belong to the intrusive queue (Vyukov), whose links are u32 node
 * indices: a producer exchanges its node onto the tail, and a link is read
 * and written relaxed because the exchange and the acquire load order it.
 */

#ifndef SCE_FORGE_ATOMICS_H
#define SCE_FORGE_ATOMICS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

extern uint32_t sce_atomic_load_acquire_u32(const uint32_t *p0);
extern uint32_t sce_atomic_load_relaxed_u32(const uint32_t *p0);
extern void sce_atomic_store_relaxed_u32(uint32_t *p0, uint32_t p1);
extern uint32_t sce_atomic_xchg_acq_rel_u32(uint32_t *p0, uint32_t p1);
extern uint32_t sce_atomic_load_seq_cst_u32(const uint32_t *p0);
extern void sce_atomic_store_release_u32(uint32_t *p0, uint32_t p1);
extern void sce_atomic_store_seq_cst_u32(uint32_t *p0, uint32_t p1);
extern uint32_t sce_atomic_cas_strong_acq_rel_u32(uint32_t *p0, uint32_t p1, uint32_t p2);
extern uint32_t sce_atomic_fetch_add_acq_rel_u32(uint32_t *p0, uint32_t p1);
extern uint32_t sce_atomic_fetch_sub_acq_rel_u32(uint32_t *p0, uint32_t p1);
extern uint32_t sce_atomic_fetch_or_acq_rel_u32(uint32_t *p0, uint32_t p1);

extern uint64_t sce_atomic_load_acquire_u64(const uint64_t *p0);
extern uint64_t sce_atomic_load_seq_cst_u64(const uint64_t *p0);
extern void sce_atomic_store_release_u64(uint64_t *p0, uint64_t p1);
extern void sce_atomic_store_seq_cst_u64(uint64_t *p0, uint64_t p1);
extern uint64_t sce_atomic_cas_strong_acq_rel_u64(uint64_t *p0, uint64_t p1, uint64_t p2);
extern uint64_t sce_atomic_fetch_add_acq_rel_u64(uint64_t *p0, uint64_t p1);
extern uint64_t sce_atomic_fetch_sub_acq_rel_u64(uint64_t *p0, uint64_t p1);
extern uint64_t sce_atomic_fetch_or_acq_rel_u64(uint64_t *p0, uint64_t p1);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_FORGE_ATOMICS_H */
