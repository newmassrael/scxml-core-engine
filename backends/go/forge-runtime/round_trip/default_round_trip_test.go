// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// RFC variant-default-uniformity go half — runtime
// round-trip property test. Mirrors
// backends/rust/forge-conformance/tests/forge_default_round_trip.rs for the Go
// backend: imports the generated codec packages and asserts that a
// freshly-constructed instance round-trips through encode → decode
// into the same declared default arm.
//
// Generated codec packages live under ./generated/ and are
// (re-)produced by generate.sh. Adding or editing the 3 marker
// fixtures requires re-running that script.

package round_trip

import (
	"bytes"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_variant_default_marker"
)

func TestDefaultRoundTripLandsInDeclaredDefaultArm(t *testing.T) {
	// Go has no Default trait: `NewCodecVariantDefaultMarker()` (RFC
	// variant-default-uniformity) is the value made by default, and a value
	// that holds no arm presents the marked default arm, made by its codec's
	// constructor, so the zero value writes the same frame (checked below).
	original := codec_variant_default_marker.NewCodecVariantDefaultMarker()
	wire := original.EncodeToBytes()

	if len(wire) != 3 {
		t.Fatalf(
			"default-emit + arm B (uint16 payload) must produce 3 wire bytes; got %d: %v",
			len(wire), wire,
		)
	}
	if wire[0]&0x03 != 0x02 {
		t.Fatalf(
			"first byte low 2 bits must encode arm B's MID (0x02); got 0x%02X — "+
				"inner Default zero-filled the header byte (β-go inner ctor change "+
				"didn't take effect)",
			wire[0],
		)
	}

	cursor := codec.NewSceCursor(wire)
	decoded, err := codec_variant_default_marker.DecodeCodecVariantDefaultMarker(&cursor)
	if err != nil {
		t.Fatalf("freshly-constructed codec must decode without error; got: %v", err)
	}
	if cursor.Remaining() != 0 {
		t.Fatalf(
			"decode must consume every emitted byte; %d byte(s) leftover",
			cursor.Remaining(),
		)
	}

	// The value holds one arm type of its variant (docs/adr/0081 step 4).
	// Arm B is the marked default, so the decode must hold arm B.
	switch arm := decoded.Body().(type) {
	case codec_variant_default_marker.CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB:
	case codec_variant_default_marker.CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmA:
		t.Fatalf(
			"round-trip dropped into arm A (the legacy first-declared arm) — " +
				"β-go outer ctor change didn't take effect",
		)
	case codec_variant_default_marker.CodecVariantDefaultMarkerVariantDefault:
		t.Fatalf(
			"round-trip dropped into the catch-all <sce:default> arm "+
				"(tag=0x%02X) — inner Default zero-filled the header byte",
			arm.Tag().Value(),
		)
	default:
		t.Fatalf("round-trip dropped into no arm at all — decode is broken")
	}

	// The zero value presents the arm a value made by default holds, so it
	// writes the same frame.
	var zero codec_variant_default_marker.CodecVariantDefaultMarker
	if !bytes.Equal(wire, zero.EncodeToBytes()) {
		t.Fatalf("the zero value must write the frame of a value made by default; got %v", zero.EncodeToBytes())
	}

	reEncoded := decoded.EncodeToBytes()
	if !bytes.Equal(wire, reEncoded) {
		t.Fatalf(
			"decode → encode must produce byte-equal output; original=%v re-encoded=%v",
			wire, reEncoded,
		)
	}
}
