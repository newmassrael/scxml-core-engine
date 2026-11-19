// SCE-GENERATED — DO NOT EDIT
// source-hash: ba163df4f9edf81a451d27f567c416db19629a5e4279555ff80cf4e47f57b6b0
// source: sce:std/shared/record_outcome.scxml
// SCE-MAP: record_outcome.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package record_outcome

import "github.com/newmassrael/sce-forge-runtime/optional"

type RecordOutcome uint8

const (
	RecordOutcomeNone RecordOutcome = 0
	RecordOutcomeAccepted RecordOutcome = 1
	RecordOutcomeHeld RecordOutcome = 2
	RecordOutcomeWaiting RecordOutcome = 3
	RecordOutcomeRefused RecordOutcome = 4
)

// ToUnderlying returns the carrier value this holds.
func (v RecordOutcome) ToUnderlying() uint8 {
	return uint8(v)
}

// DeclaredName returns the name the document declares for this value, and the
// absent value for a value no variant names — one the type can hold whether or
// not its set is open, since a defined integer type holds every carrier value.
// It is an optional value as every generated answer that may be absent is
// (docs/adr/0055, section 5).
func (v RecordOutcome) DeclaredName() optional.Of[string] {
	switch v {
	case RecordOutcomeNone:
		return optional.Some("none")
	case RecordOutcomeAccepted:
		return optional.Some("accepted")
	case RecordOutcomeHeld:
		return optional.Some("held")
	case RecordOutcomeWaiting:
		return optional.Some("waiting")
	case RecordOutcomeRefused:
		return optional.Some("refused")
	}
	return optional.None[string]()
}

// RecordOutcomeFromUnderlying returns the variant raw declares, and the absent
// value when raw declares none — the declared set is closed.
func RecordOutcomeFromUnderlying(raw uint8) optional.Of[RecordOutcome] {
	switch raw {
	case 0:
		return optional.Some(RecordOutcomeNone)
	case 1:
		return optional.Some(RecordOutcomeAccepted)
	case 2:
		return optional.Some(RecordOutcomeHeld)
	case 3:
		return optional.Some(RecordOutcomeWaiting)
	case 4:
		return optional.Some(RecordOutcomeRefused)
	}
	return optional.None[RecordOutcome]()
}
