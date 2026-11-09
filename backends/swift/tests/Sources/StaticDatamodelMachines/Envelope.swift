// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: envelope.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and skips one this codec
/// does not declare; encode writes the entries present in ascending key order, every head in its
/// shortest form. A required entry starts at its type's zero and an optional one absent (`nil`), so
/// `Envelope()` is the infallible initializer every codec offers.
public struct Envelope: Equatable, Sendable {
    /// Map key 0, required.
    public var id: [UInt8] = []

    /// Map key 1, required.
    public var source: String = ""

    /// Map key 2, required.
    public var event_type: String = ""

    /// Map key 3, required.
    public var pattern: PatternKind = PatternKind.fireForget

    /// Map key 4, required.
    public var datacontenttype: PayloadCodec = PayloadCodec.none

    /// Map key 5, required.
    public var data: [UInt8] = []

    /// Map key 6.
    public var subject: String? = nil

    /// Map key 7.
    public var correlation_id: [UInt8]? = nil

    /// Map key 8.
    public var reply_to: String? = nil

    /// Map key 9.
    public var invoke_id: [UInt8]? = nil

    /// Map key 10.
    public var rpc_status: RpcStatus? = nil

    /// Map key 11.
    public var rpc_error_message: String? = nil

    /// Map key 12.
    public var deadline_unix_ms: UInt64? = nil

    /// Map key 14.
    public var sequence_no: UInt64? = nil

    /// Map key 15.
    public var routing_id: [UInt8]? = nil

    /// Map key 16.
    public var parallel_id: String? = nil

    /// Map key 17.
    public var region_id: String? = nil

    /// Map key 18.
    public var child_session_id: String? = nil

    /// Map key 19.
    public var causal_epoch: UInt64? = nil

    /// Map key 20.
    public var causal_vector: [CausalEntryPayload]? = nil

    public init() {}

    public init(id: [UInt8], source: String, event_type: String, pattern: PatternKind, datacontenttype: PayloadCodec, data: [UInt8], subject: String?, correlation_id: [UInt8]?, reply_to: String?, invoke_id: [UInt8]?, rpc_status: RpcStatus?, rpc_error_message: String?, deadline_unix_ms: UInt64?, sequence_no: UInt64?, routing_id: [UInt8]?, parallel_id: String?, region_id: String?, child_session_id: String?, causal_epoch: UInt64?, causal_vector: [CausalEntryPayload]?) {
        self.id = id
        self.source = source
        self.event_type = event_type
        self.pattern = pattern
        self.datacontenttype = datacontenttype
        self.data = data
        self.subject = subject
        self.correlation_id = correlation_id
        self.reply_to = reply_to
        self.invoke_id = invoke_id
        self.rpc_status = rpc_status
        self.rpc_error_message = rpc_error_message
        self.deadline_unix_ms = deadline_unix_ms
        self.sequence_no = sequence_no
        self.routing_id = routing_id
        self.parallel_id = parallel_id
        self.region_id = region_id
        self.child_session_id = child_session_id
        self.causal_epoch = causal_epoch
        self.causal_vector = causal_vector
    }

