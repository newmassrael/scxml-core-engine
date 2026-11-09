// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 2): an event no Event I/O Processor
// delivered has no origin — a host-posted event and a host processor's reply
// that name none read both `_event.origin` and `_event.origintype` as the empty
// string — C++ AOT. The generated engine used to give the reply the SCXML
// processor's type with an empty address, and the Interpreter none: the two
// engines now agree.
//
// Fixture:
// integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(an_event_no_processor_delivered_has_no_origin ... HOST_PROCESSOR x-sce-host)`.

#include "an_event_no_processor_delivered_has_no_origin_sm.h"
#include "core/HostProcessor.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::an_event_no_processor_delivered_has_no_origin::an_event_no_processor_delivered_has_no_origin;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(AnEventNoProcessorDeliveredHasNoOriginAotTest, AnEventNoProcessorDeliveredHasNoOrigin) {
    SM sm;
    // The host's processor answers the document's send with a reply that names
    // no party, so the reply carries no origin. Registered before the machine
    // starts, as every host registers.
    int served = 0;
    sm.registerEventProcessor("x-sce-host", [&served](const ::SCE::HostSendRequest &) {
        ++served;
        return std::vector<::SCE::HostSendResponse>{{"host.reply", "", std::nullopt}};
    });
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    EXPECT_EQ(sm.initialize(), std::nullopt);
    sm.step();

    // The host raises an event of its own, naming no origin.
    sm.raiseExternal(SM::Event::Posted);
    sm.step();
    sm.step();

    EXPECT_EQ(served, 1) << "the host's processor was not handed the document's send once";
    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.postedBlank(), std::optional<int64_t>(1))
        << "postedBlank = " << show(sm.postedBlank()) << ": the host-posted event read an origin it was not given";
    EXPECT_EQ(sm.replyBlank(), std::optional<int64_t>(1))
        << "replyBlank = " << show(sm.replyBlank()) << ": the host's reply read an origin it was not given";
}

}  // namespace SCE::Tests
