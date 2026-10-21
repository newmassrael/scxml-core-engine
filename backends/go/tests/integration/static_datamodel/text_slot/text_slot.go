// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
// SCE-MAP: text_slot.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.slot
// Do not edit — regenerate from the source SCXML file.

package text_slot

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "text.slot"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// TextMaxSize is the most UTF-8 bytes `text` holds (`sce:max-size`), as this build resolved it.
const TextMaxSize uint32 = 32

type TextSlotPayload struct {
	Slot uint32
	Version int64
	Text string
}
