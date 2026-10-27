// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The one definition of SCE_LOG_* (docs/adr/0060): a call asks the program's installed
// sink whether its level is wanted, formats only then, and SCE::Logger is a sink a
// program installs by configuring it. The formatter's grammar is held here too.

#include "common/LogSink.h"
#include "common/ILoggerBackend.h"
#include "common/LogFormat.h"
#include "common/Logger.h"
#include "core/LogMacros.h"

#include <gtest/gtest.h>

#include <cstdint>
#include <memory>
#include <ostream>
#include <string>
#include <utility>
#include <vector>

namespace {

// An argument that counts how often it is written: formatting a message that holds
// it is the only thing that writes it.
struct CountedArgument {
    int *writes;
};

std::ostream &operator<<(std::ostream &out, const CountedArgument &argument) {
    ++*argument.writes;
    return out << "counted";
}

// A sink of the test's own, recording what reaches it.
std::vector<std::string> &recordedBySink() {
    static std::vector<std::string> messages;
    return messages;
}

bool wantsAllButDebug(SCE::LogLevel level) noexcept {
    return level != SCE::LogLevel::Debug;
}

void recordMessage(SCE::LogLevel /*level*/, std::string message, const SCE::source_location & /*where*/) {
    recordedBySink().push_back(std::move(message));
}

constexpr SCE::LogSink kRecordingSink{&wantsAllButDebug, &recordMessage};

struct BackendRecord {
    SCE::LogLevel level;
    std::string message;
};

using BackendRecords = std::vector<BackendRecord>;

// A backend the Logger forwards to. It shares its records with the test, because the
// Logger keeps the backend after the test that installed it has ended.
class RecordingBackend : public SCE::ILoggerBackend {
public:
    explicit RecordingBackend(std::shared_ptr<BackendRecords> records) : records_(std::move(records)) {}

    void log(SCE::LogLevel level, const std::string &message, const SCE::source_location & /*loc*/) override {
        records_->push_back(BackendRecord{level, message});
    }

    void setLevel(SCE::LogLevel level) override {
        level_ = level;
    }

    bool shouldLog(SCE::LogLevel level) const override {
        return level >= level_;
    }

    void flush() override {}

private:
    std::shared_ptr<BackendRecords> records_;
    SCE::LogLevel level_ = SCE::LogLevel::Trace;
};

// Puts back whatever sink was installed when the test began.
class InstalledSinkRestorer {
public:
    InstalledSinkRestorer() : saved_(SCE::detail::g_logSink.load()) {}

    ~InstalledSinkRestorer() {
        SCE::installLogSink(saved_);
    }

    InstalledSinkRestorer(const InstalledSinkRestorer &) = delete;
    InstalledSinkRestorer &operator=(const InstalledSinkRestorer &) = delete;

private:
    const SCE::LogSink *saved_;
};

bool endsWith(const std::string &text, const std::string &suffix) {
    return text.size() >= suffix.size() && text.compare(text.size() - suffix.size(), suffix.size(), suffix) == 0;
}

}  // namespace

TEST(LogSink, NothingIsFormattedWhileNoSinkIsInstalled) {
    const InstalledSinkRestorer restore;
    SCE::installLogSink(nullptr);

    int writes = 0;
    int evaluated = 0;
    SCE_LOG_ERROR("value {}", CountedArgument{&writes});
    SCE_LOG_INFO("value {}", ++evaluated);

    EXPECT_FALSE(SCE::detail::logWanted(SCE::LogLevel::Error));
    EXPECT_EQ(writes, 0) << "a message no sink wants was formatted";
    EXPECT_EQ(evaluated, 0) << "an argument of a message no sink wants was evaluated";
}

TEST(LogSink, AnInstalledSinkReceivesTheLevelsItWantsAndOnlyThoseAreFormatted) {
    const InstalledSinkRestorer restore;
    recordedBySink().clear();
    SCE::installLogSink(&kRecordingSink);

    int writes = 0;
    SCE_LOG_DEBUG("unwanted {}", CountedArgument{&writes});
    EXPECT_EQ(writes, 0) << "a level the sink does not want was formatted";

    SCE_LOG_WARN("wanted {} of {}", CountedArgument{&writes}, 2);
    EXPECT_EQ(writes, 1);
    ASSERT_EQ(recordedBySink().size(), 1u);
    EXPECT_EQ(recordedBySink()[0], "wanted counted of 2");
}

