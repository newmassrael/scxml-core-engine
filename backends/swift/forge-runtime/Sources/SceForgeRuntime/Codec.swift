// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The cursor, the sink and the typed errors of code generated from `sce:kind="codec"` documents.
// Siblings: `codec.SceCursor`, `codec.SceSink` (Go, `backends/go/forge-runtime/codec/codec.go`) and
// `SceCursor`, `SceSink`, `CodecError` (Rust, `backends/rust/forge-runtime/src/codec.rs`). The
// contract is RFC §synth-5-B L494-519: a decode of a truncated input never aborts, it answers
// "need more bytes" and leaves the cursor where it was, and an encode into a bounded sink that is
// too small answers "buffer overflow" and writes nothing.
//
// Only what the Swift codec emitter can raise is declared. A failure is added with the feature that
// raises it (a variable-length integer, a string, an enum, a presence condition), so that no error
// here is one nothing can return.

/// What a generated codec refuses to do, as a value a caller can match.
public enum CodecError: Error, Equatable, Sendable {
    /// A decode found fewer bytes than the frame needs. The caller resumes after appending more.
    case needMoreBytes
    /// An encode would write past the capacity of a bounded sink.
    case bufferOverflow
    /// A variable-length integer's continuation chain names a value wider than the type that holds
    /// it (`codec/vle-width-overflow`).
    case vleWidthOverflow
    /// A byte string's length is outside the bound its field declares (`sce:max-size`, and what
    /// its length field can say after its `sce:length-arith`): on a decode, the length the frame
    /// says, judged before the frame's own length so that such a frame is refused and never waited
    /// on; on an encode, the length of the value, refused before the first byte is written.
    case lengthOutOfRange
    /// A list's count is past the bound its field declares (`sce:max-count`): on a decode, the
    /// count the frame says, or an entry past it in a list read to the end of the frame, refused
    /// before that entry is read; on an encode, the entries of the value, refused before the
    /// first byte is written.
    case tooManyElements
}

/// The byte order of one field on the wire.
public enum SceByteOrder: Sendable {
    case big
    case little
}

/// A read-only cursor over borrowed input. A decode peeks the bytes of the frame, builds the value,
/// and only then advances, so a failed decode has consumed nothing.
public struct SceCursor: Sendable {
    private let buffer: [UInt8]
    private var position: Int

    /// Wraps `buffer` with the cursor at its first byte.
    public init(_ buffer: [UInt8]) {
        self.buffer = buffer
        self.position = 0
    }

    /// The number of bytes from the cursor to the end of the input.
    public var remaining: Int { buffer.count - position }

    /// Borrows the next `count` bytes without moving the cursor.
    public func peekSlice(_ count: Int) throws(CodecError) -> ArraySlice<UInt8> {
        guard count >= 0, remaining >= count else {
            throw .needMoreBytes
        }
        return buffer[position..<position + count]
    }

    /// Moves the cursor `count` bytes forward.
    public mutating func advance(_ count: Int) throws(CodecError) {
        guard count >= 0, remaining >= count else {
            throw .needMoreBytes
        }
        position += count
    }

    /// The integer of type `T` at the cursor, which moves past it. A frame whose layout depends on
    /// what it holds (a variable-length integer, a count a field states) is read field by field
    /// through a copy of the cursor, and the cursor is replaced by the copy only when the whole frame
    /// was read, so a frame that is refused has consumed nothing.
    public mutating func readInteger<T: FixedWidthInteger>(
        _ type: T.Type, _ order: SceByteOrder
    ) throws(CodecError) -> T {
        let value = try peekInteger(type, at: 0, order)
        position += T.bitWidth / 8
        return value
    }

    /// The next `count` bytes, which the cursor moves past.
    public mutating func readSlice(_ count: Int) throws(CodecError) -> ArraySlice<UInt8> {
        let slice = try peekSlice(count)
        position += count
        return slice
    }

    /// A variable-length unsigned integer of type `T` at the cursor, which moves past it: base-128,
    /// least significant group first, the high bit of every byte but the last saying that another
    /// follows. The last byte a `T` can need carries a full 8 data bits and no flag, so a 64-bit
    /// value takes at most 9 bytes and not 10 (the canonical Zenoh ZInt; RFC §synth-5-B Appendix B).
    /// A chain that names a value wider than `T` answers `vleWidthOverflow`, one that ends before it
    /// is complete `needMoreBytes`.
    public mutating func readVle<T: FixedWidthInteger & UnsignedInteger>(
        _ type: T.Type
    ) throws(CodecError) -> T {
        let maxBits = T.bitWidth
        let length = (maxBits - 1 + 6) / 7
        let finalShift = 7 * (length - 1)
        var value: UInt64 = 0
        var shift = 0
        var index = 0
        while index < length {
            guard position + index < buffer.count else {
                throw .needMoreBytes
            }
            let byte = UInt64(buffer[position + index])
            index += 1
            if shift == finalShift {
                let allowed = maxBits - shift
                if allowed < 8 && byte > (UInt64(1) << UInt64(allowed)) - 1 {
                    throw .vleWidthOverflow
                }
                value |= byte << UInt64(shift)
                position += index
                return T(truncatingIfNeeded: value)
            }
            value |= (byte & 0x7F) << UInt64(shift)
            if byte & 0x80 == 0 {
                position += index
                return T(truncatingIfNeeded: value)
            }
            shift += 7
        }
        throw .vleWidthOverflow
    }

