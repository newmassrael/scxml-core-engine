// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// RFC variant-default-uniformity Atomic γ-3 kotlin half — runtime
// round-trip property test. Mirrors
// sce-forge-runtime/rust/tests/forge_default_round_trip.rs for the
// Kotlin backend: imports the generated codec classes (wired into the
// jvmTest source set via the `generateRoundTripFixtures` Gradle task)
// and asserts that a freshly-constructed instance round-trips through
// encode → decode into the declared default arm.

package com.sce.forge.runtime

import com.sce.generated.codec_variant_default_marker.CodecVariantDefaultMarker
import com.sce.generated.codec_variant_default_marker.CodecVariantDefaultMarkerVariant
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class DefaultRoundTripTest {

    /**
     * Critical invariants verified:
     *  1. `CodecVariantDefaultMarker().encode()` produces 3 wire bytes
     *     (arm B's 1-byte header + 2-byte uint16 payload).
     *  2. The first byte's low 2 bits encode arm B's MID (0x02).
     *  3. Decoding consumes every emitted byte (cursor remaining == 0).
     *  4. The decoded body is the marked-default arm
     *     (`CodecVariantDefaultMarkerVariant.CodecDefaultMarkerArmB`),
     *     not the first-declared arm A.
     *  5. Re-encoding produces byte-equal output.
     */
    @Test
    fun roundTripLandsInDeclaredDefaultArm() {
        val original = CodecVariantDefaultMarker()
        val wire = original.encode()

        assertEquals(
            3,
            wire.size,
            "default-emit + arm B (uint16 payload) must produce 3 wire bytes",
        )
        assertEquals(
            0x02,
            wire[0].toInt() and 0x03,
            "first byte low 2 bits must encode arm B's MID (0x02) — if 0, " +
                "the inner Default zero-filled the header byte and β-kotlin " +
                "emission contract is broken",
        )

        val cursor = com.sce.forge.runtime.SceCursor(wire)
        val decoded = CodecVariantDefaultMarker.decode(cursor)
        assertNotNull(decoded, "freshly-constructed codec must decode without error")
        assertEquals(
            0,
            cursor.remaining(),
            "decode must consume every emitted byte; leftover means an " +
                "arm-type mismatch on dispatch",
        )

        assertTrue(
            decoded.body is CodecVariantDefaultMarkerVariant.CodecDefaultMarkerArmB,
            "round-trip must land in arm B (the marked-default); got: ${decoded.body::class.simpleName}",
        )

        val reEncoded = decoded.encode()
        assertEquals(
            wire.toList(),
            reEncoded.toList(),
            "decode → encode must produce byte-equal output (round-trip stability)",
        )
    }
}