    /// Decodes one map at the cursor and moves the cursor past it. The map is read through a copy of
    /// the cursor, which replaces it only when the whole map was read: a map that is short answers
    /// `needMoreBytes`, one that is not a map this codec reads `cborMalformed`, and either leaves the
    /// cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> Envelope {
        var reader = cursor
        let count = try Cbor.readMapLen(&reader)
        var id_held: [UInt8]? = nil
        var source_held: String? = nil
        var event_type_held: String? = nil
        var pattern_held: PatternKind? = nil
        var datacontenttype_held: PayloadCodec? = nil
        var data_held: [UInt8]? = nil
        var subject_held: String? = nil
        var correlation_id_held: [UInt8]? = nil
        var reply_to_held: String? = nil
        var invoke_id_held: [UInt8]? = nil
        var rpc_status_held: RpcStatus? = nil
        var rpc_error_message_held: String? = nil
        var deadline_unix_ms_held: UInt64? = nil
        var sequence_no_held: UInt64? = nil
        var routing_id_held: [UInt8]? = nil
        var parallel_id_held: String? = nil
        var region_id_held: String? = nil
        var child_session_id_held: String? = nil
        var causal_epoch_held: UInt64? = nil
        var causal_vector_held: [CausalEntryPayload]? = nil
        var index: UInt64 = 0
        while index < count {
            index += 1
            switch try Cbor.readUint(&reader) {
            case 0:
                // A key given twice is not one map entry.
                if id_held != nil { throw CodecError.cborMalformed }
                id_held = try Cbor.readBytesExact(&reader, length: 16)
            case 1:
                // A key given twice is not one map entry.
                if source_held != nil { throw CodecError.cborMalformed }
                source_held = try Cbor.readText(&reader, maxSize: 262144)
            case 2:
                // A key given twice is not one map entry.
                if event_type_held != nil { throw CodecError.cborMalformed }
                event_type_held = try Cbor.readText(&reader, maxSize: 262144)
            case 3:
                // A key given twice is not one map entry.
                if pattern_held != nil { throw CodecError.cborMalformed }
                // A closed enum holds only the numbers it declares.
                guard let declared = PatternKind.fromUnderlying(UInt16(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 65535))) else {
                    throw CodecError.cborOutOfRange
                }
                pattern_held = declared
            case 4:
                // A key given twice is not one map entry.
                if datacontenttype_held != nil { throw CodecError.cborMalformed }
                // A closed enum holds only the numbers it declares.
                guard let declared = PayloadCodec.fromUnderlying(UInt8(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 255))) else {
                    throw CodecError.cborOutOfRange
                }
                datacontenttype_held = declared
            case 5:
                // A key given twice is not one map entry.
                if data_held != nil { throw CodecError.cborMalformed }
                data_held = try Cbor.readBytes(&reader, maxSize: 16777216)
            case 6:
                // A key given twice is not one map entry.
                if subject_held != nil { throw CodecError.cborMalformed }
                subject_held = try Cbor.readText(&reader, maxSize: 262144)
            case 7:
                // A key given twice is not one map entry.
                if correlation_id_held != nil { throw CodecError.cborMalformed }
                correlation_id_held = try Cbor.readBytesExact(&reader, length: 16)
            case 8:
                // A key given twice is not one map entry.
                if reply_to_held != nil { throw CodecError.cborMalformed }
                reply_to_held = try Cbor.readText(&reader, maxSize: 262144)
            case 9:
                // A key given twice is not one map entry.
                if invoke_id_held != nil { throw CodecError.cborMalformed }
                invoke_id_held = try Cbor.readBytesExact(&reader, length: 16)
            case 10:
                // A key given twice is not one map entry.
                if rpc_status_held != nil { throw CodecError.cborMalformed }
                // A closed enum holds only the numbers it declares.
                guard let declared = RpcStatus.fromUnderlying(UInt8(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 255))) else {
                    throw CodecError.cborOutOfRange
                }
                rpc_status_held = declared
            case 11:
                // A key given twice is not one map entry.
                if rpc_error_message_held != nil { throw CodecError.cborMalformed }
                rpc_error_message_held = try Cbor.readText(&reader, maxSize: 262144)
            case 12:
                // A key given twice is not one map entry.
                if deadline_unix_ms_held != nil { throw CodecError.cborMalformed }
                deadline_unix_ms_held = UInt64(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 18446744073709551615))
            case 14:
                // A key given twice is not one map entry.
                if sequence_no_held != nil { throw CodecError.cborMalformed }
                sequence_no_held = UInt64(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 18446744073709551615))
            case 15:
                // A key given twice is not one map entry.
                if routing_id_held != nil { throw CodecError.cborMalformed }
                routing_id_held = try Cbor.readBytesExact(&reader, length: 16)
            case 16:
                // A key given twice is not one map entry.
                if parallel_id_held != nil { throw CodecError.cborMalformed }
                parallel_id_held = try Cbor.readText(&reader, maxSize: 262144)
            case 17:
                // A key given twice is not one map entry.
                if region_id_held != nil { throw CodecError.cborMalformed }
                region_id_held = try Cbor.readText(&reader, maxSize: 262144)
            case 18:
                // A key given twice is not one map entry.
                if child_session_id_held != nil { throw CodecError.cborMalformed }
                child_session_id_held = try Cbor.readText(&reader, maxSize: 262144)
            case 19:
                // A key given twice is not one map entry.
                if causal_epoch_held != nil { throw CodecError.cborMalformed }
                causal_epoch_held = UInt64(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 18446744073709551615))
            case 20:
                // A key given twice is not one map entry.
                if causal_vector_held != nil { throw CodecError.cborMalformed }
                // A definite-length array within the list's counts, each item one record's map.
                let len = try Cbor.readArrayLen(&reader)
                if len > 64 { throw CodecError.cborOutOfRange }
                var list: [CausalEntryPayload] = []
                list.reserveCapacity(Int(len))
                var item: UInt64 = 0
                while item < len {
                    item += 1
                    list.append(try Envelope.decodeRecordCausalEntry(&reader))
                }
                causal_vector_held = list
            default:
                try Cbor.skip(&reader)
            }
        }
        var frame = Envelope()
        guard let id_value = id_held else { throw CodecError.cborRequiredKeyMissing }
        frame.id = id_value
        guard let source_value = source_held else { throw CodecError.cborRequiredKeyMissing }
        frame.source = source_value
        guard let event_type_value = event_type_held else { throw CodecError.cborRequiredKeyMissing }
        frame.event_type = event_type_value
        guard let pattern_value = pattern_held else { throw CodecError.cborRequiredKeyMissing }
        frame.pattern = pattern_value
        guard let datacontenttype_value = datacontenttype_held else { throw CodecError.cborRequiredKeyMissing }
        frame.datacontenttype = datacontenttype_value
        guard let data_value = data_held else { throw CodecError.cborRequiredKeyMissing }
        frame.data = data_value
        frame.subject = subject_held
        frame.correlation_id = correlation_id_held
        frame.reply_to = reply_to_held
        frame.invoke_id = invoke_id_held
        frame.rpc_status = rpc_status_held
        frame.rpc_error_message = rpc_error_message_held
        frame.deadline_unix_ms = deadline_unix_ms_held
        frame.sequence_no = sequence_no_held
        frame.routing_id = routing_id_held
        frame.parallel_id = parallel_id_held
        frame.region_id = region_id_held
        frame.child_session_id = child_session_id_held
        frame.causal_epoch = causal_epoch_held
        frame.causal_vector = causal_vector_held
        cursor = reader
        return frame
    }

    /// Writes the map to `sink` in one write: a value that breaks the bound its entry declares (an
    /// exact length, a maximum size) is refused and nothing is written, and a bounded sink that cannot
    /// hold all of it answers `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var count: UInt64 = 6
        if self.subject != nil { count += 1 }
        if self.correlation_id != nil { count += 1 }
        if self.reply_to != nil { count += 1 }
        if self.invoke_id != nil { count += 1 }
        if self.rpc_status != nil { count += 1 }
        if self.rpc_error_message != nil { count += 1 }
        if self.deadline_unix_ms != nil { count += 1 }
        if self.sequence_no != nil { count += 1 }
        if self.routing_id != nil { count += 1 }
        if self.parallel_id != nil { count += 1 }
        if self.region_id != nil { count += 1 }
        if self.child_session_id != nil { count += 1 }
        if self.causal_epoch != nil { count += 1 }
        if self.causal_vector != nil { count += 1 }
        Cbor.writeMapHead(count, to: &frame)
        do {
            let value = self.id
            Cbor.writeUint(0, to: &frame)
            if value.count != 16 { throw CodecError.cborWrongLength }
            Cbor.writeBytes(value, to: &frame)
        }
        do {
            let value = self.source
            Cbor.writeUint(1, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        do {
            let value = self.event_type
            Cbor.writeUint(2, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        do {
            let value = self.pattern
            Cbor.writeUint(3, to: &frame)
            Cbor.writeUint(UInt64(PatternKind.toUnderlying(value)), to: &frame)
        }
        do {
            let value = self.datacontenttype
            Cbor.writeUint(4, to: &frame)
            Cbor.writeUint(UInt64(PayloadCodec.toUnderlying(value)), to: &frame)
        }
        do {
            let value = self.data
            Cbor.writeUint(5, to: &frame)
            if value.count > 16777216 { throw CodecError.cborOutOfRange }
            Cbor.writeBytes(value, to: &frame)
        }
        if let value = self.subject {
            Cbor.writeUint(6, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        if let value = self.correlation_id {
            Cbor.writeUint(7, to: &frame)
            if value.count != 16 { throw CodecError.cborWrongLength }
            Cbor.writeBytes(value, to: &frame)
        }
        if let value = self.reply_to {
            Cbor.writeUint(8, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        if let value = self.invoke_id {
            Cbor.writeUint(9, to: &frame)
            if value.count != 16 { throw CodecError.cborWrongLength }
            Cbor.writeBytes(value, to: &frame)
        }
        if let value = self.rpc_status {
            Cbor.writeUint(10, to: &frame)
            Cbor.writeUint(UInt64(RpcStatus.toUnderlying(value)), to: &frame)
        }
        if let value = self.rpc_error_message {
            Cbor.writeUint(11, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        if let value = self.deadline_unix_ms {
            Cbor.writeUint(12, to: &frame)
            Cbor.writeUint(UInt64(value), to: &frame)
        }
        if let value = self.sequence_no {
            Cbor.writeUint(14, to: &frame)
            Cbor.writeUint(UInt64(value), to: &frame)
        }
        if let value = self.routing_id {
            Cbor.writeUint(15, to: &frame)
            if value.count != 16 { throw CodecError.cborWrongLength }
            Cbor.writeBytes(value, to: &frame)
        }
        if let value = self.parallel_id {
            Cbor.writeUint(16, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        if let value = self.region_id {
            Cbor.writeUint(17, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        if let value = self.child_session_id {
            Cbor.writeUint(18, to: &frame)
            if value.utf8.count > 262144 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        if let value = self.causal_epoch {
            Cbor.writeUint(19, to: &frame)
            Cbor.writeUint(UInt64(value), to: &frame)
        }
        if let value = self.causal_vector {
            Cbor.writeUint(20, to: &frame)
            if value.count > 64 { throw CodecError.cborOutOfRange }
            Cbor.writeArrayHead(UInt64(value.count), to: &frame)
            for record in value {
                try Envelope.encodeRecordCausalEntry(record, to: &frame)
            }
        }
        try sink.writeBytes(frame)
    }

    /// Reads one record of the event schema `causal_entry` (`record:CausalEntry`), as a list of this map
    /// holds it (docs/adr/0085, Landing order step 3a), into the payload type the schema's own unit
    /// declares: one CBOR map of every field, each under its id as text (docs/adr/0086, section 9.2). Its
    /// keys in any order, a key the schema does not declare skipped whole; a key given twice is
    /// `cborMalformed`, a field missing `cborRequiredKeyMissing`, a text or byte string past its bound
    /// `cborOutOfRange`.
    private static func decodeRecordCausalEntry(_ reader: inout SceCursor) throws(CodecError) -> CausalEntryPayload {
        let count = try Cbor.readMapLen(&reader)
        var hi_held: UInt64? = nil
        var lo_held: UInt64? = nil
        var seq_held: UInt64? = nil
        var index: UInt64 = 0
        while index < count {
            index += 1
            switch try Cbor.readText(&reader, maxSize: nil) {
            case "hi":
                // A key given twice is not one map entry.
                if hi_held != nil { throw CodecError.cborMalformed }
                hi_held = try Cbor.readUint(&reader)
            case "lo":
                // A key given twice is not one map entry.
                if lo_held != nil { throw CodecError.cborMalformed }
                lo_held = try Cbor.readUint(&reader)
            case "seq":
                // A key given twice is not one map entry.
                if seq_held != nil { throw CodecError.cborMalformed }
                seq_held = try Cbor.readUint(&reader)
            default:
                try Cbor.skip(&reader)
            }
        }
        guard let hi_value = hi_held else { throw CodecError.cborRequiredKeyMissing }
        guard let lo_value = lo_held else { throw CodecError.cborRequiredKeyMissing }
        guard let seq_value = seq_held else { throw CodecError.cborRequiredKeyMissing }
        return CausalEntryPayload(hi: hi_value, lo: lo_value, seq: seq_value)
    }

    /// Appends one record of `causal_entry` to `frame` as one map of every field, the keys in the order
    /// RFC 8949 §4.2.1 sorts their encodings: a shorter key first.
    private static func encodeRecordCausalEntry(_ record: CausalEntryPayload, to frame: inout [UInt8]) throws(CodecError) {
        Cbor.writeMapHead(3, to: &frame)
        Cbor.writeText("hi", to: &frame)
        Cbor.writeUint(UInt64(record.hi), to: &frame)
        Cbor.writeText("lo", to: &frame)
        Cbor.writeUint(UInt64(record.lo), to: &frame)
        Cbor.writeText("seq", to: &frame)
        Cbor.writeUint(UInt64(record.seq), to: &frame)
    }
}
