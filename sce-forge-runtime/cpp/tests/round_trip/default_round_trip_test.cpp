// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// RFC variant-default-uniformity Atomic γ-3 cpp half — runtime round-trip
// property test. Mirrors sce-forge-runtime/rust/tests/forge_default_round_trip.rs
// for the C++ backend: compiles the generated codecs into the test binary
// and runs `T{}.encode().decode()` to prove the watching-zenoh R87 defect
// cannot recur on the cpp branch either.
//
// Critical invariants verified:
//   1. Encoding a freshly default-constructed outer codec produces 3 wire
//      bytes (1-byte arm-B header + 2-byte uint16 payload).
//   2. The first byte's low 2 bits encode arm B's MID (0x02), so the
//      decoder's dispatch table routes back to arm B.
//   3. Decoding consumes every emitted byte (cursor.remaining() == 0).
//   4. The decoded variant's index() == 1 (arm B's std::variant slot).
//   5. Re-encoding the decoded value produces byte-equal output.

#include <cassert>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <vector>

#include "codec_default_marker_arm_a.h"
#include "codec_default_marker_arm_b.h"
#include "codec_variant_default_marker.h"

using ::SCE::Forge::SceCursor;
using ::SCE::Generated::CodecVariantDefaultMarker::CodecVariantDefaultMarker;

static int failures = 0;

#define EXPECT(cond, msg)                                                    \
    do {                                                                     \
        if (!(cond)) {                                                       \
            ++failures;                                                      \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__,      \
                         msg);                                               \
        }                                                                    \
    } while (0)

int main() {
    const CodecVariantDefaultMarker original{};
    const auto bytes = original.encode();

    EXPECT(bytes.size() == 3,
           "default-emit + arm B (uint16 payload) must produce 3 wire bytes");
    EXPECT((bytes[0] & 0x03) == 0x02,
           "first byte low 2 bits must encode arm B's MID (0x02) — "
           "if 0 the inner Default zero-filled the header byte; "
           "β-cpp emission contract is broken");

    SceCursor cursor(bytes.data(), bytes.size());
    const auto decoded = CodecVariantDefaultMarker::decode(cursor);
    EXPECT(decoded.has_value(),
           "freshly-constructed codec must decode without error");
    EXPECT(cursor.remaining() == 0,
           "decode must consume every emitted byte; leftover means an "
           "arm-type mismatch on dispatch");

    if (decoded.has_value()) {
        // std::variant indices: 0 = ArmA, 1 = ArmB, 2 = Default catch-all.
        EXPECT(decoded->body.index() == 1,
               "round-trip must land in arm B (the marked-default arm); "
               "index 0 = legacy first-arm picked; index 2 = catch-all "
               "fallback (inner Default zero-filled the dispatch byte)");

        const auto re_encoded = decoded->encode();
        EXPECT(re_encoded == bytes,
               "re-encoding the decoded value must produce byte-equal output");
    }

    if (failures == 0) {
        std::puts("OK default_round_trip");
        return EXIT_SUCCESS;
    }
    std::fprintf(stderr, "%d assertion(s) failed\n", failures);
    return EXIT_FAILURE;
}
