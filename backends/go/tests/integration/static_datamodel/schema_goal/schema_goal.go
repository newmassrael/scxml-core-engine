// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
// SCE-MAP: schema_goal.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: goal.added
// Do not edit — regenerate from the source SCXML file.

package schema_goal
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "goal.added"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 67

type SchemaGoalPayload struct {
	Id uint32
	Day optional.Of[int64]
	Weight int32
}
