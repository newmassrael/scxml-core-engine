// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A procedure and a validator that hold a value of a codec the C++ backend
// prints from its plan (docs/adr/0081 step 4), compiled and run.
//
// The headers are generated at build time: codec_origin_envelope and the body
// it holds, codec_origin_leaf, from tests/forge/resources, and the two readers
// from sce-build/tests/fixtures/printed_codec_reader. The envelope presents
// `hdr`, `note` (a text of at most 16 bytes), `required`, `optional` (a body
// `hdr.has_opt` gates) and `items` (at most three leaves); its encode derives
// `note_len` and `m`, which no value holds (docs/adr/0081 §3).
//
// printed_codec_rebuild assigns `hdr` and `note`, each by making the value
// again from the elements it presents, sends the frame and reports the
// `note_len` it carried; a note past its bound raises `error.execution` and
// ends in `refused` before anything is sent. printed_codec_check holds a value
// to an entry count and a note length, read as the encode derives them.

#include "codec_origin_envelope.h"
#include "codec_origin_leaf.h"
#include "printed_codec_check.h"
#include "printed_codec_rebuild.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

using Envelope = ::SCE::Generated::CodecOriginEnvelope::CodecOriginEnvelope;
using Leaf = ::SCE::Generated::CodecOriginLeaf::CodecOriginLeaf;
using ::SCE::Forge::ProcedureServiceRequest;
using ::SCE::Forge::ProcedureServiceResponse;

/// Runs the procedure with `hdr` and `note`, keeping the payload it sends.
::SCE::Forge::ProcedureRunResult rebuild(std::uint8_t hdr, const std::string &note,
                                         std::optional<std::vector<std::uint8_t>> &sent) {
    auto handler = [&sent](const ProcedureServiceRequest &request) {
        sent = request.payload;
        ProcedureServiceResponse response;
        response.success = true;
        return response;
    };
    return ::SCE::Generated::PrintedCodecRebuild::execute(handler, hdr, note);
}

// The value made again from the elements it presents is written with the
// numbers its encode derives: the note's length, and no entry; the gated body
// the value does not hold is left out, and its flag with it.
void an_assigned_value_is_written_with_what_its_encode_derives() {
    std::optional<std::vector<std::uint8_t>> sent;
    const auto result = rebuild(0x00, "hello", sent);
    CHECK(result.completed && result.final_state == "done", "a note within its bound is sent");
    const std::vector<std::uint8_t> frame{0x00, 0x05, 'h', 'e', 'l', 'l', 'o', 0x00, 0x00};
    CHECK(sent == std::optional<std::vector<std::uint8_t>>(frame), "the frame carries the note and its length");
    const auto note_len = result.done_data.find("note_len");
    CHECK(note_len != result.done_data.end() && note_len->second == "5",
          "the procedure reads the note's length as the frame carries it");
}

// A note past its bound is no value of the element: making it refuses, which
// raises `error.execution` before the value is made again or sent.
void a_note_past_its_bound_is_refused() {
    std::optional<std::vector<std::uint8_t>> sent;
    const auto result = rebuild(0x00, "seventeen bytes!!", sent);
    CHECK(result.completed && result.final_state == "refused", "a note past its bound raises error.execution");
    CHECK(!sent.has_value(), "nothing is sent for a refused note");
}

// The validator reads the count of the entries and the length of the note off
// the value, as the encode derives them, whatever else the value holds.
void a_validator_reads_the_numbers_the_frame_carries() {
    const std::vector<std::uint8_t> data{0x01, 0x02};
    const Leaf leaf(::SCE::Forge::Bytes<16>::make(data).value());
    const std::vector<Leaf> entries{leaf, leaf};
    const std::string note = "abc";
    ::SCE::Generated::PrintedCodecCheck::PrintedCodecCheck check;
    check.env_ = Envelope(0x01, ::SCE::Forge::Text<16>::make(note).value(), leaf, std::optional<Leaf>(leaf),
                          ::SCE::Forge::List<Leaf, 3>::make(entries).value());
    CHECK(check.validate(2, 3).valid, "two entries and a note of three bytes are what the value derives");
    CHECK(!check.validate(1, 3).valid, "one entry is not what the value derives");
    CHECK(!check.validate(2, 4).valid, "a note of four bytes is not what the value derives");
}

}  // namespace

int main() {
    an_assigned_value_is_written_with_what_its_encode_derives();
    a_note_past_its_bound_is_refused();
    a_validator_reads_the_numbers_the_frame_carries();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("printed codec readers: every check passed");
    return 0;
}
