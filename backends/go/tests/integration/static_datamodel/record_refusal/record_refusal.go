// SCE-GENERATED — DO NOT EDIT
// source-hash: 19a127f54c9deebd5f70030e23e03228c6cca2b28c17631c421836bbc812839a
// source: sce:std/shared/record_refusal.scxml
// SCE-MAP: record_refusal.scxml:39 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package record_refusal

import "github.com/newmassrael/sce-forge-runtime/optional"

type RecordRefusal uint8

const (
	RecordRefusalNotWriter RecordRefusal = 1
	RecordRefusalNoRoomForValue RecordRefusal = 2
	RecordRefusalNoRoomForHistory RecordRefusal = 3
	RecordRefusalNoRoomForDigest RecordRefusal = 4
	RecordRefusalChangeTooLarge RecordRefusal = 6
	RecordRefusalRegisterNamedTwice RecordRefusal = 7
	RecordRefusalClockExhausted RecordRefusal = 8
	RecordRefusalPeerRefused RecordRefusal = 9
)

// ToUnderlying returns the carrier value this holds.
func (v RecordRefusal) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v RecordRefusal) DeclaredName() optional.Of[string] {
	switch v {
	case RecordRefusalNotWriter:
		return optional.Some("notWriter")
	case RecordRefusalNoRoomForValue:
		return optional.Some("noRoomForValue")
	case RecordRefusalNoRoomForHistory:
		return optional.Some("noRoomForHistory")
	case RecordRefusalNoRoomForDigest:
		return optional.Some("noRoomForDigest")
	case RecordRefusalChangeTooLarge:
		return optional.Some("changeTooLarge")
	case RecordRefusalRegisterNamedTwice:
		return optional.Some("registerNamedTwice")
	case RecordRefusalClockExhausted:
		return optional.Some("clockExhausted")
	case RecordRefusalPeerRefused:
		return optional.Some("peerRefused")
	}
	return optional.None[string]()
}

// RecordRefusalFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func RecordRefusalFromUnderlying(raw uint8) optional.Of[RecordRefusal] {
	switch raw {
	case 1:
		return optional.Some(RecordRefusalNotWriter)
	case 2:
		return optional.Some(RecordRefusalNoRoomForValue)
	case 3:
		return optional.Some(RecordRefusalNoRoomForHistory)
	case 4:
		return optional.Some(RecordRefusalNoRoomForDigest)
	case 6:
		return optional.Some(RecordRefusalChangeTooLarge)
	case 7:
		return optional.Some(RecordRefusalRegisterNamedTwice)
	case 8:
		return optional.Some(RecordRefusalClockExhausted)
	case 9:
		return optional.Some(RecordRefusalPeerRefused)
	}
	return optional.None[RecordRefusal]()
}
