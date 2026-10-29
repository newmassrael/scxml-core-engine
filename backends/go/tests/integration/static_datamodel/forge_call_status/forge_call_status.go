// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: forge_call_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package forge_call_status

import "github.com/newmassrael/sce-forge-runtime/optional"

type ForgeCallStatus uint8

const (
	ForgeCallStatusOpen ForgeCallStatus = 1
	ForgeCallStatusHeld ForgeCallStatus = 2
	ForgeCallStatusDone ForgeCallStatus = 3
)

// ToUnderlying returns the carrier value this holds.
func (v ForgeCallStatus) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v ForgeCallStatus) DeclaredName() optional.Of[string] {
	switch v {
	case ForgeCallStatusOpen:
		return optional.Some("open")
	case ForgeCallStatusHeld:
		return optional.Some("held")
	case ForgeCallStatusDone:
		return optional.Some("done")
	}
	return optional.None[string]()
}

// ForgeCallStatusFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func ForgeCallStatusFromUnderlying(raw uint8) optional.Of[ForgeCallStatus] {
	switch raw {
	case 1:
		return optional.Some(ForgeCallStatusOpen)
	case 2:
		return optional.Some(ForgeCallStatusHeld)
	case 3:
		return optional.Some(ForgeCallStatusDone)
	}
	return optional.None[ForgeCallStatus]()
}