    /// The integer of type `T` that starts `offset` bytes after the cursor, read without moving it.
    /// A signed type is the two's-complement reading of the same bytes.
    public func peekInteger<T: FixedWidthInteger>(
        _ type: T.Type, at offset: Int, _ order: SceByteOrder
    ) throws(CodecError) -> T {
        let width = T.bitWidth / 8
        guard offset >= 0, remaining - offset >= width else {
            throw .needMoreBytes
        }
        var value: T = 0
        for index in 0..<width {
            let byte = T(truncatingIfNeeded: buffer[position + offset + index])
            switch order {
            case .big:
                value = (value << 8) | byte
            case .little:
                value |= byte << T(index * 8)
            }
        }
        return value
    }
}

/// Writes `value` into `buffer`, `offset` bytes from its start, in `order`: the counterpart of
/// `SceCursor.peekInteger`, for an encode that lays a whole frame out and hands it to the sink in
/// one write. A signed type is written as its two's-complement bytes.
///
/// The frame is sized by the generated code from its own fields, so a value that does not fit is a
/// defect of the generator and not an input: it stops the program and does not corrupt a neighbour.
public func storeInteger<T: FixedWidthInteger>(
    _ value: T, into buffer: inout [UInt8], at offset: Int, _ order: SceByteOrder
) {
    let width = T.bitWidth / 8
    precondition(
        offset >= 0 && offset + width <= buffer.count,
        "a \(width)-byte integer at \(offset) does not fit a frame of \(buffer.count) bytes")
    for index in 0..<width {
        let byte = UInt8(truncatingIfNeeded: value >> T(index * 8))
        switch order {
        case .big:
            buffer[offset + width - 1 - index] = byte
        case .little:
            buffer[offset + index] = byte
        }
    }
}

/// Appends `value` to `buffer` in `order`: the counterpart of `SceCursor.readInteger`, for an encode
/// whose layout depends on what it holds and so cannot be laid out at fixed offsets
/// (`storeInteger`). A signed type is written as its two's-complement bytes.
public func appendInteger<T: FixedWidthInteger>(
    _ value: T, to buffer: inout [UInt8], _ order: SceByteOrder
) {
    let width = T.bitWidth / 8
    for index in 0..<width {
        let shift = order == .big ? (width - 1 - index) * 8 : index * 8
        buffer.append(UInt8(truncatingIfNeeded: value >> T(shift)))
    }
}

/// Appends `value` to `buffer` as a variable-length unsigned integer: the counterpart of
/// `SceCursor.readVle`. Every byte but the last carries 7 data bits and a flag that another follows,
/// and the last byte a `T` can need carries a full 8 data bits, so the value of a `T` takes the
/// fewest bytes that hold it and a 64-bit one never more than 9 (the canonical Zenoh ZInt).
public func appendVle<T: FixedWidthInteger & UnsignedInteger>(
    _ value: T, to buffer: inout [UInt8]
) {
    let continuationMax = (T.bitWidth - 1 + 6) / 7 - 1
    var rest = UInt64(truncatingIfNeeded: value)
    var written = 0
    while rest >= 0x80 && written < continuationMax {
        buffer.append(UInt8(truncatingIfNeeded: rest & 0x7F) | 0x80)
        rest >>= 7
        written += 1
    }
    buffer.append(UInt8(truncatingIfNeeded: rest))
}

/// The write-side of an encode: the destination of the bytes, owned by the caller.
public protocol SceSink {
    /// Appends `data`. A bounded sink answers `bufferOverflow` and writes nothing when `data` does
    /// not fit; a growable one never fails.
    mutating func writeBytes(_ data: [UInt8]) throws(CodecError)

    /// The bytes this sink has written since it was made.
    var position: Int { get }
}

extension SceSink {
    /// Appends `value` in `order`. A signed type is written as its two's-complement bytes.
    public mutating func writeInteger<T: FixedWidthInteger>(
        _ value: T, _ order: SceByteOrder
    ) throws(CodecError) {
        let width = T.bitWidth / 8
        var bytes = [UInt8](repeating: 0, count: width)
        for index in 0..<width {
            let byte = UInt8(truncatingIfNeeded: value >> T(index * 8))
            switch order {
            case .big:
                bytes[width - 1 - index] = byte
            case .little:
                bytes[index] = byte
            }
        }
        try writeBytes(bytes)
    }
}

/// A growable sink: every write succeeds, and the bytes are `bytes`.
public struct BytesSink: SceSink {
    /// What the sink holds, including any prefix it was made with.
    public private(set) var bytes: [UInt8]
    private let startLength: Int

    /// An empty sink, or one that continues after `prefix` (a frame packed behind another), whose
    /// bytes `position` does not count.
    public init(prefix: [UInt8] = []) {
        self.bytes = prefix
        self.startLength = prefix.count
    }

    public mutating func writeBytes(_ data: [UInt8]) throws(CodecError) {
        bytes.append(contentsOf: data)
    }

    public var position: Int { bytes.count - startLength }
}

/// A sink of fixed capacity, for a frame that has to fit a buffer the caller owns.
public struct BoundedSink: SceSink {
    private var storage: [UInt8]
    private var used = 0

    /// A sink that holds at most `capacity` bytes.
    public init(capacity: Int) {
        self.storage = [UInt8](repeating: 0, count: max(0, capacity))
    }

    public mutating func writeBytes(_ data: [UInt8]) throws(CodecError) {
        guard storage.count - used >= data.count else {
            throw .bufferOverflow
        }
        for (index, byte) in data.enumerated() {
            storage[used + index] = byte
        }
        used += data.count
    }

    public var position: Int { used }

    /// The bytes left in the sink.
    public var remaining: Int { storage.count - used }

    /// The bytes written so far.
    public var written: [UInt8] { Array(storage[0..<used]) }
}
