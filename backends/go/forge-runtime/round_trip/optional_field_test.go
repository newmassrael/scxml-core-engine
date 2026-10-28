// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0055, section 8 ("One representation"): a value a codec reads and a
// statechart holds has one type. A generated statechart holds an
// `optional<T>` variable as `optional.Of[T]`, and these tests hold the generated
// codecs to the same type: the field a decode fills is assigned to such a
// variable, and a variable is assigned to the field an encode reads, with no
// conversion in between (an assignment of one type to another of its name
// would not compile), and an absent value and a present zero are two values
// on the way through.
//
// The codecs under test are generated packages under ./generated/, produced
// by generate.sh from the documents of tests/forge/resources.

package round_trip

import (
	"bytes"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/codec"
	"github.com/newmassrael/sce-forge-runtime/optional"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_cbor_map"
	"github.com/newmassrael/sce-forge-runtime/round_trip/generated/codec_present_if_basic"
)

// variables are the variables of a statechart that holds what the codecs
// carry: the types a generated machine spells `optional<uint16>`,
// `optional<uint64>` and `optional<bool>` with.
type variables struct {
	seq      optional.Of[uint16]
	deadline optional.Of[uint64]
	urgent   optional.Of[bool]
}

// A field gated by `sce:present-if` is read into a statechart's variable whole,
// and written back from one: present with its value when the carrier's flag is
// set, absent when it is not.
func TestAGatedFieldIsTheTypeAStatechartVariableHolds(t *testing.T) {
	frames := []struct {
		name  string
		wire  []byte
		value optional.Of[uint16]
	}{
		{"flag set", []byte{0x01, 0x12, 0x34}, optional.Some(uint16(0x1234))},
		{"flag clear", []byte{0x00}, optional.None[uint16]()},
	}
	for _, f := range frames {
		cursor := codec.NewSceCursor(f.wire)
		decoded, err := codec_present_if_basic.DecodeCodecPresentIfBasic(&cursor)
		if err != nil {
			t.Fatalf("%s: decode: %v", f.name, err)
		}
		held := variables{seq: decoded.Seq}
		if !held.seq.Equal(f.value) {
			t.Errorf("%s: the variable holds %v, want %v", f.name, held.seq, f.value)
		}

		written := codec_present_if_basic.CodecPresentIfBasic{Flags: decoded.Flags, Seq: held.seq}
		if got := written.EncodeToBytes(); !bytes.Equal(got, f.wire) {
			t.Errorf("%s: written from the variable as % x, want % x", f.name, got, f.wire)
		}
	}
}

// cborMapFrame is a map of `entries` entries whose four required entries are
// the oracle's (tests/forge/conformance/numerical_reference.json,
// `codec_cbor_map`: id 00 01 ... 0f, source "node-a", pattern 3, payload de ad),
// followed by `tail`, the optional entries in ascending key order.
func cborMapFrame(entries byte, tail ...byte) []byte {
	frame := []byte{entries, 0x00, 0x50}
	for i := byte(0); i < 16; i++ {
		frame = append(frame, i)
	}
	frame = append(frame, 0x01, 0x66, 'n', 'o', 'd', 'e', '-', 'a', 0x03, 0x03, 0x05, 0x42, 0xde, 0xad)
	return append(frame, tail...)
}

// An optional entry of a CBOR map is read into a statechart's variable whole and
// written back from one. A present `false` and a present zero are values, and an
// entry the map lacks is absent: the three are told apart in both directions.
func TestAnOptionalCborEntryIsTheTypeAStatechartVariableHolds(t *testing.T) {
	frames := []struct {
		name     string
		wire     []byte
		deadline optional.Of[uint64]
		urgent   optional.Of[bool]
	}{
		{
			"neither written",
			cborMapFrame(0xa4),
			optional.None[uint64](),
			optional.None[bool](),
		},
		{
			"a present false and a present zero",
			cborMapFrame(0xa6, 0x07, 0xf4, 0x0c, 0x00),
			optional.Some(uint64(0)),
			optional.Some(false),
		},
		{
			"both written with a value",
			cborMapFrame(0xa6, 0x07, 0xf5, 0x0c, 0x1b, 0, 0, 0, 1, 0, 0, 0, 0),
			optional.Some(uint64(1) << 32),
			optional.Some(true),
		},
	}
	for _, f := range frames {
		cursor := codec.NewSceCursor(f.wire)
		decoded, err := codec_cbor_map.DecodeCodecCborMap(&cursor)
		if err != nil {
			t.Fatalf("%s: decode: %v", f.name, err)
		}
		held := variables{deadline: decoded.Deadline, urgent: decoded.Urgent}
		if !held.deadline.Equal(f.deadline) {
			t.Errorf("%s: the deadline variable holds %v, want %v", f.name, held.deadline, f.deadline)
		}
		if !held.urgent.Equal(f.urgent) {
			t.Errorf("%s: the urgent variable holds %v, want %v", f.name, held.urgent, f.urgent)
		}

		written := codec_cbor_map.NewCodecCborMap()
		written.Id = decoded.Id
		written.Source = decoded.Source
		written.Pattern = decoded.Pattern
		written.Payload = decoded.Payload
		written.Deadline = held.deadline
		written.Urgent = held.urgent
		got, err := written.EncodeToBytes()
		if err != nil {
			t.Fatalf("%s: encode refused: %v", f.name, err)
		}
		if !bytes.Equal(got, f.wire) {
			t.Errorf("%s: written from the variables as % x, want % x", f.name, got, f.wire)
		}
	}
}
