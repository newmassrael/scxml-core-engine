// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reference harness (docs/adr/0087, section 6) run by a host program: the
// statechart asks its host's model for one record of an answer schema, reads the
// answer with `<sce:lift>`, checks it, and asks again with a repair prompt until it
// holds a record it accepts or gives up.
//
// The documents are the ones the repository's tests replay, read in place
// (sce-build/tests/fixtures/static_datamodel/static_model_harness.scxml and what it
// imports), so there is one copy of each. The host is the C++ reference host of the
// standard host features (sce/include/host/StandardHost.h) over a scripted provider:
// this program reaches no model. A host with one gives the reference host a provider
// of its own (`SCE::StandardHost::ModelProvider`), which is handed the prompt and the
// answer schema's id, and finds the schema's JSON Schema and GBNF grammar in the
// machine's table (`ANSWER_SCHEMAS`).
//
// The provider answers twice: first a record whose title is past its 16 bytes, which
// the lift refuses as `tooLong`, then one the harness accepts. The program prints
// each prompt the harness sent and how the run ended, and exits non-zero when the run
// did not end as the documents say it does.

#include "common/SceClock.h"
#include "host/StandardHost.h"
#include "static_model_harness_sm.h"

#include <cstdint>
#include <iostream>
#include <iterator>
#include <memory>
#include <string>
#include <vector>

namespace {

namespace host = SCE::StandardHost;

/// The transport of a host that makes no HTTP exchange: the harness asks for none.
class NoTransport : public host::Transport {
public:
    host::Exchange exchange(const host::HttpRequest &) override {
        return {host::Exchange::Kind::Failure, {}};
    }
};

}  // namespace

int main() {
    using Harness = SCE::Generated::static_model_harness::static_model_harness;

    host::ScriptedClock clock;
    NoTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::ScriptedProvider provider;
    // A title of 15 ASCII bytes and one two-byte character, U+00E9: 17 bytes, one past its bound.
    const std::string tooLong = std::string(R"({"id":7,"amount":-42,"title":"abcdefghijklmno)") + "\xC3\xA9" +
                                R"(","kind":"task","note":null,"tag":"ab","priority":1})";
    const std::string accepted =
        R"({"amount":-42,"id":7,"kind":"task","note":null,"priority":1,"tag":"ab","title":"Plan"})";
    provider.answers.push_back({tooLong, "stop"});
    provider.answers.push_back({accepted, "stop"});
    reference.setModelProvider(provider);
    // The machine's table of answer schemas: the host resolves the id the build wrote into
    // each request (`sce:answer`) in it.
    const std::vector<SCE::AnswerSchema> schemas(std::begin(SCE::Generated::static_model_harness::ANSWER_SCHEMAS),
                                                 std::end(SCE::Generated::static_model_harness::ANSWER_SCHEMAS));
    reference.setAnswerSchemas(schemas.data(), schemas.size());

    Harness harness;
    host::serve(harness, reference);
    harness.setClock(std::make_shared<SCE::ManualClock>(0));
    // A `datamodel="sce-static"` machine has no script session whose failure
    // could refuse its start (docs/adr/0096), so the start answers nothing.
    static_assert(!Harness::START_MAY_BE_REFUSED, "a sce-static machine's start cannot be refused");
    static_cast<void>(harness.initialize());
    // A run makes its calls and takes each completion within one event, a call a round.
    const auto settle = [&harness]() {
        for (int round = 0; round < 8; ++round) {
            harness.tick();
        }
    };
    settle();
    harness.raiseExternal("harness.offer", R"({"id":7})");
    settle();
    harness.raiseExternal("harness.start", R"({"request":"Plan my day"})");
    settle();

    int call = 0;
    for (const auto &start : reference.takeStarts()) {
        const auto prompt = start.params.find("prompt");
        std::cout << "--- call " << ++call << " (" << start.operation << ")\n"
                  << (prompt != start.params.end() ? prompt->second : std::string()) << "\n";
    }
    const std::string state(Harness::PolicyType::getStateName(harness.getCurrentState()));
    const std::string outcome(harness.outcome());
    const auto attempts = harness.attempts();
    std::cout << "--- the run ended in `" << state << "`: outcome `" << outcome << "`, " << attempts
              << " attempt(s) after the first\n";

    // The documents say: the first answer is refused and repaired, the second accepted.
    const bool asSaid = call == 2 && state == "accepted" && outcome == "accepted" && attempts == 1;
    if (!asSaid) {
        std::cerr << "model_harness_example: the run did not end as the documents say\n";
    }
    return asSaid ? 0 : 1;
}
