// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: codec_content_line_event.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="content-line")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One `VEVENT` component of content lines (SCE_FORGE.md §4.6.4). Decode reads the properties
/// this codec declares in any order and skips the rest; encode writes the entries present in
/// declaration order, folded at 75 octets. A required entry starts at its type's default and an
/// optional one absent (`nil`), so `CodecContentLineEvent()` is the infallible initializer every codec
/// offers. A repeated property that declares a parameter, or whose line holds a list of values, is a
/// list of line records.
public struct CodecContentLineEvent: Equatable, Sendable {
    /// Property `UID`, required.
    public var uid: String = ""
    /// Property `DTSTART`, required.
    public var dtstart: String = ""
    /// Parameter `TZID` of `DTSTART`, required.
    public var dtstartTzid: String = ""
    /// Property `SUMMARY`.
    public var summary: String? = nil
    /// Property `DESCRIPTION`.
    public var description: String? = nil
    /// Property `ORGANIZER`.
    public var organizer: String? = nil
    /// Parameter `CN` of `ORGANIZER`.
    public var organizerCn: String? = nil
    /// Property `RRULE`.
    public var rrule: String? = nil
    /// Property `EXDATE`, up to 3 lines.
    public var exdate: [String] = []
    /// Property `SEQUENCE`.
    public var sequence: UInt16? = nil
    /// Property `PRIORITY`.
    public var priority: Int32? = nil
    /// Property `X-ALL-DAY`.
    public var allDay: Bool? = nil

    public init() {}

    public init(uid: String, dtstart: String, dtstartTzid: String, summary: String?, description: String?, organizer: String?, organizerCn: String?, rrule: String?, exdate: [String], sequence: UInt16?, priority: Int32?, allDay: Bool?) {
        self.uid = uid
        self.dtstart = dtstart
        self.dtstartTzid = dtstartTzid
        self.summary = summary
        self.description = description
        self.organizer = organizer
        self.organizerCn = organizerCn
        self.rrule = rrule
        self.exdate = exdate
        self.sequence = sequence
        self.priority = priority
        self.allDay = allDay
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
        do {
            let v = self.dtstart
            out.property("DTSTART")
            try out.param("TZID", self.dtstartTzid, maxSize: 24)
            try out.string(v, text: false, maxSize: 20)
        }
        do {
            let held = self.summary
            if let v = held {
                out.property("SUMMARY")
                try out.string(v, text: true, maxSize: 24)
            }
        }
        do {
            let held = self.description
            if let v = held {
                out.property("DESCRIPTION")
                try out.string(v, text: true, maxSize: 160)
            }
        }
        do {
            let held = self.organizer
            let organizerCnHeld = self.organizerCn
            if let v = held {
                out.property("ORGANIZER")
                if let organizerCnValue = organizerCnHeld {
                    try out.param("CN", organizerCnValue, maxSize: 24)
                }
                try out.string(v, text: false, maxSize: 40)
            } else if organizerCnHeld != nil {
                // A parameter with no value of its property has no line to stand on.
                throw CodecError.lineRequiredMissing
            }
        }
        do {
            let held = self.rrule
            if let v = held {
                out.property("RRULE")
                try out.string(v, text: false, maxSize: 40)
            }
        }
        if self.exdate.count > 3 { throw CodecError.lineTooMany }
        for v in self.exdate {
            out.property("EXDATE")
            try out.string(v, text: false, maxSize: 20)
        }
        do {
            let held = self.sequence
            if let v = held {
                out.property("SEQUENCE")
                out.uint(UInt64(v))
            }
        }
        do {
            let held = self.priority
            if let v = held {
                out.property("PRIORITY")
                out.int(Int64(v))
            }
        }
        do {
            let held = self.allDay
            if let v = held {
                out.property("X-ALL-DAY")
                out.boolean(v)
            }
        }
        out.finish()
        try sink.writeBytes(out.bytes)
    }

