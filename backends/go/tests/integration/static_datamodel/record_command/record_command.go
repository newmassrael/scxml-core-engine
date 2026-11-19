// SCE-GENERATED — DO NOT EDIT
// source-hash: d33807f273908bcf0c618164c9358f1caa01fcae9ec6f6c74812a2887ca07eec
// source: sce:std/shared/record_command.scxml
// SCE-MAP: record_command.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package record_command

import "github.com/newmassrael/sce-forge-runtime/optional"

type RecordCommand uint8

const (
	RecordCommandEdit RecordCommand = 0
	RecordCommandClear RecordCommand = 1
	RecordCommandRetire RecordCommand = 2
)

// ToUnderlying returns the carrier value this holds.
func (v RecordCommand) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v RecordCommand) DeclaredName() optional.Of[string] {
	switch v {
	case RecordCommandEdit:
		return optional.Some("edit")
	case RecordCommandClear:
		return optional.Some("clear")
	case RecordCommandRetire:
		return optional.Some("retire")
	}
	return optional.None[string]()
}

// RecordCommandFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func RecordCommandFromUnderlying(raw uint8) optional.Of[RecordCommand] {
	switch raw {
	case 0:
		return optional.Some(RecordCommandEdit)
	case 1:
		return optional.Some(RecordCommandClear)
	case 2:
		return optional.Some(RecordCommandRetire)
	}
	return optional.None[RecordCommand]()
}
