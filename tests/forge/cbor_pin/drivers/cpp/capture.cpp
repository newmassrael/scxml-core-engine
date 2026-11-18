// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The C++ driver of the CBOR pin (tests/forge/cbor_pin/pin.py, docs/adr/0101 step 0).
//
//     capture INPUTS OUTPUT
//
// Compiled by scripts/capture_cbor_pin.sh against the headers the generator wrote
// for the pinned codecs and every document they import (`sce-codegen generate
// --with-imports -l cpp`) and the forge runtime's. Each line of INPUTS,
// `<codec>/<case> <hex>`, is decoded by that codec and the value it read encoded
// again; OUTPUT gets `<id> ok <hex>` with what the encode wrote, or
// `<id> refused <words>` naming the `CodecError` the decode or the encode answered.

#include <cstddef>
#include <cstdint>
#include <fstream>
#include <iostream>
#include <string>
#include <utility>
#include <vector>

#include "broadcast_log_conformance_text_event.h"
#include "codec_cbor_map.h"
#include "codec_cbor_open_enum.h"
#include "codec_cbor_record_list.h"
#include "codec_cbor_record_list_bytes.h"
#include "codec_cbor_record_list_least.h"
#include "codec_cbor_signed_enum.h"
#include "envelope.h"
#include "sce/forge/codec.h"

namespace {

using Bytes = std::vector<std::uint8_t>;

struct Outcome {
    bool encoded;
    Bytes bytes;
    std::string refusal;
};

template <typename Codec> Outcome round_trip(const Bytes &input) {
    ::SCE::Forge::SceCursor cursor(input.data(), input.size());
    auto decoded = Codec::decode(cursor);
    if (!decoded) {
        return {false, {}, "decode CodecError " + std::to_string(static_cast<int>(decoded.error()))};
    }
    Bytes out;
    ::SCE::Forge::VectorSink sink(out);
    if (const auto refused = (*decoded).encode(sink)) {
        return {false, {}, "encode CodecError " + std::to_string(static_cast<int>(*refused))};
    }
    return {true, std::move(out), {}};
}

using RoundTrip = Outcome (*)(const Bytes &);

const std::vector<std::pair<std::string, RoundTrip>> &codecs() {
    static const std::vector<std::pair<std::string, RoundTrip>> table = {
        {"codec_cbor_map", &round_trip<::SCE::Generated::CodecCborMap::CodecCborMap>},
        {"broadcast_log_conformance_text_event",
         &round_trip<::SCE::Generated::BroadcastLogConformanceTextEvent::BroadcastLogConformanceTextEvent>},
        {"codec_cbor_open_enum", &round_trip<::SCE::Generated::CodecCborOpenEnum::CodecCborOpenEnum>},
        {"codec_cbor_signed_enum", &round_trip<::SCE::Generated::CodecCborSignedEnum::CodecCborSignedEnum>},
        {"codec_cbor_record_list", &round_trip<::SCE::Generated::CodecCborRecordList::CodecCborRecordList>},
        {"codec_cbor_record_list_bytes",
         &round_trip<::SCE::Generated::CodecCborRecordListBytes::CodecCborRecordListBytes>},
        {"codec_cbor_record_list_least",
         &round_trip<::SCE::Generated::CodecCborRecordListLeast::CodecCborRecordListLeast>},
        {"envelope", &round_trip<::SCE::Generated::Envelope::Envelope>},
    };
    return table;
}

int nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    return -1;
}

bool unhex(const std::string &text, Bytes &out) {
    if (text.size() % 2 != 0) {
        return false;
    }
    out.clear();
    out.reserve(text.size() / 2);
    for (std::size_t i = 0; i < text.size(); i += 2) {
        const int high = nibble(text[i]);
        const int low = nibble(text[i + 1]);
        if (high < 0 || low < 0) {
            return false;
        }
        out.push_back(static_cast<std::uint8_t>(high << 4 | low));
    }
    return true;
}

void put_hex(std::string &line, const Bytes &bytes) {
    static const char digits[] = "0123456789abcdef";
    line.reserve(line.size() + bytes.size() * 2);
    for (const std::uint8_t b : bytes) {
        line.push_back(digits[b >> 4]);
        line.push_back(digits[b & 0x0f]);
    }
}

}  // namespace

int main(int argc, char **argv) {
    if (argc != 3) {
        std::cerr << "usage: capture INPUTS OUTPUT\n";
        return 2;
    }
    std::ifstream source(argv[1]);
    std::ofstream sink(argv[2], std::ios::binary);
    if (!source || !sink) {
        std::cerr << "capture: cannot open " << argv[1] << " or " << argv[2] << "\n";
        return 1;
    }
    std::string line;
    Bytes input;
    while (std::getline(source, line)) {
        if (line.empty()) {
            continue;
        }
        const std::size_t space = line.find(' ');
        const std::string id = line.substr(0, space);
        const std::string codec = id.substr(0, id.find('/'));
        RoundTrip run = nullptr;
        for (const auto &entry : codecs()) {
            if (entry.first == codec) {
                run = entry.second;
            }
        }
        if (space == std::string::npos || run == nullptr || !unhex(line.substr(space + 1), input)) {
            std::cerr << "capture: not a line of a pinned codec's case: " << id << "\n";
            return 1;
        }
        const Outcome outcome = run(input);
        std::string written = id;
        if (outcome.encoded) {
            written += " ok ";
            put_hex(written, outcome.bytes);
        } else {
            written += " refused " + outcome.refusal;
        }
        written += '\n';
        sink << written;
    }
    sink.close();
    return sink ? 0 : 1;
}
