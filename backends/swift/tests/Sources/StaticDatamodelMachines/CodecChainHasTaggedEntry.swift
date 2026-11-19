// SCE-GENERATED — DO NOT EDIT
// source-hash: 057a65a147fe2e656cbfb14939ef95a41c7c3d239d2f87620515e42f908c4239
// source: codec_chain_has_tagged_entry.scxml
// SCE-MAP: codec_chain_has_tagged_entry.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainHasTaggedEntry` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasTaggedEntry.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasTaggedEntry`, which conforms itself.
public protocol CodecChainHasTaggedEntrySource {
    var entry_type: UInt8 { get }
    var ctl: UInt8 { get }
    var body: UInt8 { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasTaggedEntry: Equatable, Sendable, CodecChainHasTaggedEntrySource {
    /// The bytes of one frame.
    public static let frameLength = 3

    /// The field at byte 0, big-endian.
    public var entry_type: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var ctl: UInt8 = 0

    /// Bit 0 of `ctl` (flag `more`).
    public var more: Bool {
        get { (ctl & 0x1) != 0 }
        set { ctl = newValue ? (ctl | 0x1) : (ctl & ~0x1) }
    }

    /// The field at byte 2, big-endian.
    public var body: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(entry_type: UInt8, ctl: UInt8, body: UInt8) {
        self.entry_type = entry_type
        self.ctl = ctl
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasTaggedEntry {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecChainHasTaggedEntry(
            entry_type: try cursor.peekInteger(UInt8.self, at: 0, .big),
            ctl: try cursor.peekInteger(UInt8.self, at: 1, .big),
            body: try cursor.peekInteger(UInt8.self, at: 2, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecChainHasTaggedEntry` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `more` flag of `ctl`, where a parent sets it.
        public var ctlMore: Bool?

        public init(ctlMore: Bool? = nil) {
            self.ctlMore = ctlMore
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasTaggedEntrySource` is written,
    /// with no copy into `CodecChainHasTaggedEntry`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasTaggedEntrySource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var ctl_folded = source.ctl
        if let set = overrides.ctlMore { ctl_folded = set ? (ctl_folded | 0x1) : (ctl_folded & ~0x1) }
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(source.entry_type, into: &frame, at: 0, .big)
        storeInteger(ctl_folded, into: &frame, at: 1, .big)
        storeInteger(source.body, into: &frame, at: 2, .big)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
