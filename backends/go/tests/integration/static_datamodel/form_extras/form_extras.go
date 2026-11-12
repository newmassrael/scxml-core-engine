// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f53f6deefa1a9e19646e018ad72cacc30c4c7ca7f7f96f21be9f17f58cfdae
// SCE-MAP: form_extras.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: form.extras
// Do not edit — regenerate from the source SCXML file.

package form_extras
import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_shape_kind_open "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_kind_open"
)

// EventName is the event this schema types (`sce:event-name`), as a host names it.
const EventName string = "form.extras"

// Revision is the revision of the schema (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 1

// SlotsMaxCount is the most elements `slots` holds (the bound its type writes), as this build resolved it.
const SlotsMaxCount uint32 = 3

// FlagsMaxCount is the most elements `flags` holds (the bound its type writes), as this build resolved it.
const FlagsMaxCount uint32 = 2

// SpareMaxCount is the most elements `spare` holds (the bound its type writes), as this build resolved it.
const SpareMaxCount uint32 = 2

// GridMaxCount is the most elements `grid` holds (the bound its type writes), as this build resolved it.
const GridMaxCount uint32 = 2

// DataMaxLen is the most bytes the event's `_event.data` JSON text takes: every list
// full, every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 146

// SchemaID is the id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:form_extras:1"

// No JSONSchema: the enum `Shape` is open (`sce:strict-variants="false"`): a value of it may be a number no name spells, and a grammar spells a value of an enum by its name.

type FormExtrasPayload struct {
	Shape sce_shape_kind_open.ShapeKindOpen
	Slots []optional.Of[uint16]
	Least int64
	Flags []bool
	Spare optional.Of[[]uint8]
	Grid [][]uint8
}
