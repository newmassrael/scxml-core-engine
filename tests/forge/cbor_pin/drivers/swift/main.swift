// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Swift driver of the CBOR pin (tests/forge/cbor_pin/pin.py, docs/adr/0101 step 0).
//
//     CborPinCapture INPUTS OUTPUT
//
// scripts/capture_cbor_pin.sh builds it as the executable target of a package of
// its own, beside the Swift the generator wrote for the pinned codecs and every
// document they import (`sce-codegen generate --with-imports -l swift`), over the
// forge runtime's package. Each line of INPUTS, `<codec>/<case> <hex>`, is decoded
// by that codec and the value it read encoded again; OUTPUT gets `<id> ok <hex>`
// with what the encode wrote, or `<id> refused <words>` naming the `CodecError`
// the decode or the encode threw.

import Foundation
import SceForgeRuntime

/// What a pinned codec offers the driver: the two halves every generated CBOR map has.
protocol PinnedCodec {
    static func decode(_ cursor: inout SceCursor) throws(CodecError) -> Self
    func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError)
}

extension CodecCborMap: PinnedCodec {}
extension BroadcastLogConformanceTextEvent: PinnedCodec {}
extension CodecCborOpenEnum: PinnedCodec {}
extension CodecCborSignedEnum: PinnedCodec {}
extension CodecCborRecordList: PinnedCodec {}
extension CodecCborRecordListBytes: PinnedCodec {}
extension CodecCborRecordListLeast: PinnedCodec {}
extension Envelope: PinnedCodec {}

/// The bytes the encode wrote, or the refusal in words.
enum Outcome {
    case encoded([UInt8])
    case refused(String)
}

func roundTrip<Codec: PinnedCodec>(_ codec: Codec.Type, _ input: [UInt8]) -> Outcome {
    var cursor = SceCursor(input)
    let value: Codec
    do {
        value = try Codec.decode(&cursor)
    } catch {
        return .refused("decode \(error)")
    }
    var sink = BytesSink()
    do {
        try value.encode(into: &sink)
    } catch {
        return .refused("encode \(error)")
    }
    return .encoded(sink.bytes)
}

func roundTrip(codec name: Substring, _ input: [UInt8]) -> Outcome? {
    switch name {
    case "codec_cbor_map": return roundTrip(CodecCborMap.self, input)
    case "broadcast_log_conformance_text_event": return roundTrip(BroadcastLogConformanceTextEvent.self, input)
    case "codec_cbor_open_enum": return roundTrip(CodecCborOpenEnum.self, input)
    case "codec_cbor_signed_enum": return roundTrip(CodecCborSignedEnum.self, input)
    case "codec_cbor_record_list": return roundTrip(CodecCborRecordList.self, input)
    case "codec_cbor_record_list_bytes": return roundTrip(CodecCborRecordListBytes.self, input)
    case "codec_cbor_record_list_least": return roundTrip(CodecCborRecordListLeast.self, input)
    case "envelope": return roundTrip(Envelope.self, input)
    default: return nil
    }
}

func nibble(_ c: UInt8) -> UInt8? {
    switch c {
    case UInt8(ascii: "0")...UInt8(ascii: "9"): return c - UInt8(ascii: "0")
    case UInt8(ascii: "a")...UInt8(ascii: "f"): return c - UInt8(ascii: "a") + 10
    default: return nil
    }
}

func unhex(_ digits: ArraySlice<UInt8>) -> [UInt8]? {
    guard digits.count % 2 == 0 else { return nil }
    var bytes = [UInt8]()
    bytes.reserveCapacity(digits.count / 2)
    var index = digits.startIndex
    while index < digits.endIndex {
        guard let high = nibble(digits[index]), let low = nibble(digits[index + 1]) else { return nil }
        bytes.append(high << 4 | low)
        index += 2
    }
    return bytes
}

func appendHex(_ bytes: [UInt8], to line: inout [UInt8]) {
    let digits = Array("0123456789abcdef".utf8)
    line.reserveCapacity(line.count + bytes.count * 2)
    for b in bytes {
        line.append(digits[Int(b >> 4)])
        line.append(digits[Int(b & 0x0f)])
    }
}

func capture(inputs: String, output: String) -> Int32 {
    guard let data = FileManager.default.contents(atPath: inputs) else {
        FileHandle.standardError.write(Data("capture: cannot read \(inputs)\n".utf8))
        return 1
    }
    let text = [UInt8](data)
    var written = [UInt8]()
    for line in text.split(separator: UInt8(ascii: "\n"), omittingEmptySubsequences: true) {
        guard let space = line.firstIndex(of: UInt8(ascii: " ")),
            let input = unhex(line[(space + 1)...])
        else {
            FileHandle.standardError.write(Data("capture: not a case line\n".utf8))
            return 1
        }
        let id = String(decoding: line[..<space], as: UTF8.self)
        let name = id.split(separator: "/", maxSplits: 1).first ?? Substring(id)
        guard let outcome = roundTrip(codec: name, input) else {
            FileHandle.standardError.write(Data("capture: \(id) names no pinned codec\n".utf8))
            return 1
        }
        written.append(contentsOf: Array(id.utf8))
        switch outcome {
        case .encoded(let bytes):
            written.append(contentsOf: Array(" ok ".utf8))
            appendHex(bytes, to: &written)
        case .refused(let words):
            written.append(contentsOf: Array(" refused \(words)".utf8))
        }
        written.append(UInt8(ascii: "\n"))
    }
    guard FileManager.default.createFile(atPath: output, contents: Data(written)) else {
        FileHandle.standardError.write(Data("capture: cannot write \(output)\n".utf8))
        return 1
    }
    return 0
}

let arguments = CommandLine.arguments
if arguments.count != 3 {
    FileHandle.standardError.write(Data("usage: CborPinCapture INPUTS OUTPUT\n".utf8))
    exit(2)
}
exit(capture(inputs: arguments[1], output: arguments[2]))
