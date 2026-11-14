// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_content_line_open.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="content-line")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One line of `ATTENDEE` (docs/adr/0014): the parameters it carries and its value. A parameter that may be absent is `nil`.
public struct CodecContentLineOpenAttendeeLine: Equatable, Sendable {
    /// Parameter `CN`.
    public var attendeeCn: String? = nil
    /// The value of the line.
    public var value: String = ""

    public init() {}

    public init(attendeeCn: String?, value: String) {
        self.attendeeCn = attendeeCn
        self.value = value
    }
}

/// One `VEVENT` component of content lines (SCE_FORGE.md §4.6.4). Decode reads the properties
/// this codec declares in any order and skips the rest; encode writes the entries present in
/// declaration order, folded at 75 octets. A required entry starts at its type's default and an
/// optional one absent (`nil`), so `CodecContentLineOpen()` is the infallible initializer every codec
/// offers. A repeated property that declares a parameter, or whose line holds a list of values, is a
/// list of line records.
public struct CodecContentLineOpen: Equatable, Sendable {
    /// Property `UID`, required.
    public var uid: String = ""
    /// Property `DESCRIPTION`.
    public var description: String? = nil
    /// Property `ATTENDEE`, any number of lines, each a record, required.
    public var attendee: [CodecContentLineOpenAttendeeLine] = []
    /// Property `CATEGORIES`, a list of any number of values.
    public var categories: [String] = []
    /// Property `RESOURCES`, up to 2 lines.
    public var resources: [String] = []

    public init() {}

    public init(uid: String, description: String?, attendee: [CodecContentLineOpenAttendeeLine], categories: [String], resources: [String]) {
        self.uid = uid
        self.description = description
        self.attendee = attendee
        self.categories = categories
        self.resources = resources
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
        do {
            let held = self.description
            if let v = held {
                out.property("DESCRIPTION")
                try out.string(v, text: true, maxSize: ContentLine.unbounded)
            }
        }
        if self.attendee.isEmpty { throw CodecError.lineRequiredMissing }
        for record in self.attendee {
            let v = record.value
            out.property("ATTENDEE")
            if let attendeeCnValue = record.attendeeCn {
                try out.param("CN", attendeeCnValue, maxSize: ContentLine.unbounded)
            }
            try out.string(v, text: false, maxSize: ContentLine.unbounded)
        }
        do {
            let held = self.categories
            if !held.isEmpty {
                let v = held
                out.property("CATEGORIES")
                try out.strings(v, separator: ",", text: true, maxSize: ContentLine.unbounded, maxValues: ContentLine.unbounded)
            }
        }
        if self.resources.count > 2 { throw CodecError.lineTooMany }
        for v in self.resources {
            out.property("RESOURCES")
            try out.string(v, text: false, maxSize: ContentLine.unbounded)
        }
        out.finish()
        try sink.writeBytes(out.bytes)
    }

    /// Decodes one component at the cursor and moves the cursor past its `END:` line. The component
    /// is read from a copy of the input, and the cursor moves only when it was read whole: a
    /// component that is short answers `needMoreBytes`, and any other refusal names the rule that
    /// refused it; either leaves the cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecContentLineOpen {
        let rest = Array(try cursor.peekSlice(cursor.remaining))
        var reader = try ContentLineReader.begin(rest, component: "VEVENT")
        var uidValue: String? = nil
        var descriptionValue: String? = nil
        var attendeeValues: [CodecContentLineOpenAttendeeLine] = []
        var categoriesValue: [String]? = nil
        var resourcesValues: [String] = []
        while var line = try reader.next() {
            if line.isNamed("UID") {
                if uidValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 16, text: false)
                uidValue = v
            } else if line.isNamed("DESCRIPTION") {
                if descriptionValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: ContentLine.unbounded, text: true)
                descriptionValue = v
            } else if line.isNamed("ATTENDEE") {
                var attendeeCnLine: String? = nil
                while try line.nextParam() {
                    if line.paramNamed("CN") {
                        if attendeeCnLine != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: ContentLine.unbounded)
                        attendeeCnLine = param
                    }
                }
                let v = try line.readString(maxSize: ContentLine.unbounded, text: false)
                attendeeValues.append(CodecContentLineOpenAttendeeLine(attendeeCn: attendeeCnLine, value: v))
            } else if line.isNamed("CATEGORIES") {
                if categoriesValue != nil { throw CodecError.lineTooMany }
                let v = try line.readStrings(separator: ",", maxValues: ContentLine.unbounded, maxSize: ContentLine.unbounded, text: true)
                categoriesValue = v
            } else if line.isNamed("RESOURCES") {
                if resourcesValues.count >= 2 { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: ContentLine.unbounded, text: false)
                resourcesValues.append(v)
            }
        }
        // A required list needs a line.
        if attendeeValues.isEmpty { throw CodecError.lineRequiredMissing }
        var frame = CodecContentLineOpen()
        guard let uidFinal = uidValue else { throw CodecError.lineRequiredMissing }
        frame.uid = uidFinal
        frame.description = descriptionValue
        frame.attendee = attendeeValues
        frame.categories = categoriesValue ?? []
        frame.resources = resourcesValues
        // Only a component that decoded whole moves the cursor.
        try cursor.advance(reader.consumed)
        return frame
    }
}
