// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_content_line_enums.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="content-line")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

private let TEXTS_lifecycle = EnumTexts([
    EnumText("NEEDS-ACTION", 1),
    EnumText("IN-PROCESS", 2),
    EnumText("COMPLETED", 4),
    EnumText("cancelled", 8),
])

private let TEXTS_access = EnumTexts([
    EnumText("PUBLIC", 0),
    EnumText("PRIVATE", 1),
    EnumText("CONFIDENTIAL", 2),
])

private let TEXTS_role = EnumTexts([
    EnumText("CHAIR", 10),
    EnumText("REQ-PARTICIPANT", 300),
    EnumText("NON-PARTICIPANT", 4000),
])

/// One line of `ATTENDEE` (docs/adr/0014): the parameters it carries and its value. A parameter that may be absent is `nil`.
public struct CodecContentLineEnumsAttendeeLine: Equatable, Sendable {
    /// Parameter `ROLE`, required.
    public var attendeeRole: EnumContentLineRole = EnumContentLineRole.chair
    /// Parameter `PARTSTAT`.
    public var attendeeStatus: EnumContentLineStatus? = nil
    /// The value of the line.
    public var value: String = ""

    public init() {}

    public init(attendeeRole: EnumContentLineRole, attendeeStatus: EnumContentLineStatus?, value: String) {
        self.attendeeRole = attendeeRole
        self.attendeeStatus = attendeeStatus
        self.value = value
    }
}

/// One line of `X-STEP` (docs/adr/0014): the parameters it carries and its value. A parameter that may be absent is `nil`.
public struct CodecContentLineEnumsStepLine: Equatable, Sendable {
    /// Parameter `NOTE`.
    public var stepNote: String? = nil
    /// The value of the line.
    public var value: EnumContentLineStatus = EnumContentLineStatus.needsAction

    public init() {}

    public init(stepNote: String?, value: EnumContentLineStatus) {
        self.stepNote = stepNote
        self.value = value
    }
}

/// One `VEVENT` component of content lines (SCE_FORGE.md §4.6.4). Decode reads the properties
/// this codec declares in any order and skips the rest; encode writes the entries present in
/// declaration order, folded at 75 octets. A required entry starts at its type's default and an
/// optional one absent (`nil`), so `CodecContentLineEnums()` is the infallible initializer every codec
/// offers. A repeated property that declares a parameter, or whose line holds a list of values, is a
/// list of line records.
public struct CodecContentLineEnums: Equatable, Sendable {
    /// Property `UID`, required.
    public var uid: String = ""
    /// Property `STATUS`.
    public var status: EnumContentLineStatus? = nil
    /// Property `CLASS`, required.
    public var clazz: EnumContentLineClass = EnumContentLineClass.unclassified
    /// Property `X-HISTORY`, up to 3 lines.
    public var history: [EnumContentLineStatus] = []
    /// Property `ATTENDEE`, up to 3 lines, each a record.
    public var attendee: [CodecContentLineEnumsAttendeeLine] = []
    /// Property `X-STEP`, up to 2 lines, each a record.
    public var step: [CodecContentLineEnumsStepLine] = []
    /// Property `COMMENT`.
    public var note: String? = nil

    public init() {}

