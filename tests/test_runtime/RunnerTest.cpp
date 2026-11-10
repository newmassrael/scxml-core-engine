// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of a generated suite in the C++ runtime
// (docs/SCE_TTCN3_LANE.md §4; test/Runner.h): the command line every
// backend's runner takes, and the readers of a module parameter's value given
// on it. Each case is the twin of the Rust runtime's of the same subject, in
// backends/rust/runtime/src/test/runner.rs, with the same lines and the same
// refusals. The program run whole, over a generated suite, is
// tests/integration/ARunnerProgramWritesTheLogOfEachTraceTest.cpp.

#include <gtest/gtest.h>

#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "saved/SavedValues.h"
#include "test/Runner.h"

namespace {

using SCE::Saved::writeCanonical;
using SCE::Test::Runner::RunnerOptions;

/// The reason @p read refuses with, which it must.
template <typename Read> std::string refusal(Read read) {
    try {
        read();
    } catch (const std::invalid_argument &refused) {
        return refused.what();
    }
    ADD_FAILURE() << "the line is read where it should be refused";
    return "";
}

/// The command line every backend's runner takes.
TEST(TheRunner, ReadsTheCommandLineAsEveryRunnerReadsIt) {
    const RunnerOptions options =
        RunnerOptions::parse({"--trace-dir", "traces", "--log-dir", "logs", "--world", "ptcs,guard",
                              "--module-parameter", "job=9", "--module-parameter", "label=a=b"});
    EXPECT_EQ(options.traceDir.string(), "traces");
    EXPECT_EQ(options.logDir.string(), "logs");
    EXPECT_EQ(options.world, (SCE::Test::Features{"ptcs", "guard"}));
    EXPECT_EQ(options.moduleParameters,
              (std::vector<std::pair<std::string, std::string>>{{"job", "9"}, {"label", "a=b"}}));
    const RunnerOptions bare = RunnerOptions::parse({"--log-dir", "l", "--trace-dir", "t"});
    EXPECT_TRUE(bare.world.empty());
    const std::vector<std::pair<std::vector<std::string>, std::string>> refused = {
        {{"--log-dir", "l"}, "--trace-dir is not given"},
        {{"--trace-dir", "t"}, "--log-dir is not given"},
        {{"--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u"}, "--trace-dir is given twice"},
        {{"--trace-dir", "t", "--log-dir", "l", "--seed", "1"}, "--seed is no option"},
        {{"--trace-dir"}, "--trace-dir is given no value"},
        {{"--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job"}, "is not <id>=<value>"},
        {{"--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job=1", "--module-parameter", "job=2"},
         "'job' is given twice"},
    };
    for (const auto &[line, why] : refused) {
        const std::string reason = refusal([&line = line] { (void)RunnerOptions::parse(line); });
        EXPECT_NE(reason.find(why), std::string::npos) << why << ": " << reason;
    }
}

/// A module parameter's literal is read as the saved value of its type, and
/// one that is no value of it is refused, naming the parameter.
TEST(TheRunner, ReadsAModuleParameterAsTheSavedValueOfItsType) {
    namespace Runner = SCE::Test::Runner;
    EXPECT_EQ(writeCanonical(Runner::integerParameter("job", "9", &SCE::Saved::savedWriteUint32)),
              writeCanonical(SCE::Saved::savedWriteUint32(9)));
    EXPECT_EQ(writeCanonical(Runner::integerParameter("wide", "18446744073709551615", &SCE::Saved::savedWriteUint64)),
              writeCanonical(SCE::Saved::savedWriteUint64(std::numeric_limits<std::uint64_t>::max())));
    EXPECT_EQ(
        refusal([] { (void)Runner::integerParameter("job", "256", &SCE::Saved::savedWriteUint8); }).rfind("job: ", 0),
        0u);
    EXPECT_NE(refusal([] { (void)Runner::integerParameter("job", "09", &SCE::Saved::savedWriteUint32); }), "");
    EXPECT_EQ(writeCanonical(Runner::boolParameter("armed", "true")), "true");
    EXPECT_NE(refusal([] { (void)Runner::boolParameter("armed", "yes"); }), "");
    EXPECT_EQ(writeCanonical(Runner::textParameter("nightly")), "\"nightly\"");
    EXPECT_EQ(writeCanonical(Runner::enumParameter("green")), "\"green\"");
    EXPECT_EQ(writeCanonical(Runner::bytesParameter("key", "0aFF")),
              writeCanonical(SCE::Saved::savedWriteBytes(std::vector<std::uint8_t>{0x0a, 0xff})));
    EXPECT_NE(refusal([] { (void)Runner::bytesParameter("key", "0a0"); }), "");
    EXPECT_NE(refusal([] { (void)Runner::bytesParameter("key", "0g"); }), "");
    EXPECT_NE(refusal([] { (void)Runner::bytesParameter("key", "+a"); }), "");
    EXPECT_EQ(std::string(Runner::undeclaredParameter("limit").what()),
              "the suite declares no module parameter 'limit'");
}

}  // namespace
