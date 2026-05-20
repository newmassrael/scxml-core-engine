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
 * RFC §5.B B1-α: encode is writer-based — the test exercises both
 * the heap-free `_encode_to_buf` convenience facade and the primary
 * `_encode(self, w)` over a caller-owned `sce_forge_writer_t`. A
 * deliberately-undersized `SpanSink`-equivalent buffer surfaces the
 * typed `SCE_FORGE_CODEC_BUFFER_OVERFLOW` status.
 *
 * Critical invariants verified:
 *   1. `T x = T_DEFAULT_INIT; encode_to_buf(&x, …)` produces a 3-byte
 *      frame (arm B's 1-byte header + 2-byte uint16 payload).
 *   2. Frame[0] low 2 bits encode arm B's MID (0x02).
 *   3. Decoding consumes every emitted byte (cursor remaining == 0).
 *   4. Decoded variant's `kind` enum names arm B.
 *   5. Re-encoding produces byte-equal output.
 *   6. Writer-direct encode (over a stack buffer) yields the same
 *      bytes as the facade.
 *   7. Undersized buffer surfaces SCE_FORGE_CODEC_BUFFER_OVERFLOW.
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

    uint8_t encoded_bytes[CODEC_VARIANT_DEFAULT_MARKER_MAX_BYTES];
    size_t encoded_len = 0;
    sce_forge_codec_status_t enc_st =
        codec_variant_default_marker_encode_to_buf(
            &original, encoded_bytes, sizeof(encoded_bytes), &encoded_len);
    EXPECT(enc_st == SCE_FORGE_CODEC_OK,
           "encode_to_buf must succeed when buffer >= MAX_BYTES");
    EXPECT(encoded_len == 3,
           "default-emit + arm B (uint16 payload) must produce 3 wire bytes");
    EXPECT((encoded_bytes[0] & 0x03) == 0x02,
           "first byte low 2 bits must encode arm B's MID (0x02) — "
           "if 0 the inner _DEFAULT_INIT macro didn't bake the header byte; "
           "β-c11 emission contract is broken");

    sce_forge_cursor_t cursor =
        sce_forge_cursor_init(encoded_bytes, encoded_len);
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

    uint8_t re_encoded_bytes[CODEC_VARIANT_DEFAULT_MARKER_MAX_BYTES];
    size_t re_encoded_len = 0;
    sce_forge_codec_status_t re_st =
        codec_variant_default_marker_encode_to_buf(
            &decoded, re_encoded_bytes, sizeof(re_encoded_bytes), &re_encoded_len);
    EXPECT(re_st == SCE_FORGE_CODEC_OK,
           "re-encode_to_buf must succeed");
    EXPECT(re_encoded_len == encoded_len,
           "re-encode length must match original");
    EXPECT(memcmp(re_encoded_bytes, encoded_bytes, encoded_len) == 0,
           "re-encoded bytes must match original byte-for-byte");

    /* RFC §5.B B1-α writer-direct path: same bytes via a caller-owned
     * writer over a stack buffer. */
    {
        uint8_t direct_bytes[CODEC_VARIANT_DEFAULT_MARKER_MAX_BYTES];
        sce_forge_writer_t w = sce_forge_writer_init_buf(direct_bytes, sizeof(direct_bytes));
        sce_forge_codec_status_t direct_st =
            codec_variant_default_marker_encode(&decoded, &w);
        EXPECT(direct_st == SCE_FORGE_CODEC_OK,
               "writer-direct encode must succeed when cap >= MAX_BYTES");
        EXPECT(sce_forge_writer_position(&w) == encoded_len,
               "writer position must equal facade encoded length");
        EXPECT(memcmp(direct_bytes, encoded_bytes, encoded_len) == 0,
               "writer-direct bytes must equal facade output");
    }

    /* Bounded-buffer BufferOverflow path: a writer sized strictly
     * smaller than the actual wire length must surface the typed
     * error (validates the encode-side typed error contract). */
    if (encoded_len > 0) {
        uint8_t tiny[2] = {0};  /* less than encoded_len=3 */
        sce_forge_writer_t w_tiny = sce_forge_writer_init_buf(tiny, sizeof(tiny));
        sce_forge_codec_status_t over_st =
            codec_variant_default_marker_encode(&decoded, &w_tiny);
        EXPECT(over_st == SCE_FORGE_CODEC_BUFFER_OVERFLOW,
               "writer encode must surface BUFFER_OVERFLOW when cap < bytes");
    }

    if (failures == 0) {
        puts("OK default_round_trip_c");
        return EXIT_SUCCESS;
    }
    fprintf(stderr, "%d assertion(s) failed\n", failures);
    return EXIT_FAILURE;
}
