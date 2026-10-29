// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: enum_item_status.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_item_status

import "github.com/newmassrael/sce-forge-runtime/optional"

type EnumItemStatus uint8

const (
	EnumItemStatusOpen EnumItemStatus = 2
	EnumItemStatusInProgress EnumItemStatus = 5
	EnumItemStatusDone EnumItemStatus = 7
)

// ToUnderlying returns the carrier value this holds.
func (v EnumItemStatus) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v EnumItemStatus) DeclaredName() optional.Of[string] {
	switch v {
	case EnumItemStatusOpen:
		return optional.Some("open")
	case EnumItemStatusInProgress:
		return optional.Some("in_progress")
	case EnumItemStatusDone:
		return optional.Some("done")
	}
	return optional.None[string]()
}

// EnumItemStatusFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func EnumItemStatusFromUnderlying(raw uint8) optional.Of[EnumItemStatus] {
	switch raw {
	case 2:
		return optional.Some(EnumItemStatusOpen)
	case 5:
		return optional.Some(EnumItemStatusInProgress)
	case 7:
		return optional.Some(EnumItemStatusDone)
	}
	return optional.None[EnumItemStatus]()
}
