// SCE-GENERATED — DO NOT EDIT
// source-hash: 39ada52555aee5503dbb3d1d37300c230416e1974d0d6066365187651f7d002d
// source: codec_init_cookie_body.scxml
// SCE-MAP: codec_init_cookie_body.scxml:36 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecInitCookieBody` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecInitCookieBody.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecInitCookieBody`, which conforms itself.
public protocol CodecInitCookieBodySource {
    var version: UInt8 { get }
    var cookie: BoundedBytes<CodecInitCookieBody.CookieBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecInitCookieBody: Equatable, Sendable, CodecInitCookieBodySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `cookie` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum CookieBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 64
    }

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// On the wire only when `(A & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `cookie_size` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var cookie: BoundedBytes<CodecInitCookieBody.CookieBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cookie: BoundedBytes<CodecInitCookieBody.CookieBound>?) {
        self.version = version
        self.cookie = cookie
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, A: UInt8) throws(CodecError) -> CodecInitCookieBody {
        var reader = cursor
        let version = try reader.readInteger(UInt8.self, .big)
        var cookie_size: UInt16? = nil
        if (A & 0x1) != 0 {
            cookie_size = try reader.readVle(UInt16.self)
        }
        var cookie: BoundedBytes<CodecInitCookieBody.CookieBound>? = nil
        if (A & 0x1) != 0 {
            if (cookie_size ?? 0) > 64 {
                throw CodecError.lengthOutOfRange
            }
            cookie = try BoundedBytes<CodecInitCookieBody.CookieBound>(decoding: Array(try reader.readSlice(Int(clamping: (cookie_size ?? 0)))))
        }
        cursor = reader
        return CodecInitCookieBody(version: version, cookie: cookie)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecInitCookieBodySource` is written,
    /// with no copy into `CodecInitCookieBody`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecInitCookieBodySource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let A: UInt8 = source.cookie != nil ? 1 : 0
        let cookie_size_derived: UInt16 = source.cookie != nil ? UInt16((source.cookie?.byteCount ?? 0)) : 0
        var frame: [UInt8] = []
        appendInteger(source.version, to: &frame, .big)
        if A != 0 {
            appendVle(cookie_size_derived, to: &frame)
        }
        if let held = source.cookie {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `A` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputA<Source: CodecInitCookieBodySource>(_ source: Source) -> UInt8 {
        source.cookie != nil ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