    /// Decodes one component at the cursor and moves the cursor past its `END:` line. The component
    /// is read from a copy of the input, and the cursor moves only when it was read whole: a
    /// component that is short answers `needMoreBytes`, and any other refusal names the rule that
    /// refused it; either leaves the cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecContentLineEvent {
        let rest = Array(try cursor.peekSlice(cursor.remaining))
        var reader = try ContentLineReader.begin(rest, component: "VEVENT")
        var uidValue: String? = nil
        var dtstartValue: String? = nil
        var dtstartTzidRead: String? = nil
        var summaryValue: String? = nil
        var descriptionValue: String? = nil
        var organizerValue: String? = nil
        var organizerCnRead: String? = nil
        var rruleValue: String? = nil
        var exdateValues: [String] = []
        var sequenceValue: UInt16? = nil
        var priorityValue: Int32? = nil
        var allDayValue: Bool? = nil
        while var line = try reader.next() {
            if line.isNamed("UID") {
                if uidValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 24, text: false)
                uidValue = v
            } else if line.isNamed("DTSTART") {
                if dtstartValue != nil { throw CodecError.lineTooMany }
                while try line.nextParam() {
                    if line.paramNamed("TZID") {
                        if dtstartTzidRead != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 24)
                        dtstartTzidRead = param
                    }
                }
                let v = try line.readString(maxSize: 20, text: false)
                dtstartValue = v
            } else if line.isNamed("SUMMARY") {
                if summaryValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 24, text: true)
                summaryValue = v
            } else if line.isNamed("DESCRIPTION") {
                if descriptionValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 160, text: true)
                descriptionValue = v
            } else if line.isNamed("ORGANIZER") {
                if organizerValue != nil { throw CodecError.lineTooMany }
                while try line.nextParam() {
                    if line.paramNamed("CN") {
                        if organizerCnRead != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 24)
                        organizerCnRead = param
                    }
                }
                let v = try line.readString(maxSize: 40, text: false)
                organizerValue = v
            } else if line.isNamed("RRULE") {
                if rruleValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 40, text: false)
                rruleValue = v
            } else if line.isNamed("EXDATE") {
                if exdateValues.count >= 3 { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 20, text: false)
                exdateValues.append(v)
            } else if line.isNamed("SEQUENCE") {
                if sequenceValue != nil { throw CodecError.lineTooMany }
                let v = UInt16(truncatingIfNeeded: try line.readUint(max: UInt64(UInt16.max)))
                sequenceValue = v
            } else if line.isNamed("PRIORITY") {
                if priorityValue != nil { throw CodecError.lineTooMany }
                let v = Int32(truncatingIfNeeded: try line.readInt(min: Int64(Int32.min), max: Int64(Int32.max)))
                priorityValue = v
            } else if line.isNamed("X-ALL-DAY") {
                if allDayValue != nil { throw CodecError.lineTooMany }
                let v = try line.readBool()
                allDayValue = v
            }
        }
        var frame = CodecContentLineEvent()
        guard let uidFinal = uidValue else { throw CodecError.lineRequiredMissing }
        frame.uid = uidFinal
        guard let dtstartFinal = dtstartValue else { throw CodecError.lineRequiredMissing }
        frame.dtstart = dtstartFinal
        guard let dtstartTzidFinal = dtstartTzidRead else { throw CodecError.lineRequiredMissing }
        frame.dtstartTzid = dtstartTzidFinal
        frame.summary = summaryValue
        frame.description = descriptionValue
        frame.organizer = organizerValue
        frame.organizerCn = organizerCnRead
        frame.rrule = rruleValue
        frame.exdate = exdateValues
        frame.sequence = sequenceValue
        frame.priority = priorityValue
        frame.allDay = allDayValue
        // Only a component that decoded whole moves the cursor.
        try cursor.advance(reader.consumed)
        return frame
    }
}
