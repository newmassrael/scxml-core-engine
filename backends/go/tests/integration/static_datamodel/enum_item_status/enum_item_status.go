// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
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
