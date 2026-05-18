/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * RFC variant-default-uniformity Atomic γ-3 c11 half — runtime
 * round-trip property test. Mirrors
 * sce-forge-runtime/rust/tests/forge_default_round_trip.rs for the
 * C11 backend: includes the generated codec headers and exercises
 * the `<UPPER>_DEFAULT_INIT` designated-initializer macro contract
 * end-to-end.
 *
 * Critical invariants verified:
 *   1. `T x = T_DEFAULT_INIT; encode(&x)` produces a 3-byte frame
 *      (arm B's 1-byte header + 2-byte uint16 payload).
 *   2. Frame[0] low 2 bits encode arm B's MID (0x02).
 *   3. Decoding consumes every emitted byte (cursor remaining == 0).
 *   4. Decoded variant's `kind` enum names arm B.
 *   5. Re-encoding produces byte-equal output.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_default_marker_arm_a.h"
#include "codec_default_marker_arm_b.h"
#include "codec_variant_default_marker.h"

static int failures = 0;

#define EXPECT(cond, msg)                                                    \
    do {                                                                     \
        if (!(cond)) {                                                       \
            ++failures;                                                      \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, msg);     \
        }                                                                    \
    } while (0)

int main(void) {
    /* `T_DEFAULT_INIT` composes arm B's own _DEFAULT_INIT macro into
     * the union slot so the resulting struct's header byte is
     * pre-set to 0x02. Bare `{0}` would leave kind=0 (= arm A
     * enum slot 0) AND the union slot zero-filled — a round-trip
     * landmine the β-c11 emission contract fixes. */
    codec_variant_default_marker_t original = CODEC_VARIANT_DEFAULT_MARKER_DEFAULT_INIT;
    codec_variant_default_marker_encoded_t encoded =
        codec_variant_default_marker_encode(&original);

    EXPECT(encoded.len == 3,
           "default-emit + arm B (uint16 payload) must produce 3 wire bytes");
    EXPECT((encoded.bytes[0] & 0x03) == 0x02,
           "first byte low 2 bits must encode arm B's MID (0x02) — "
           "if 0 the inner _DEFAULT_INIT macro didn't bake the header byte; "
           "β-c11 emission contract is broken");

    sce_forge_cursor_t cursor =
        sce_forge_cursor_init(encoded.bytes, encoded.len);
    codec_variant_default_marker_t decoded;
    memset(&decoded, 0, sizeof(decoded));
    sce_forge_codec_status_t st =
        codec_variant_default_marker_decode(&cursor, &decoded);
    EXPECT(st == SCE_FORGE_CODEC_OK,
           "freshly-constructed codec must decode without error");
    EXPECT(sce_forge_cursor_remaining(&cursor) == 0,
           "decode must consume every emitted byte; leftover means an "
           "arm-type mismatch on dispatch");

    EXPECT(decoded.body.kind == CODEC_VARIANT_DEFAULT_MARKER_BODY_KIND_CODEC_DEFAULT_MARKER_ARM_B,
           "round-trip must land in arm B (the marked-default arm) — "
           "kind == ARM_A would mean the legacy first-arm convention took "
           "effect; kind == DEFAULT would mean the catch-all fallback fired");

    codec_variant_default_marker_encoded_t re_encoded =
        codec_variant_default_marker_encode(&decoded);
    EXPECT(re_encoded.len == encoded.len,
           "re-encode length must match original");
    EXPECT(memcmp(re_encoded.bytes, encoded.bytes, encoded.len) == 0,
           "re-encoded bytes must match original byte-for-byte");

    if (failures == 0) {
        puts("OK default_round_trip_c");
        return EXIT_SUCCESS;
    }
    fprintf(stderr, "%d assertion(s) failed\n", failures);
    return EXIT_FAILURE;
}
