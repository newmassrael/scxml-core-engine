// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22
// SCE-MAP: text_notice.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="text-template")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package text_notice

import (
	"strconv"
	"strings"

	"github.com/newmassrael/sce-forge-runtime/texttemplate"
)

// Revision is the revision of the template (`sce:revision`, 1 when the document declares none).
const Revision uint32 = 2

// MaxLen is the most UTF-8 bytes Render returns: the literal text, and every hole at
// its longest (docs/adr/0087, section 3.1, item 5).
const MaxLen int = 95

// FenceOpen is the marker an untrusted hole's text opens with.
const FenceOpen string = "[[TEXT"

// FenceClose is the marker an untrusted hole's text closes with.
const FenceClose string = "TEXT]]"

// Render renders the template `text_notice`: its text with each hole put where
// the document puts it, the same bytes on every backend. Each text is at most its
// hole's bound, which a statechart's `<sce:render>` holds by construction; the result
// is then at most MaxLen bytes.
//   - holeWho: the hole `who`, a text of at most 8 UTF-8 bytes, put as it is (trusted).
//   - holeCount: the hole `count`, a int16, put as its decimal digits.
//   - holeView: the hole `view`: the declared name of a variant of enum:ViewMode, put as it is.
//   - holeSaid: the hole `said`, a text of at most 16 UTF-8 bytes, put between the fence's markers, every marker removed from it (untrusted).
func Render(holeWho string, holeCount int16, holeView string, holeSaid string) string {
	var out strings.Builder
	out.Grow(MaxLen)
	out.WriteString(holeWho)
	out.WriteString(" (")
	out.WriteString(strconv.FormatInt(int64(holeCount), 10))
	out.WriteString(", ")
	out.WriteString(holeView)
	out.WriteString("): ")
	out.WriteString(FenceOpen)
	out.WriteString(texttemplate.RemoveMarkers(holeSaid, FenceOpen, FenceClose))
	out.WriteString(FenceClose)
	out.WriteString("\nagain ")
	out.WriteString(FenceOpen)
	out.WriteString(texttemplate.RemoveMarkers(holeSaid, FenceOpen, FenceClose))
	out.WriteString(FenceClose)
	return out.String()
}
