// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_content_line_records.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="content-line")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One line of `ATTENDEE` (docs/adr/0014): the parameters it carries and its value. A parameter that may be absent is `nil`.
public struct CodecContentLineRecordsAttendeeLine: Equatable, Sendable {
    /// Parameter `CN`.
    public var attendeeCn: String? = nil
    /// Parameter `ROLE`, required.
    public var attendeeRole: String = ""
    /// The value of the line.
    public var value: String = ""

    public init() {}

    public init(attendeeCn: String?, attendeeRole: String, value: String) {
        self.attendeeCn = attendeeCn
        self.attendeeRole = attendeeRole
        self.value = value
    }
}

/// One line of `EXDATE` (docs/adr/0014): the parameters it carries and its list of values, cut at `,`. A parameter that may be absent is `nil`.
public struct CodecContentLineRecordsExdateLine: Equatable, Sendable {
    /// Parameter `TZID`.
    public var exdateTzid: String? = nil
    /// The values of the line, at most 3.
    public var values: [String] = []

    public init() {}

    public init(exdateTzid: String?, values: [String]) {
        self.exdateTzid = exdateTzid
        self.values = values
    }
}

/// One `VEVENT` component of content lines (SCE_FORGE.md §4.6.4). Decode reads the properties
/// this codec declares in any order and skips the rest; encode writes the entries present in
/// declaration order, folded at 75 octets. A required entry starts at its type's default and an
/// optional one absent (`nil`), so `CodecContentLineRecords()` is the infallible initializer every codec
/// offers. A repeated property that declares a parameter, or whose line holds a list of values, is a
/// list of line records.
public struct CodecContentLineRecords: Equatable, Sendable {
    /// Property `UID`, required.
    public var uid: String = ""
    /// Property `ATTENDEE`, up to 3 lines, each a record.
    public var attendee: [CodecContentLineRecordsAttendeeLine] = []
    /// Property `EXDATE`, up to 3 lines, each a record.
    public var exdate: [CodecContentLineRecordsExdateLine] = []
    /// Property `CATEGORIES`, a list of at most 4 values.
    public var categories: [String] = []
    /// Property `GEO`, a list of at most 2 values.
    public var geo: [String] = []

    public init() {}

    public init(uid: String, attendee: [CodecContentLineRecordsAttendeeLine], exdate: [CodecContentLineRecordsExdateLine], categories: [String], geo: [String]) {
        self.uid = uid
        self.attendee = attendee
        self.exdate = exdate
        self.categories = categories
        self.geo = geo
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
            try out.string(v, text: false, maxSize: 24)
        }
        if self.attendee.count > 3 { throw CodecError.lineTooMany }
        for record in self.attendee {
            let v = record.value
            out.property("ATTENDEE")
            if let attendeeCnValue = record.attendeeCn {
                try out.param("CN", attendeeCnValue, maxSize: 24)
            }
            try out.param("ROLE", record.attendeeRole, maxSize: 16)
            try out.string(v, text: false, maxSize: 40)
        }
        if self.exdate.count > 3 { throw CodecError.lineTooMany }
        for record in self.exdate {
            let v = record.values
            out.property("EXDATE")
            if let exdateTzidValue = record.exdateTzid {
                try out.param("TZID", exdateTzidValue, maxSize: 24)
            }
            try out.strings(v, separator: ",", text: false, maxSize: 20, maxValues: 3)
        }
        do {
            let held = self.categories
            if !held.isEmpty {
                let v = held
                out.property("CATEGORIES")
                try out.strings(v, separator: ",", text: true, maxSize: 12, maxValues: 4)
            }
        }
        do {
            let held = self.geo
            if !held.isEmpty {
                let v = held
                out.property("GEO")
                try out.strings(v, separator: ";", text: false, maxSize: 16, maxValues: 2)
            }
        }
        out.finish()
        try sink.writeBytes(out.bytes)
    }

    /// Decodes one component at the cursor and moves the cursor past its `END:` line. The component
    /// is read from a copy of the input, and the cursor moves only when it was read whole: a
    /// component that is short answers `needMoreBytes`, and any other refusal names the rule that
    /// refused it; either leaves the cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecContentLineRecords {
        let rest = Array(try cursor.peekSlice(cursor.remaining))
        var reader = try ContentLineReader.begin(rest, component: "VEVENT")
        var uidValue: String? = nil
        var attendeeValues: [CodecContentLineRecordsAttendeeLine] = []
        var exdateValues: [CodecContentLineRecordsExdateLine] = []
        var categoriesValue: [String]? = nil
        var geoValue: [String]? = nil
        while var line = try reader.next() {
            if line.isNamed("UID") {
                if uidValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 24, text: false)
                uidValue = v
            } else if line.isNamed("ATTENDEE") {
                if attendeeValues.count >= 3 { throw CodecError.lineTooMany }
                var attendeeCnLine: String? = nil
                var attendeeRoleLine: String? = nil
                while try line.nextParam() {
                    if line.paramNamed("CN") {
                        if attendeeCnLine != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 24)
                        attendeeCnLine = param
                    } else if line.paramNamed("ROLE") {
                        if attendeeRoleLine != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 16)
                        attendeeRoleLine = param
                    }
                }
                let v = try line.readString(maxSize: 40, text: false)
                guard let attendeeRoleFinal = attendeeRoleLine else { throw CodecError.lineRequiredMissing }
                attendeeValues.append(CodecContentLineRecordsAttendeeLine(attendeeCn: attendeeCnLine, attendeeRole: attendeeRoleFinal, value: v))
            } else if line.isNamed("EXDATE") {
                if exdateValues.count >= 3 { throw CodecError.lineTooMany }
                var exdateTzidLine: String? = nil
                while try line.nextParam() {
                    if line.paramNamed("TZID") {
                        if exdateTzidLine != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 24)
                        exdateTzidLine = param
                    }
                }
                let v = try line.readStrings(separator: ",", maxValues: 3, maxSize: 20, text: false)
                exdateValues.append(CodecContentLineRecordsExdateLine(exdateTzid: exdateTzidLine, values: v))
            } else if line.isNamed("CATEGORIES") {
                if categoriesValue != nil { throw CodecError.lineTooMany }
                let v = try line.readStrings(separator: ",", maxValues: 4, maxSize: 12, text: true)
                categoriesValue = v
            } else if line.isNamed("GEO") {
                if geoValue != nil { throw CodecError.lineTooMany }
                let v = try line.readStrings(separator: ";", maxValues: 2, maxSize: 16, text: false)
                geoValue = v
            }
        }
        var frame = CodecContentLineRecords()
        guard let uidFinal = uidValue else { throw CodecError.lineRequiredMissing }
        frame.uid = uidFinal
        frame.attendee = attendeeValues
        frame.exdate = exdateValues
        frame.categories = categoriesValue ?? []
        frame.geo = geoValue ?? []
        // Only a component that decoded whole moves the cursor.
        try cursor.advance(reader.consumed)
        return frame
    }
}
