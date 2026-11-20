// SCE-GENERATED — DO NOT EDIT
// source-hash: baff1d849badab60ce543e56f1d0bea450368449d0cbe8843444eac686ae9143
// source: codec_zenoh_declaration.scxml
// SCE-MAP: codec_zenoh_declaration.scxml:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohDeclaration`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohDeclarationArms<A0: CodecZenohDeclKexprSource, A1: CodecZenohUndeclKexprSource, A2: CodecZenohDeclSubscriberSource, A3: CodecZenohUndeclSubscriberSource, A4: CodecZenohDeclQueryableSource, A5: CodecZenohUndeclQueryableSource, A6: CodecZenohDeclTokenSource, A7: CodecZenohUndeclTokenSource, A8: CodecZenohDeclFinalSource, D: CodecZenohDeclFinalSource> {
    case codecZenohDeclKexpr(A0)
    case codecZenohUndeclKexpr(A1)
    case codecZenohDeclSubscriber(A2)
    case codecZenohUndeclSubscriber(A3)
    case codecZenohDeclQueryable(A4)
    case codecZenohUndeclQueryable(A5)
    case codecZenohDeclToken(A6)
    case codecZenohUndeclToken(A7)
    case codecZenohDeclFinal(A8)
    case `default`(tag: CodecZenohDeclarationUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohDeclKexpr: return 0x0
        case .codecZenohUndeclKexpr: return 0x1
        case .codecZenohDeclSubscriber: return 0x2
        case .codecZenohUndeclSubscriber: return 0x3
        case .codecZenohDeclQueryable: return 0x4
        case .codecZenohUndeclQueryable: return 0x5
        case .codecZenohDeclToken: return 0x6
        case .codecZenohUndeclToken: return 0x7
        case .codecZenohDeclFinal: return 0x1A
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohDeclarationArms: Equatable where A0: Equatable, A1: Equatable, A2: Equatable, A3: Equatable, A4: Equatable, A5: Equatable, A6: Equatable, A7: Equatable, A8: Equatable, D: Equatable {}

extension CodecZenohDeclarationArms: Sendable where A0: Sendable, A1: Sendable, A2: Sendable, A3: Sendable, A4: Sendable, A5: Sendable, A6: Sendable, A7: Sendable, A8: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohDeclaration` holds, each arm the value of its own codec.
public typealias CodecZenohDeclarationVariant = CodecZenohDeclarationArms<CodecZenohDeclKexpr, CodecZenohUndeclKexpr, CodecZenohDeclSubscriber, CodecZenohUndeclSubscriber, CodecZenohDeclQueryable, CodecZenohUndeclQueryable, CodecZenohDeclToken, CodecZenohUndeclToken, CodecZenohDeclFinal, CodecZenohDeclFinal>

/// The tag the catch-all arm of `CodecZenohDeclaration` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohDeclarationUnknownTag: Equatable, Hashable, Sendable {
    /// The tag.
    public let value: UInt8

    /// The tag `value`, or `nil` where an arm names it or its bits do not hold it.
    public init?(_ value: UInt8) {
        guard Self.admits(value) else { return nil }
        self.value = value
    }

    /// A tag the decode read where no arm names one, or the least one the catch-all admits.
    fileprivate init(admitted value: UInt8) {
        self.value = value
    }

    /// Whether the catch-all may hold `value`.
    public static func admits(_ value: UInt8) -> Bool {
        switch value {
        case 0x0...0x7, 0x1A: return false
        default: return value <= 0x1F
        }
    }
}

/// The elements a value of `CodecZenohDeclaration` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohDeclaration.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohDeclaration`, which conforms itself.
public protocol CodecZenohDeclarationSource {
    /// What the body of the arm `codecZenohDeclKexpr` is presented as: a source of `CodecZenohDeclKexpr`, written by its encode.
    associatedtype CodecZenohDeclKexpr_Arm: CodecZenohDeclKexprSource
    /// What the body of the arm `codecZenohUndeclKexpr` is presented as: a source of `CodecZenohUndeclKexpr`, written by its encode.
    associatedtype CodecZenohUndeclKexpr_Arm: CodecZenohUndeclKexprSource
    /// What the body of the arm `codecZenohDeclSubscriber` is presented as: a source of `CodecZenohDeclSubscriber`, written by its encode.
    associatedtype CodecZenohDeclSubscriber_Arm: CodecZenohDeclSubscriberSource
    /// What the body of the arm `codecZenohUndeclSubscriber` is presented as: a source of `CodecZenohUndeclSubscriber`, written by its encode.
    associatedtype CodecZenohUndeclSubscriber_Arm: CodecZenohUndeclSubscriberSource
    /// What the body of the arm `codecZenohDeclQueryable` is presented as: a source of `CodecZenohDeclQueryable`, written by its encode.
    associatedtype CodecZenohDeclQueryable_Arm: CodecZenohDeclQueryableSource
    /// What the body of the arm `codecZenohUndeclQueryable` is presented as: a source of `CodecZenohUndeclQueryable`, written by its encode.
    associatedtype CodecZenohUndeclQueryable_Arm: CodecZenohUndeclQueryableSource
    /// What the body of the arm `codecZenohDeclToken` is presented as: a source of `CodecZenohDeclToken`, written by its encode.
    associatedtype CodecZenohDeclToken_Arm: CodecZenohDeclTokenSource
    /// What the body of the arm `codecZenohUndeclToken` is presented as: a source of `CodecZenohUndeclToken`, written by its encode.
    associatedtype CodecZenohUndeclToken_Arm: CodecZenohUndeclTokenSource
    /// What the body of the arm `codecZenohDeclFinal` is presented as: a source of `CodecZenohDeclFinal`, written by its encode.
    associatedtype CodecZenohDeclFinal_Arm: CodecZenohDeclFinalSource
    /// What the body of the arm `codecZenohDeclFinal` is presented as: a source of `CodecZenohDeclFinal`, written by its encode.
    associatedtype Default_Arm: CodecZenohDeclFinalSource
    var header: UInt8 { get }
    var body: CodecZenohDeclarationArms<CodecZenohDeclKexpr_Arm, CodecZenohUndeclKexpr_Arm, CodecZenohDeclSubscriber_Arm, CodecZenohUndeclSubscriber_Arm, CodecZenohDeclQueryable_Arm, CodecZenohUndeclQueryable_Arm, CodecZenohDeclToken_Arm, CodecZenohUndeclToken_Arm, CodecZenohDeclFinal_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclaration: Equatable, Sendable, CodecZenohDeclarationSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecZenohDeclKexpr.frameLength, CodecZenohUndeclKexpr.frameLength, CodecZenohDeclSubscriber.frameLength, CodecZenohUndeclSubscriber.frameLength, CodecZenohDeclQueryable.frameLength, CodecZenohUndeclQueryable.frameLength, CodecZenohDeclToken.frameLength, CodecZenohUndeclToken.frameLength, CodecZenohDeclFinal.frameLength)

    /// The value presents the body of the arm `codecZenohDeclKexpr` as a value of its own codec.
    public typealias CodecZenohDeclKexpr_Arm = CodecZenohDeclKexpr

    /// The value presents the body of the arm `codecZenohUndeclKexpr` as a value of its own codec.
    public typealias CodecZenohUndeclKexpr_Arm = CodecZenohUndeclKexpr

    /// The value presents the body of the arm `codecZenohDeclSubscriber` as a value of its own codec.
    public typealias CodecZenohDeclSubscriber_Arm = CodecZenohDeclSubscriber

    /// The value presents the body of the arm `codecZenohUndeclSubscriber` as a value of its own codec.
    public typealias CodecZenohUndeclSubscriber_Arm = CodecZenohUndeclSubscriber

    /// The value presents the body of the arm `codecZenohDeclQueryable` as a value of its own codec.
    public typealias CodecZenohDeclQueryable_Arm = CodecZenohDeclQueryable

    /// The value presents the body of the arm `codecZenohUndeclQueryable` as a value of its own codec.
    public typealias CodecZenohUndeclQueryable_Arm = CodecZenohUndeclQueryable

    /// The value presents the body of the arm `codecZenohDeclToken` as a value of its own codec.
    public typealias CodecZenohDeclToken_Arm = CodecZenohDeclToken

    /// The value presents the body of the arm `codecZenohUndeclToken` as a value of its own codec.
    public typealias CodecZenohUndeclToken_Arm = CodecZenohUndeclToken

    /// The value presents the body of the arm `codecZenohDeclFinal` as a value of its own codec.
    public typealias CodecZenohDeclFinal_Arm = CodecZenohDeclFinal

    /// The value presents the body of the arm `codecZenohDeclFinal` as a value of its own codec.
    public typealias Default_Arm = CodecZenohDeclFinal

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `N`).
    public var N: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohDeclarationVariant = .codecZenohDeclFinal(CodecZenohDeclFinal())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecZenohDeclarationArms<CodecZenohDeclKexpr_Arm, CodecZenohUndeclKexpr_Arm, CodecZenohDeclSubscriber_Arm, CodecZenohUndeclSubscriber_Arm, CodecZenohDeclQueryable_Arm, CodecZenohUndeclQueryable_Arm, CodecZenohDeclToken_Arm, CodecZenohUndeclToken_Arm, CodecZenohDeclFinal_Arm, Default_Arm>) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohDeclaration {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let body: CodecZenohDeclarationVariant
        let body_tag = (header >> 0) & 0x1F
        switch body_tag {
        case 0x0:
            body = .codecZenohDeclKexpr(try CodecZenohDeclKexpr.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1)))
        case 0x1:
            body = .codecZenohUndeclKexpr(try CodecZenohUndeclKexpr.decode(&reader))
        case 0x2:
            body = .codecZenohDeclSubscriber(try CodecZenohDeclSubscriber.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1)))
        case 0x3:
            body = .codecZenohUndeclSubscriber(try CodecZenohUndeclSubscriber.decode(&reader, Z: UInt8(truncatingIfNeeded: (header >> 7) & 0x1)))
        case 0x4:
            body = .codecZenohDeclQueryable(try CodecZenohDeclQueryable.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1), Z: UInt8(truncatingIfNeeded: (header >> 7) & 0x1)))
        case 0x5:
            body = .codecZenohUndeclQueryable(try CodecZenohUndeclQueryable.decode(&reader, Z: UInt8(truncatingIfNeeded: (header >> 7) & 0x1)))
        case 0x6:
            body = .codecZenohDeclToken(try CodecZenohDeclToken.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1)))
        case 0x7:
            body = .codecZenohUndeclToken(try CodecZenohUndeclToken.decode(&reader, Z: UInt8(truncatingIfNeeded: (header >> 7) & 0x1)))
        case 0x1A:
            body = .codecZenohDeclFinal(try CodecZenohDeclFinal.decode(&reader))
        default:
            body = .default(tag: CodecZenohDeclarationUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohDeclFinal.decode(&reader))
        }
        cursor = reader
        return CodecZenohDeclaration(header: header, body: body)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohDeclaration` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `M` flag of `header`, where a parent sets it.
        public var headerM: Bool?

        public init(headerM: Bool? = nil) {
            self.headerM = headerM
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohDeclarationSource` is written,
    /// with no copy into `CodecZenohDeclaration`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohDeclarationSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_N_derived_said4: Int? = nil
        if case .codecZenohDeclKexpr(let arm) = source.body { header_N_derived_said4 = Int(CodecZenohDeclKexpr.inputN(arm)) }
        if case .codecZenohDeclSubscriber(let arm) = source.body { header_N_derived_said4 = Int(CodecZenohDeclSubscriber.inputN(arm)) }
        if case .codecZenohDeclQueryable(let arm) = source.body { header_N_derived_said4 = Int(CodecZenohDeclQueryable.inputN(arm)) }
        if case .codecZenohDeclToken(let arm) = source.body { header_N_derived_said4 = Int(CodecZenohDeclToken.inputN(arm)) }
        var header_folded = source.header
        if header_N_derived_said4 != nil { header_folded = (header_folded & ~0x20) | ((UInt8(truncatingIfNeeded: (header_N_derived_said4 ?? 0)) & 0x1) << 5) }
        var header_Z_derived_said4: Int? = nil
        if case .codecZenohUndeclSubscriber(let arm) = source.body { header_Z_derived_said4 = Int(CodecZenohUndeclSubscriber.inputZ(arm)) }
        if case .codecZenohDeclQueryable(let arm) = source.body { header_Z_derived_said4 = Int(CodecZenohDeclQueryable.inputZ(arm)) }
        if case .codecZenohUndeclQueryable(let arm) = source.body { header_Z_derived_said4 = Int(CodecZenohUndeclQueryable.inputZ(arm)) }
        if case .codecZenohUndeclToken(let arm) = source.body { header_Z_derived_said4 = Int(CodecZenohUndeclToken.inputZ(arm)) }
        if header_Z_derived_said4 != nil { header_folded = (header_folded & ~0x80) | ((UInt8(truncatingIfNeeded: (header_Z_derived_said4 ?? 0)) & 0x1) << 7) }
        header_folded = (header_folded & ~0x1F) | ((UInt8(truncatingIfNeeded: source.body.tag) & 0x1F) << 0)
        if let set = overrides.headerM { header_folded = set ? (header_folded | 0x40) : (header_folded & ~0x40) }
        var frame = [UInt8](repeating: 0, count: 1)
        storeInteger(header_folded, into: &frame, at: 0, .big)
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohDeclKexpr(let arm):
            try CodecZenohDeclKexpr.encodeSource(arm, into: &body_arm)
        case .codecZenohUndeclKexpr(let arm):
            try CodecZenohUndeclKexpr.encodeSource(arm, into: &body_arm)
        case .codecZenohDeclSubscriber(let arm):
            try CodecZenohDeclSubscriber.encodeSource(arm, into: &body_arm)
        case .codecZenohUndeclSubscriber(let arm):
            try CodecZenohUndeclSubscriber.encodeSource(arm, into: &body_arm)
        case .codecZenohDeclQueryable(let arm):
            try CodecZenohDeclQueryable.encodeSource(arm, into: &body_arm)
        case .codecZenohUndeclQueryable(let arm):
            try CodecZenohUndeclQueryable.encodeSource(arm, into: &body_arm)
        case .codecZenohDeclToken(let arm):
            try CodecZenohDeclToken.encodeSource(arm, into: &body_arm)
        case .codecZenohUndeclToken(let arm):
            try CodecZenohUndeclToken.encodeSource(arm, into: &body_arm)
        case .codecZenohDeclFinal(let arm):
            try CodecZenohDeclFinal.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohDeclFinal.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