TEST(LogSink, ConfiguringTheLoggerInstallsItsSinkAndHeaderOnlyCallsReachTheBackend) {
    const InstalledSinkRestorer restore;
    SCE::installLogSink(nullptr);

    auto records = std::make_shared<BackendRecords>();
    SCE::Logger::setBackend(std::make_unique<RecordingBackend>(records));
    EXPECT_EQ(SCE::detail::g_logSink.load(), &SCE::Logger::sink());

    SCE_LOG_WARN("{} of {}", 3, std::string("five"));
    ASSERT_EQ(records->size(), 1u);
    EXPECT_EQ((*records)[0].level, SCE::LogLevel::Warn);
    // The Logger names the calling function before the message, as for its own calls.
    EXPECT_TRUE(endsWith((*records)[0].message, "() - 3 of five")) << (*records)[0].message;

    // The sink asks the backend's level before formatting.
    SCE::Logger::setLevel(SCE::LogLevel::Error);
    int writes = 0;
    SCE_LOG_INFO("below the level {}", CountedArgument{&writes});
    EXPECT_EQ(writes, 0);
    EXPECT_EQ(records->size(), 1u);

    // initialize and setLevel install the sink as setBackend does; the backend stays.
    SCE::installLogSink(nullptr);
    SCE::Logger::initialize();
    EXPECT_EQ(SCE::detail::g_logSink.load(), &SCE::Logger::sink());
    SCE::installLogSink(nullptr);
    SCE::Logger::setLevel(SCE::LogLevel::Trace);
    EXPECT_EQ(SCE::detail::g_logSink.load(), &SCE::Logger::sink());
    SCE_LOG_ERROR("after {}", "initialize");
    ASSERT_EQ(records->size(), 2u);
    EXPECT_TRUE(endsWith((*records)[1].message, "() - after initialize")) << (*records)[1].message;
}

TEST(LogSink, ALoggerSinkInitKeepsASinkTheProgramInstalled) {
    const InstalledSinkRestorer restore;

    SCE::installLogSink(&kRecordingSink);
    const SCE::Logger::SinkInit keeps;
    EXPECT_EQ(SCE::detail::g_logSink.load(), &kRecordingSink);

    SCE::installLogSink(nullptr);
    const SCE::Logger::SinkInit installs;
    EXPECT_EQ(SCE::detail::g_logSink.load(), &SCE::Logger::sink());
}

TEST(LogFormat, EachFieldTakesTheNextArgument) {
    EXPECT_EQ(SCE::detail::formatLog("a {} b {}", 1, "x"), "a 1 b x");
    EXPECT_EQ(SCE::detail::formatLog("plain"), "plain");
    EXPECT_EQ(SCE::detail::formatLog("{}{}", std::string("ab"), 'c'), "abc");
}

TEST(LogFormat, DoubledBracesAreOneLiteralBrace) {
    EXPECT_EQ(SCE::detail::formatLog("{{}}"), "{}");
    EXPECT_EQ(SCE::detail::formatLog("{{{}}}", 7), "{7}");
    EXPECT_EQ(SCE::detail::formatLog("{{\"a\":{}}}", 1), "{\"a\":1}");
}

TEST(LogFormat, AFieldWithNoArgumentLeftIsWrittenAsItStands) {
    EXPECT_EQ(SCE::detail::formatLog("{} and {}", 1), "1 and {}");
    EXPECT_EQ(SCE::detail::formatLog("{}"), "{}");
}

TEST(LogFormat, AnArgumentWithNoFieldLeftIsNotWritten) {
    EXPECT_EQ(SCE::detail::formatLog("only {}", 1, 2), "only 1");
    EXPECT_EQ(SCE::detail::formatLog("none", 1), "none");
}

TEST(LogFormat, AnyOtherBraceIsWrittenAsItStands) {
    EXPECT_EQ(SCE::detail::formatLog("{x} } {", 1), "{x} } {");
}

TEST(LogFormat, ValuesAreWrittenAsStdFormatWroteThem) {
    EXPECT_EQ(SCE::detail::formatLog("{} {}", true, false), "true false");
    EXPECT_EQ(SCE::detail::formatLog("{}", std::uint8_t{65}), "65");
    EXPECT_EQ(SCE::detail::formatLog("{}", std::int8_t{-3}), "-3");
    EXPECT_EQ(SCE::detail::formatLog("{}", 'A'), "A");
    const char *absent = nullptr;
    EXPECT_EQ(SCE::detail::formatLog("[{}]", absent), "[(null)]");
    EXPECT_EQ(SCE::detail::formatLog(nullptr, 1), "");
}

TEST(LogFormat, AFloatingPointValueIsItsShortestTextThatReadsBack) {
    EXPECT_EQ(SCE::detail::formatLog("{}", 0.1), "0.1");
    EXPECT_EQ(SCE::detail::formatLog("{}", 3.141592653589793), "3.141592653589793");
    EXPECT_EQ(SCE::detail::formatLog("{}", 2.5f), "2.5");
}
