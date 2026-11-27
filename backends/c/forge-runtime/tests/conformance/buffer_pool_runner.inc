/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The scenario runner of buffer_pool_test.c, written once and included once per
 * pool. The includer defines POOL_PREFIX (the pool's snake_case name and an
 * underscore) and POOL_UPPER (the same in upper case), and POOL_CAT; F(name)
 * names one of the pool's functions or types and U(name) one of its macros.
 */

#define F(name) POOL_CAT(POOL_PREFIX, name)
#define U(name) POOL_CAT(POOL_UPPER, name)

/* A lending's outcome as the contract spells it: the handle's, or why the
 * pool refused. */
static long F(lend_outcome)(sce_slot_handle_t handle, size_t len, size_t headroom) {
    if (handle.state != SCE_SLOT_INVALID) {
        return LEND_OK;
    }
    switch (F(lend_refusal)(len, headroom)) {
    case U(LEND_LENGTH):
        return LEND_LENGTH;
    case U(LEND_HEADROOM):
        return LEND_HEADROOM;
    case U(LEND_FITS):
    default:
        return LEND_EXHAUSTED;
    }
}

static void F(run)(const pool_scenario_t *scenario) {
    sce_slot_handle_t held[8];
    F(slot_index_t) indices[8];
    F(span_t) sent;
    bool made;
    const char *id = scenario->id;

    for (size_t r = 0; r < 8u; ++r) {
        held[r].state = SCE_SLOT_INVALID;
        held[r].idx = SIZE_MAX;
        indices[r] = F(no_index)();
    }
    sent.ptr = NULL;
    sent.len = 0u;

    if (scenario->over) {
        made = F(init_over)(arena + scenario->offset, scenario->span, scenario->count, scenario->len);
    } else {
        made = F(init)(scenario->count);
    }
    CHECK(made == (scenario->make_ok != 0), id, 0u);
    if (!made) {
        return;
    }

    for (size_t n = 0; n < scenario->step_count; ++n) {
        const pool_step_t *step = &scenario->steps[n];
        switch (step->op) {
        case OP_COUNT:
            CHECK((long)F(count)() == step->expect, id, n);
            break;
        case OP_SLOT_LEN:
            CHECK((long)F(slot_len)() == step->expect, id, n);
            break;
        case OP_FREE:
            CHECK((long)F(free_count)() == step->expect, id, n);
            break;
        case OP_VIOLATIONS:
            CHECK((long)F(ownership_violations)() == step->expect, id, n);
            break;
        case OP_REFUSED_PREFIXES:
            CHECK((long)F(refused_prefixes)() == step->expect, id, n);
            break;
        case OP_ACQUIRE: {
            sce_slot_handle_t h = F(pool_acquire_for_encode)((size_t)step->a, (size_t)step->b);
            CHECK(F(lend_outcome)(h, (size_t)step->a, (size_t)step->b) == step->expect, id, n);
            if (h.state != SCE_SLOT_INVALID) {
                held[step->out] = h;
            }
            break;
        }
        case OP_RECEIVE: {
            sce_slot_handle_t armed = F(link_arm_rx)((size_t)step->a);
            CHECK(F(lend_outcome)(armed, (size_t)step->a, 0u) == step->expect, id, n);
            if (armed.state != SCE_SLOT_INVALID) {
                size_t position = armed.idx;
                uint8_t *at = F(dma_armed_rx_ptr)(&armed);
                CHECK(at != NULL, id, n);
                /* Standing in for the peripheral writing the slot it was
                   pointed at, and for its completion IRQ. */
                for (size_t i = 0; at != NULL && i < step->nbytes; ++i) {
                    at[i] = step->bytes[i];
                }
                CHECK(F(dma_start_rx)(&armed), id, n);
                held[step->out] = F(rx_complete)(position);
                CHECK(held[step->out].state == SCE_SLOT_CPU_REF, id, n);
            }
            break;
        }
        case OP_CAPACITY:
            CHECK((long)F(slot_capacity)(&held[step->slot]) == step->expect, id, n);
            break;
        case OP_WRITE: {
            uint8_t *w = F(slot_write)(&held[step->slot]);
            CHECK(w != NULL, id, n);
            for (size_t i = 0; w != NULL && i < step->nbytes; ++i) {
                w[i] = step->bytes[i];
            }
            break;
        }
        case OP_READ: {
            const uint8_t *r = F(slot_read)(&held[step->slot]);
            CHECK(r != NULL, id, n);
            for (size_t i = 0; r != NULL && i < step->nbytes; ++i) {
                CHECK(r[i] == step->bytes[i], id, n);
            }
            break;
        }
        case OP_ARM_TX: {
            bool armed = F(link_arm_tx)(&held[step->slot], (size_t)step->a);
            CHECK((armed ? OUTCOME_OK : OUTCOME_REFUSED) == step->expect, id, n);
            if (armed) {
                indices[step->out] = F(slot_index)(&held[step->slot]);
            }
            break;
        }
        case OP_RETURN:
            CHECK((F(pool_return)(&held[step->slot]) ? OUTCOME_OK : OUTCOME_REFUSED) == step->expect, id, n);
            break;
        case OP_INDEX:
            indices[step->out] = F(slot_index)(&held[step->slot]);
            CHECK(indices[step->out].slot != UINT32_MAX, id, n);
            break;
        case OP_INTO_INDEX:
            indices[step->out] = F(into_index)(&held[step->slot]);
            CHECK(indices[step->out].slot != UINT32_MAX, id, n);
            break;
        case OP_RECLAIM: {
            sce_slot_handle_t h = F(reclaim)(indices[step->index], (sce_slot_state_t)step->a);
            CHECK((h.state != SCE_SLOT_INVALID ? OUTCOME_OK : OUTCOME_REFUSED) == step->expect, id, n);
            if (h.state != SCE_SLOT_INVALID) {
                held[step->out] = h;
            }
            break;
        }
        case OP_GENERATION:
            CHECK((long)indices[step->index].generation == step->expect, id, n);
            break;
        case OP_TAKE: {
            const uint32_t violations = F(ownership_violations)();
            const uint32_t refused = F(refused_prefixes)();
            F(span_t) span = F(take_prefixed)(indices[step->index], step->bytes, step->nbytes);
            long outcome = TAKE_OK;
            if (span.ptr == NULL) {
                outcome = F(refused_prefixes)() != refused ? TAKE_PREFIX_TOO_WIDE : TAKE_STALE;
                CHECK(outcome == TAKE_PREFIX_TOO_WIDE || F(ownership_violations)() != violations, id, n);
            }
            CHECK(outcome == step->expect, id, n);
            if (outcome == TAKE_OK) {
                CHECK((long)span.len == step->a, id, n);
                sent = span;
            }
            break;
        }
        case OP_SENT:
            CHECK(sent.ptr != NULL && sent.len == step->nbytes, id, n);
            for (size_t i = 0; sent.ptr != NULL && i < step->nbytes && i < sent.len; ++i) {
                CHECK(sent.ptr[i] == step->bytes[i], id, n);
            }
            break;
        case OP_COMPLETE_TX:
            CHECK((F(tx_complete)(indices[step->index].slot) ? 1L : 0L) == step->expect, id, n);
            break;
        case OP_LEND: {
            F(slot_index_t) lent = F(lend)(&held[step->slot], (uint32_t)step->a);
            CHECK((lent.slot != UINT32_MAX ? OUTCOME_OK : OUTCOME_REFUSED) == step->expect, id, n);
            if (lent.slot != UINT32_MAX) {
                indices[step->out] = lent;
            }
            break;
        }
        case OP_HOLD:
            CHECK((F(hold)(indices[step->index]) ? 1L : 0L) == step->expect, id, n);
            break;
        case OP_RELEASE:
            CHECK((F(release)(indices[step->index]) ? 1L : 0L) == step->expect, id, n);
            break;
        case OP_LENT: {
            F(span_t) span = F(lent)(indices[step->index]);
            CHECK((span.ptr == NULL ? -1L : (long)span.len) == step->expect, id, n);
            break;
        }
        default:
            CHECK(false && "unknown op", id, n);
            break;
        }
    }
}

#undef F
#undef U