    public init(uid: String, status: EnumContentLineStatus?, clazz: EnumContentLineClass, history: [EnumContentLineStatus], attendee: [CodecContentLineEnumsAttendeeLine], step: [CodecContentLineEnumsStepLine], note: String?) {
        self.uid = uid
        self.status = status
        self.clazz = clazz
        self.history = history
        self.attendee = attendee
        self.step = step
        self.note = note
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
            try out.string(v, text: true, maxSize: 16)
        }
        do {
            let held = self.status
            if let v = held {
                out.property("STATUS")
                try out.enumValue(TEXTS_lifecycle, carrier: UInt64(truncatingIfNeeded: EnumContentLineStatus.toUnderlying(v)))
            }
        }
        do {
            let v = self.clazz
            out.property("CLASS")
            try out.enumValue(TEXTS_access, carrier: UInt64(truncatingIfNeeded: EnumContentLineClass.toUnderlying(v)))
        }
        if self.history.count > 3 { throw CodecError.lineTooMany }
        for v in self.history {
            out.property("X-HISTORY")
            try out.enumValue(TEXTS_lifecycle, carrier: UInt64(truncatingIfNeeded: EnumContentLineStatus.toUnderlying(v)))
        }
        if self.attendee.count > 3 { throw CodecError.lineTooMany }
        for record in self.attendee {
            let v = record.value
            out.property("ATTENDEE")
            do {
                let pv = record.attendeeRole
                try out.enumParam("ROLE", TEXTS_role, carrier: UInt64(truncatingIfNeeded: EnumContentLineRole.toUnderlying(pv)))
            }
            if let attendeeStatusValue = record.attendeeStatus {
                do {
                    let pv = attendeeStatusValue
                    try out.enumParam("PARTSTAT", TEXTS_lifecycle, carrier: UInt64(truncatingIfNeeded: EnumContentLineStatus.toUnderlying(pv)))
                }
            }
            try out.string(v, text: false, maxSize: 24)
        }
        if self.step.count > 2 { throw CodecError.lineTooMany }
        for record in self.step {
            let v = record.value
            out.property("X-STEP")
            if let stepNoteValue = record.stepNote {
                try out.param("NOTE", stepNoteValue, maxSize: 16)
            }
            try out.enumValue(TEXTS_lifecycle, carrier: UInt64(truncatingIfNeeded: EnumContentLineStatus.toUnderlying(v)))
        }
        do {
            let held = self.note
            if let v = held {
                out.property("COMMENT")
                try out.string(v, text: true, maxSize: 120)
            }
        }
        out.finish()
        try sink.writeBytes(out.bytes)
    }

    /// Decodes one component at the cursor and moves the cursor past its `END:` line. The component
    /// is read from a copy of the input, and the cursor moves only when it was read whole: a
    /// component that is short answers `needMoreBytes`, and any other refusal names the rule that
    /// refused it; either leaves the cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecContentLineEnums {
        let rest = Array(try cursor.peekSlice(cursor.remaining))
        var reader = try ContentLineReader.begin(rest, component: "VEVENT")
        var uidValue: String? = nil
        var statusValue: EnumContentLineStatus? = nil
        var clazzValue: EnumContentLineClass? = nil
        var historyValues: [EnumContentLineStatus] = []
        var attendeeValues: [CodecContentLineEnumsAttendeeLine] = []
        var stepValues: [CodecContentLineEnumsStepLine] = []
        var noteValue: String? = nil
        while var line = try reader.next() {
            if line.isNamed("UID") {
                if uidValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 16, text: true)
                uidValue = v
            } else if line.isNamed("STATUS") {
                if statusValue != nil { throw CodecError.lineTooMany }
                guard let v = EnumContentLineStatus.fromUnderlying(UInt8(truncatingIfNeeded: try line.readEnum(TEXTS_lifecycle))) else {
                    throw CodecError.lineBadValue
                }
                statusValue = v
            } else if line.isNamed("CLASS") {
                if clazzValue != nil { throw CodecError.lineTooMany }
                let v = EnumContentLineClass.fromUnderlying(UInt8(truncatingIfNeeded: try line.readEnum(TEXTS_access)))
                clazzValue = v
            } else if line.isNamed("X-HISTORY") {
                if historyValues.count >= 3 { throw CodecError.lineTooMany }
                guard let v = EnumContentLineStatus.fromUnderlying(UInt8(truncatingIfNeeded: try line.readEnum(TEXTS_lifecycle))) else {
                    throw CodecError.lineBadValue
                }
                historyValues.append(v)
            } else if line.isNamed("ATTENDEE") {
                if attendeeValues.count >= 3 { throw CodecError.lineTooMany }
                var attendeeRoleLine: EnumContentLineRole? = nil
                var attendeeStatusLine: EnumContentLineStatus? = nil
                while try line.nextParam() {
                    if line.paramNamed("ROLE") {
                        if attendeeRoleLine != nil { throw CodecError.lineTooMany }
                        guard let param = EnumContentLineRole.fromUnderlying(UInt16(truncatingIfNeeded: try line.readParamEnum(TEXTS_role))) else {
                            throw CodecError.lineBadValue
                        }
                        attendeeRoleLine = param
                    } else if line.paramNamed("PARTSTAT") {
                        if attendeeStatusLine != nil { throw CodecError.lineTooMany }
                        guard let param = EnumContentLineStatus.fromUnderlying(UInt8(truncatingIfNeeded: try line.readParamEnum(TEXTS_lifecycle))) else {
                            throw CodecError.lineBadValue
                        }
                        attendeeStatusLine = param
                    }
                }
                let v = try line.readString(maxSize: 24, text: false)
                guard let attendeeRoleFinal = attendeeRoleLine else { throw CodecError.lineRequiredMissing }
                attendeeValues.append(CodecContentLineEnumsAttendeeLine(attendeeRole: attendeeRoleFinal, attendeeStatus: attendeeStatusLine, value: v))
            } else if line.isNamed("X-STEP") {
                if stepValues.count >= 2 { throw CodecError.lineTooMany }
                var stepNoteLine: String? = nil
                while try line.nextParam() {
                    if line.paramNamed("NOTE") {
                        if stepNoteLine != nil { throw CodecError.lineTooMany }
                        let param = try line.readParamString(maxSize: 16)
                        stepNoteLine = param
                    }
                }
                guard let v = EnumContentLineStatus.fromUnderlying(UInt8(truncatingIfNeeded: try line.readEnum(TEXTS_lifecycle))) else {
                    throw CodecError.lineBadValue
                }
                stepValues.append(CodecContentLineEnumsStepLine(stepNote: stepNoteLine, value: v))
            } else if line.isNamed("COMMENT") {
                if noteValue != nil { throw CodecError.lineTooMany }
                let v = try line.readString(maxSize: 120, text: true)
                noteValue = v
            }
        }
        var frame = CodecContentLineEnums()
        guard let uidFinal = uidValue else { throw CodecError.lineRequiredMissing }
        frame.uid = uidFinal
        frame.status = statusValue
        guard let clazzFinal = clazzValue else { throw CodecError.lineRequiredMissing }
        frame.clazz = clazzFinal
        frame.history = historyValues
        frame.attendee = attendeeValues
        frame.step = stepValues
        frame.note = noteValue
        // Only a component that decoded whole moves the cursor.
        try cursor.advance(reader.consumed)
        return frame
    }
}
