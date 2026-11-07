// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
// SCE-MAP: text_notice.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="text-template")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The template `text_notice` (docs/adr/0087, section 3). A caseless enum of the
/// module, as an algorithm's is, so two templates never share a name.
public enum TextNotice {
    /// The revision of the template (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 2

    /// The most UTF-8 bytes `render` returns: the literal text, and every hole at its longest.
    public static let maxLen: Int = 95

    /// The marker an untrusted hole's text opens with.
    public static let fenceOpen: String = "[[TEXT"

    /// The marker an untrusted hole's text closes with.
    public static let fenceClose: String = "TEXT]]"

    /// Renders the template: its text with each hole put where the document puts it, the same
    /// bytes on every backend. Each text is at most its hole's bound in UTF-8 bytes, which a
    /// statechart's `<sce:render>` holds by construction; the result is then at most `maxLen`
    /// UTF-8 bytes.
    /// - `holeWho`: the hole `who`, a text of at most 8 UTF-8 bytes, put as it is (trusted).
    /// - `holeCount`: the hole `count`, a int16, put as its decimal digits.
    /// - `holeView`: the hole `view`: the declared name of a variant of enum:ViewMode, put as it is.
    /// - `holeSaid`: the hole `said`, a text of at most 16 UTF-8 bytes, put between the fence's markers, every marker removed from it (untrusted).
    public static func render(_ holeWho: String, _ holeCount: Int16, _ holeView: String, _ holeSaid: String) -> String {
        var out = ""
        out.reserveCapacity(maxLen)
        out += holeWho
        out += " ("
        out += String(holeCount)
        out += ", "
        out += holeView
        out += "): "
        out += fenceOpen
        out += SceForgeRuntime.TextTemplate.removeMarkers(holeSaid, open: fenceOpen, close: fenceClose)
        out += fenceClose
        out += "\nagain "
        out += fenceOpen
        out += SceForgeRuntime.TextTemplate.removeMarkers(holeSaid, open: fenceOpen, close: fenceClose)
        out += fenceClose
        return out
    }
}
