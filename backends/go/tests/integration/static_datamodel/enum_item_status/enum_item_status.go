// SCE-GENERATED — DO NOT EDIT
// source-hash: 23b50ff6771257bf06531b96d0a47eeaa5c39fb71939d9bd5e537100a9105206
// SCE-MAP: enum_item_status.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package enum_item_status

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

// DeclaredName returns the name the document declares for this value, and
// false for a value no variant names — one the type can hold whether or not
// its set is open, since a defined integer type holds every carrier value.
func (v EnumItemStatus) DeclaredName() (string, bool) {
	switch v {
	case EnumItemStatusOpen:
		return "open", true
	case EnumItemStatusInProgress:
		return "in_progress", true
	case EnumItemStatusDone:
		return "done", true
	}
	return "", false
}

// EnumItemStatusFromUnderlying returns the variant raw declares, and false
// when raw declares none — the declared set is closed.
func EnumItemStatusFromUnderlying(raw uint8) (EnumItemStatus, bool) {
	switch raw {
	case 2:
		return EnumItemStatusOpen, true
	case 5:
		return EnumItemStatusInProgress, true
	case 7:
		return EnumItemStatusDone, true
	}
	return 0, false
}
