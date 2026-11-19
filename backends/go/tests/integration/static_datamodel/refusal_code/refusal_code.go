// SCE-GENERATED — DO NOT EDIT
// source-hash: 02adaafa0792f63058539be94114b73db781d48032e02bada12e15df066d4aad
// source: sce:std/lift/refusal_code.scxml
// SCE-MAP: refusal_code.scxml:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package refusal_code

import "github.com/newmassrael/sce-forge-runtime/optional"

type RefusalCode uint8

const (
	RefusalCodeNotJson RefusalCode = 1
	RefusalCodeNotObject RefusalCode = 2
	RefusalCodeTooDeep RefusalCode = 3
	RefusalCodeMissing RefusalCode = 4
	RefusalCodeNullNotOptional RefusalCode = 5
	RefusalCodeWrongKind RefusalCode = 6
	RefusalCodeBadNumber RefusalCode = 7
	RefusalCodeOutOfRange RefusalCode = 8
	RefusalCodeTooLong RefusalCode = 9
	RefusalCodeTooMany RefusalCode = 10
	RefusalCodeUnknownVariant RefusalCode = 11
	RefusalCodeNotByteText RefusalCode = 12
	RefusalCodeNulInText RefusalCode = 13
	RefusalCodeTooFew RefusalCode = 14
)

// ToUnderlying returns the carrier value this holds.
func (v RefusalCode) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v RefusalCode) DeclaredName() optional.Of[string] {
	switch v {
	case RefusalCodeNotJson:
		return optional.Some("notJson")
	case RefusalCodeNotObject:
		return optional.Some("notObject")
	case RefusalCodeTooDeep:
		return optional.Some("tooDeep")
	case RefusalCodeMissing:
		return optional.Some("missing")
	case RefusalCodeNullNotOptional:
		return optional.Some("nullNotOptional")
	case RefusalCodeWrongKind:
		return optional.Some("wrongKind")
	case RefusalCodeBadNumber:
		return optional.Some("badNumber")
	case RefusalCodeOutOfRange:
		return optional.Some("outOfRange")
	case RefusalCodeTooLong:
		return optional.Some("tooLong")
	case RefusalCodeTooMany:
		return optional.Some("tooMany")
	case RefusalCodeUnknownVariant:
		return optional.Some("unknownVariant")
	case RefusalCodeNotByteText:
		return optional.Some("notByteText")
	case RefusalCodeNulInText:
		return optional.Some("nulInText")
	case RefusalCodeTooFew:
		return optional.Some("tooFew")
	}
	return optional.None[string]()
}

// RefusalCodeFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func RefusalCodeFromUnderlying(raw uint8) optional.Of[RefusalCode] {
	switch raw {
	case 1:
		return optional.Some(RefusalCodeNotJson)
	case 2:
		return optional.Some(RefusalCodeNotObject)
	case 3:
		return optional.Some(RefusalCodeTooDeep)
	case 4:
		return optional.Some(RefusalCodeMissing)
	case 5:
		return optional.Some(RefusalCodeNullNotOptional)
	case 6:
		return optional.Some(RefusalCodeWrongKind)
	case 7:
		return optional.Some(RefusalCodeBadNumber)
	case 8:
		return optional.Some(RefusalCodeOutOfRange)
	case 9:
		return optional.Some(RefusalCodeTooLong)
	case 10:
		return optional.Some(RefusalCodeTooMany)
	case 11:
		return optional.Some(RefusalCodeUnknownVariant)
	case 12:
		return optional.Some(RefusalCodeNotByteText)
	case 13:
		return optional.Some(RefusalCodeNulInText)
	case 14:
		return optional.Some(RefusalCodeTooFew)
	}
	return optional.None[RefusalCode]()
}
