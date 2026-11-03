// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fe63bbaa76604f09a48d24750e75e0a784e90fcb598cdca75c3ac643915bb8d
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

type SchemaGoalPayload struct {
	Id uint32
	Day optional.Of[int64]
	Weight int32
}
