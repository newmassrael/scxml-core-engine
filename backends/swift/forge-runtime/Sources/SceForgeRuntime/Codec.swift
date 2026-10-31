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
