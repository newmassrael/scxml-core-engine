// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22
// SCE-MAP: text_notice.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="text-template")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.text_notice

/** The template `text_notice` (docs/adr/0087, section 3). */
public object TextNotice {
    /** The revision of the template (`sce:revision`, 1 when the document declares none). */
    public const val REVISION: UInt = 2u

    /** The most UTF-8 bytes [render] returns: the literal text, and every hole at its longest. */
    public const val MAX_LEN: Int = 95

    /** The marker an untrusted hole's text opens with. */
    public const val FENCE_OPEN: String = "[[TEXT"

    /** The marker an untrusted hole's text closes with. */
    public const val FENCE_CLOSE: String = "TEXT]]"

    /**
     * Renders the template: its text with each hole put where the document puts it, the
     * same text on every backend. Each text is at most its hole's bound in UTF-8 bytes,
     * which a statechart's `<sce:render>` holds by construction; the result is then at
     * most [MAX_LEN] UTF-8 bytes.
     * - [holeWho]: the hole `who`, a text of at most 8 UTF-8 bytes, put as it is (trusted).
     * - [holeCount]: the hole `count`, a int16, put as its decimal digits.
     * - [holeView]: the hole `view`: the declared name of a variant of enum:ViewMode, put as it is.
     * - [holeSaid]: the hole `said`, a text of at most 16 UTF-8 bytes, put between the fence's markers, every marker removed from it (untrusted).
     */
    public fun render(holeWho: String, holeCount: Short, holeView: String, holeSaid: String): String {
        val out = StringBuilder(MAX_LEN)
        out.append(holeWho)
        out.append(" (")
        out.append(holeCount.toString())
        out.append(", ")
        out.append(holeView)
        out.append("): ")
        out.append(FENCE_OPEN)
        out.append(com.sce.forge.runtime.TextTemplate.removeMarkers(holeSaid, FENCE_OPEN, FENCE_CLOSE))
        out.append(FENCE_CLOSE)
        out.append("\nagain ")
        out.append(FENCE_OPEN)
        out.append(com.sce.forge.runtime.TextTemplate.removeMarkers(holeSaid, FENCE_OPEN, FENCE_CLOSE))
        out.append(FENCE_CLOSE)
        return out.toString()
    }
}
