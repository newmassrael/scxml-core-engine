// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: enum_harness_code.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_harness_code

import "github.com/newmassrael/sce-forge-runtime/optional"

type EnumHarnessCode uint8

const (
	EnumHarnessCodeNotJson EnumHarnessCode = 1
	EnumHarnessCodeNotObject EnumHarnessCode = 2
	EnumHarnessCodeTooDeep EnumHarnessCode = 3
	EnumHarnessCodeMissing EnumHarnessCode = 4
	EnumHarnessCodeNullNotOptional EnumHarnessCode = 5
	EnumHarnessCodeWrongKind EnumHarnessCode = 6
	EnumHarnessCodeBadNumber EnumHarnessCode = 7
	EnumHarnessCodeOutOfRange EnumHarnessCode = 8
	EnumHarnessCodeTooLong EnumHarnessCode = 9
	EnumHarnessCodeTooMany EnumHarnessCode = 10
	EnumHarnessCodeUnknownVariant EnumHarnessCode = 11
	EnumHarnessCodeNotByteText EnumHarnessCode = 12
	EnumHarnessCodeNulInText EnumHarnessCode = 13
	EnumHarnessCodeTooFew EnumHarnessCode = 14
	EnumHarnessCodeNotOffered EnumHarnessCode = 15
)

// ToUnderlying returns the carrier value this holds.
func (v EnumHarnessCode) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v EnumHarnessCode) DeclaredName() optional.Of[string] {
	switch v {
	case EnumHarnessCodeNotJson:
		return optional.Some("notJson")
	case EnumHarnessCodeNotObject:
		return optional.Some("notObject")
	case EnumHarnessCodeTooDeep:
		return optional.Some("tooDeep")
	case EnumHarnessCodeMissing:
		return optional.Some("missing")
	case EnumHarnessCodeNullNotOptional:
		return optional.Some("nullNotOptional")
	case EnumHarnessCodeWrongKind:
		return optional.Some("wrongKind")
	case EnumHarnessCodeBadNumber:
		return optional.Some("badNumber")
	case EnumHarnessCodeOutOfRange:
		return optional.Some("outOfRange")
	case EnumHarnessCodeTooLong:
		return optional.Some("tooLong")
	case EnumHarnessCodeTooMany:
		return optional.Some("tooMany")
	case EnumHarnessCodeUnknownVariant:
		return optional.Some("unknownVariant")
	case EnumHarnessCodeNotByteText:
		return optional.Some("notByteText")
	case EnumHarnessCodeNulInText:
		return optional.Some("nulInText")
	case EnumHarnessCodeTooFew:
		return optional.Some("tooFew")
	case EnumHarnessCodeNotOffered:
		return optional.Some("notOffered")
	}
	return optional.None[string]()
}

// EnumHarnessCodeFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func EnumHarnessCodeFromUnderlying(raw uint8) optional.Of[EnumHarnessCode] {
	switch raw {
	case 1:
		return optional.Some(EnumHarnessCodeNotJson)
	case 2:
		return optional.Some(EnumHarnessCodeNotObject)
	case 3:
		return optional.Some(EnumHarnessCodeTooDeep)
	case 4:
		return optional.Some(EnumHarnessCodeMissing)
	case 5:
		return optional.Some(EnumHarnessCodeNullNotOptional)
	case 6:
		return optional.Some(EnumHarnessCodeWrongKind)
	case 7:
		return optional.Some(EnumHarnessCodeBadNumber)
	case 8:
		return optional.Some(EnumHarnessCodeOutOfRange)
	case 9:
		return optional.Some(EnumHarnessCodeTooLong)
	case 10:
		return optional.Some(EnumHarnessCodeTooMany)
	case 11:
		return optional.Some(EnumHarnessCodeUnknownVariant)
	case 12:
		return optional.Some(EnumHarnessCodeNotByteText)
	case 13:
		return optional.Some(EnumHarnessCodeNulInText)
	case 14:
		return optional.Some(EnumHarnessCodeTooFew)
	case 15:
		return optional.Some(EnumHarnessCodeNotOffered)
	}
	return optional.None[EnumHarnessCode]()
}
