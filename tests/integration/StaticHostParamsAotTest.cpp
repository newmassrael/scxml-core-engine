// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param expr>` of a `<send>` or an `<invoke>` the
// HOST serves, in a `datamodel="sce-static"` machine, carries the value of a typed
// expression read from the machine's own fields when the send or the invoke
// happens. C++ AOT compile+run gate; the Rust, Go and Kotlin twins are
// `a_static_machines_typed_params_reach_the_host.rs`, `host_params_test.go` and
// `StaticHostParamsTest`.
//
// The machine is built with NO script engine: its variables are fields, and a
// `<param>` that needed an engine to be read would not have one to ask.
//
// What this holds is the value on the wire. The run `bump`, `go` changes every
// variable before the send and the invoke read it, so a copy taken at start-up
// (3, false, "idle") is told from what the fields hold now (4, true, "busy").
//
// Fixture: sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml
// (shared with the Rust, Go and Kotlin channels; `tests/CMakeLists.txt` compiles it
// here with the same two declarations `scripts/regen_host_processor.sh` passes).

#include "statechart_static_host_params_sm.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "core/HostProcessor.h"

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::statechart_static_host_params::statechart_static_host_params;
using Params = std::map<std::string, std::vector<std::string>>;

/// The type the fixture was compiled for. `tests/CMakeLists.txt` passes this same
/// string to both declarations.
constexpr const char *DECLARED_TYPE = "x-sce-host";

class StaticHostParamsAotTest : public ::testing::Test {
protected:
    /// A machine with the host's side registered, standing at `idle`.
    void SetUp() override {
        sm.registerEventProcessor(DECLARED_TYPE, [this](const SCE::HostSendRequest &req) {
            sends.push_back(req);
            return std::vector<SCE::HostSendResponse>{};
        });
        sm.registerInvoker(DECLARED_TYPE, [this](const SCE::HostInvokeEvent &ev) {
            if (ev.start.has_value()) {
                starts.push_back(*ev.start);
            }
            return std::optional<SCE::HostInvokeResponse>();
        });
        sm.initialize();
        sm.step();
    }

    void drive(const std::vector<std::string> &events) {
        for (const auto &event : events) {
            sm.raiseExternal(event);
            sm.step();
        }
    }

    /// The text each param crosses as, given what `count`, `ready`, `label` and
    /// `twice` hold. `delta` and `ratio` never change; `boom` is left out, because
    /// the multiplication that makes it overflows a 32-bit field.
    static Params wanted(const std::string &count, const std::string &ready, const std::string &label,
                         const std::string &twice) {
        return {{"count", {count}}, {"ready", {ready}}, {"label", {label}},
                {"twice", {twice}}, {"delta", {"-5"}},  {"ratio", {"1.5"}}};
    }

    /// `eventData` is the pairs as JSON, typed as the data model holds them: a
    /// number stays a number, a bool a bool, a string a string.
    static void expectTypedEventData(const std::string &eventData, const std::string &what) {
        const auto value = nlohmann::json::parse(eventData, nullptr, false);
        ASSERT_FALSE(value.is_discarded()) << what << ": eventData is not JSON: " << eventData;
        EXPECT_EQ(value.at("count"), 4) << what << ": " << eventData;
        EXPECT_EQ(value.at("ready"), true) << what << ": " << eventData;
        EXPECT_EQ(value.at("label"), "busy") << what << ": " << eventData;
        EXPECT_EQ(value.at("twice"), 8) << what << ": " << eventData;
        EXPECT_EQ(value.at("delta"), -5) << what << ": " << eventData;
        EXPECT_EQ(value.at("ratio"), 1.5) << what << ": " << eventData;
        EXPECT_FALSE(value.contains("boom")) << what << ": a pair whose value failed is left out: " << eventData;
    }

    Machine sm;
    std::vector<SCE::HostSendRequest> sends;
    std::vector<SCE::HostInvokeRequest> starts;
};

}  // namespace

TEST_F(StaticHostParamsAotTest, ASendParamCarriesTheValueTheFieldsHoldWhenItIsSent) {
    drive({"bump", "go"});

    ASSERT_EQ(sends.size(), 1u) << "one <send>, one request";
    EXPECT_EQ(sends[0].params, wanted("4", "true", "busy", "8")) << "the text each <param> crosses as";
    expectTypedEventData(sends[0].eventData, "send");
}

TEST_F(StaticHostParamsAotTest, AnInvokeParamCarriesTheValueTheFieldsHoldWhenItStarts) {
    drive({"bump", "go"});

    ASSERT_EQ(starts.size(), 1u) << "one <invoke>, one start";
    EXPECT_EQ(starts[0].params, wanted("4", "true", "busy", "8"))
        << "the text each <param> crosses as: a copy taken at start-up would say count 3, ready false, label idle";
    expectTypedEventData(starts[0].eventData, "invoke");
}

// The same machine on the shorter run: nothing has written a variable, so the
// fields still hold what `<data expr>` gave them. The control that keeps the two
// cases above from passing on a value that is simply always the new one.
TEST_F(StaticHostParamsAotTest, AParamReadBeforeAnyBumpCarriesTheDeclaredValues) {
    drive({"go"});

    ASSERT_EQ(sends.size(), 1u);
    ASSERT_EQ(starts.size(), 1u);
    EXPECT_EQ(sends[0].params, wanted("3", "false", "idle", "6"));
    EXPECT_EQ(starts[0].params, wanted("3", "false", "idle", "6"));
}

// W3C SCXML 5.7.1: a `<param>` whose value cannot be computed — here a
// multiplication a 32-bit field cannot hold — is reported with `error.execution`
// and its pair left out, while the message still goes and the invocation still
// starts. `errors` counts the reports the document took, one for the send and one
// for the invoke, so a pair dropped in silence is told from one reported.
TEST_F(StaticHostParamsAotTest, AParamWhoseValueCannotBeComputedIsReportedAndLeftOut) {
    drive({"bump", "go"});

    ASSERT_EQ(sends.size(), 1u) << "the send still went";
    ASSERT_EQ(starts.size(), 1u) << "the invoke still started";
    EXPECT_EQ(sm.errors(), 2u) << "one error.execution for the send's `boom` and one for the invoke's";
    EXPECT_EQ(sends[0].params.count("boom"), 0u) << "the failed pair is left out of the send";
    EXPECT_EQ(starts[0].params.count("boom"), 0u) << "the failed pair is left out of the invoke";
}

}  // namespace SCE::Tests
