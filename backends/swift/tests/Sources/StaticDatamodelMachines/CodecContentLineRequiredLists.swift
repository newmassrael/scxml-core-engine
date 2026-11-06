// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_content_line_required_lists.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="content-line")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One line of `REC` (docs/adr/0014): the parameters it carries and its value. A parameter that may be absent is `nil`.
public struct CodecContentLineRequiredListsRecLine: Equatable, Sendable {
    /// Parameter `KIND`.
    public var recKind: String? = nil
    /// The value of the line.
    public var value: String = ""

    public init() {}

    public init(recKind: String?, value: String) {
        self.recKind = recKind
        self.value = value
    }
}

/// One `VEVENT` component of content lines (SCE_FORGE.md §4.6.4). Decode reads the properties
/// this codec declares in any order and skips the rest; encode writes the entries present in
/// declaration order, folded at 75 octets. A required entry starts at its type's default and an
/// optional one absent (`nil`), so `CodecContentLineRequiredLists()` is the infallible initializer every codec
/// offers. A repeated property that declares a parameter, or whose line holds a list of values, is a
/// list of line records.
public struct CodecContentLineRequiredLists: Equatable, Sendable {
    /// Property `UID`, required.
    public var uid: String = ""
    /// Property `TAG`, up to 2 lines, required.
    public var tag: [String] = []
    /// Property `CATS`, a list of at most 3 values, required.
    public var cats: [String] = []
    /// Property `REC`, up to 2 lines, each a record, required.
    public var rec: [CodecContentLineRequiredListsRecLine] = []

    public init() {}

    public init(uid: String, tag: [String], cats: [String], rec: [CodecContentLineRequiredListsRecLine]) {
        self.uid = uid
        self.tag = tag
        self.cats = cats
        self.rec = rec
    }

    /// Writes this component to `sink` in one write: a value past its declared bound, or one a line
    /// could not carry, is refused and nothing is written, and a bounded sink that cannot hold all of
    /// it answers `bufferOverflow` and holds none of it (SCE_FORGE.md §4.6.4).
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var out = ContentLineWriter(component: "VEVENT")
        out.begin()
        do {
            let v = self.uid
            out.property("UID")
            try out.string(v, text: false, maxSize: 16)
        }
        if self.tag.count > 2 { throw CodecError.lineTooMany }
        if self.tag.isEmpty { throw CodecError.lineRequiredMissing }
        for v in self.tag {
            out.property("TAG")
            try out.string(v, text: false, maxSize: 8)
        }
        do {
            let v = self.cats
            out.property("CATS")
            try out.strings(v, separator: ",", text: true, maxSize: 8, maxValues: 3)
        }
        if self.rec.count > 2 { throw CodecError.lineTooMany }
        if self.rec.isEmpty { throw CodecError.lineRequiredMissing }
        for record in self.rec {
            let v = record.value
            out.property("REC")
            if let recKindValue = record.recKind {
                try out.param("KIND", recKindValue, maxSize: 8)
            }
            try out.string(v, text: false, maxSize: 8)
        }
        out.finish()
        try sink.writeBytes(out.bytes)
    }

    /// Decodes one component at the cursor and moves the cursor past its `END:` line. The component
    /// is read from a copy of the input, and the cursor moves only when it was read whole: a
    /// component that is short answers `needMoreBytes`, and any other refusal names the rule that
    /// refused it; either leaves the cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecContentLineRequiredLists {
        let rest = Array(try cursor.peekSlice(cursor.remaining))
        var reader = try ContentLineReader.begin(rest, component: "VEVENT")
        var uidValue: String? = nil
        var tagValues: [String] = []
        var catsValue: [String]? = nil
        var recValues: [CodecContentLineRequiredListsRecLine] = []
        while var line = try reader.next() {
            if line.isNamed("UID") {
                if uidValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 16, text: false)
                uidValue = v
            } else if line.isNamed("TAG") {
                if tagValues.count >= 2 { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 8, text: false)
                tagValues.append(v)
            } else if line.isNamed("CATS") {
                if catsValue != nil { throw CodecError.lineTooMany }
                let v = try line.readStrings(separator: ",", maxValues: 3, maxSize: 8, text: true)
                catsValue = v
            } else if line.isNamed("REC") {
                if recValues.count >= 2 { throw CodecError.lineTooMany }
                var recKindLine: String? = nil
                while try line.nextParam() {
                    if line.paramNamed("KIND") {
                        if recKindLine != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 8)
                        recKindLine = param
                    }
                }
                let v = try line.readString(maxSize: 8, text: false)
                recValues.append(CodecContentLineRequiredListsRecLine(recKind: recKindLine, value: v))
            }
        }
        // A required list needs a line.
        if tagValues.isEmpty { throw CodecError.lineRequiredMissing }
        // A required list needs a line.
        if recValues.isEmpty { throw CodecError.lineRequiredMissing }
        var frame = CodecContentLineRequiredLists()
        guard let uidFinal = uidValue else { throw CodecError.lineRequiredMissing }
        frame.uid = uidFinal
        frame.tag = tagValues
        guard let catsFinal = catsValue else { throw CodecError.lineRequiredMissing }
        frame.cats = catsFinal
        frame.rec = recValues
        // Only a component that decoded whole moves the cursor.
        try cursor.advance(reader.consumed)
        return frame
    }
}
