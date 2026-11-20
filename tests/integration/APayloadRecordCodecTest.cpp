// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0109, D9: a record signed by name has two writers, the runtime's record codec (common/RecordCbor.h),
// which a machine writes the record with from its payload JSON, and the codec ADR 0101 prints for the same
// schema, which a peer that reads the signed record is built from. Each case of the corpus
// (tests/forge/conformance/numerical_reference.json, `codec_cbor_payload_record`) is written by both to its one
// set of bytes and read by both back to its value, and each side reads what the other wrote. The printed side is a
// map of the record under key 0 (codec_cbor_payload_note.scxml, codec_cbor_payload_leaves.scxml), whose value is
// `a1 00` and the record's own map. The Rust twin is
// backends/rust/forge-conformance/tests/forge_cbor_payload_record.rs.

#include "codec_cbor_payload_leaves.h"
#include "codec_cbor_payload_note.h"
#include "common/RecordCbor.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
using Bytes = std::vector<uint8_t>;
using PrintedNote = ::SCE::Generated::CodecCborPayloadNote::CodecCborPayloadNote;
using PrintedLeaves = ::SCE::Generated::CodecCborPayloadLeaves::CodecCborPayloadLeaves;

/// The head of the printed codec's map of one entry and the key of that entry, which the record's own map follows.
const Bytes kUnderKey0{0xa1, 0x00};

json corpus() {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "forge" / "conformance" /
                     "numerical_reference.json");
    EXPECT_TRUE(in.good()) << "the corpus is not readable";
    return json::parse(in).at("codecs").at("codec_cbor_payload_record");
}

Bytes bytesOf(const json &v) {
    Bytes out;
    for (const json &b : v) {
        out.push_back(b.get<uint8_t>());
    }
    return out;
}

/// The record @p encoded holds, read by the printed codec @p Codec and written by it again: the record's own map,
/// after the head and the key the printed map adds.
template <typename Codec> Bytes printedOf(const Bytes &encoded, const std::string &note) {
    Bytes framed = kUnderKey0;
    framed.insert(framed.end(), encoded.begin(), encoded.end());
    ::SCE::Forge::SceCursor cursor(framed.data(), framed.size());
    auto read = Codec::decode(cursor);
    if (!read) {
        ADD_FAILURE() << note << ": the printed codec refused the record (" << static_cast<int>(read.error()) << ")";
        return {};
    }
    EXPECT_EQ(cursor.remaining(), 0u) << note << ": the printed codec read the whole map";
    const auto written = read->encode_to_vec();
    if (!written || written->size() < kUnderKey0.size()) {
        ADD_FAILURE() << note << ": the printed codec did not write the record";
        return {};
    }
    EXPECT_EQ(Bytes(written->begin(), written->begin() + 2), kUnderKey0) << note;
    return Bytes(written->begin() + 2, written->end());
}

/// The printed codec of the record @p record names.
Bytes printed(const std::string &record, const Bytes &encoded, const std::string &note) {
    if (record == "note") {
        return printedOf<PrintedNote>(encoded, note);
    }
    if (record == "leaves") {
        return printedOf<PrintedLeaves>(encoded, note);
    }
    ADD_FAILURE() << "the corpus names a record " << record << " this test has no codec of";
    return {};
}

}  // namespace

TEST(APayloadRecordCodec, TheRecordCodecWritesAndReadsEachCaseAsTheCorpusDoes) {
    const json entry = corpus();
    ASSERT_GE(entry.at("cases").size(), 6u);
    for (const json &item : entry.at("cases")) {
        const std::string note = item.at("note").get<std::string>();
        const std::string description =
            entry.at("records").at(item.at("record").get<std::string>()).at("description").get<std::string>();
        const Bytes encoded = bytesOf(item.at("encoded"));
        std::string refusal;
        const auto written = Common::RecordCbor::encode(item.at("payload").dump(), description, refusal);
        ASSERT_TRUE(written.has_value()) << note << ": " << refusal;
        EXPECT_EQ(*written, encoded) << note << ": the record codec's bytes";
        const auto read = Common::RecordCbor::decode(encoded, description, refusal);
        ASSERT_TRUE(read.has_value()) << note << ": " << refusal;
        EXPECT_EQ(json::parse(*read), item.at("payload")) << note << ": the record codec's value";
    }
}

TEST(APayloadRecordCodec, ThePrintedCodecWritesAndReadsEachCaseAsTheCorpusDoes) {
    const json entry = corpus();
    for (const json &item : entry.at("cases")) {
        const std::string note = item.at("note").get<std::string>();
        const Bytes encoded = bytesOf(item.at("encoded"));
        EXPECT_EQ(printed(item.at("record").get<std::string>(), encoded, note), encoded)
            << note << ": the printed codec's bytes";
    }
}

TEST(APayloadRecordCodec, EachWriterWritesWhatTheOtherReads) {
    const json entry = corpus();
    for (const json &item : entry.at("cases")) {
        const std::string note = item.at("note").get<std::string>();
        const std::string record = item.at("record").get<std::string>();
        const std::string description = entry.at("records").at(record).at("description").get<std::string>();
        std::string refusal;
        // The record codec writes; the printed codec reads it and writes the same bytes again.
        const auto byRuntime = Common::RecordCbor::encode(item.at("payload").dump(), description, refusal);
        ASSERT_TRUE(byRuntime.has_value()) << note << ": " << refusal;
        EXPECT_EQ(printed(record, *byRuntime, note), *byRuntime)
            << note << ": the printed codec reads the record codec's bytes";
        // The printed codec writes; the record codec reads it to the value.
        const Bytes byPrinted = printed(record, bytesOf(item.at("encoded")), note);
        const auto read = Common::RecordCbor::decode(byPrinted, description, refusal);
        ASSERT_TRUE(read.has_value()) << note << ": " << refusal;
        EXPECT_EQ(json::parse(*read), item.at("payload")) << note << ": the record codec reads the printed bytes";
    }
}

}  // namespace SCE::Tests
