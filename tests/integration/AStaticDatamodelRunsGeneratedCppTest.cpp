// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated
// C++: a variable is a member of the policy, every expression was lowered to C++
// at build time, and no script engine is built.
//
// The scenarios here are the ones Kotlin, Rust and the Interpreter replay —
// `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`, whose
// expected values are derived from the document, not observed from a backend —
// so the four engines are held to one answer. The machines are generated from
// the fixtures beside them by `tests/CMakeLists.txt`.
//
// What a failing scenario would say:
//
//   * `static_counter`: guards, `<assign>`, `<if>`/`<elseif>` and `In()` are
//     native, and `count` reaches the host as the `uint32` it is.
//   * `static_overflow`: a checked integer operation that overflows is a
//     failure, not a wrapped value (§3.4.1) — the variable keeps what it held,
//     `error.execution` is raised, and a guard over the overflowing sum is false.
//   * `static_block_ends`: an error ends the block it stands in (W3C SCXML 4.9)
//     — the statements after a failed `<assign>`, after a failed `<if>` cond,
//     or inside a branch that failed do not run, while the next block does.
//   * `static_host_call`: a host action takes the machine's variables as typed
//     arguments, and `static_host_call_arguments` (beside this file): an
//     argument that overflows stops the call and raises `error.execution`.

#include "common/SceClock.h"
#include "static_block_ends_list_sm.h"
#include "static_block_ends_sm.h"
#include "static_bytes_sm.h"
#include "static_bytes_wire_sm.h"
#include "static_cancel_expr_sm.h"
#include "static_counter_sm.h"
#include "static_donedata_content_sm.h"
#include "static_donedata_record_sm.h"
#include "static_donedata_sm.h"
#include "static_enum_sm.h"
#include "static_event_arrival_sm.h"
#include "static_event_wildcard_sm.h"
#include "static_foreach_sm.h"
#include "static_history_sm.h"
#include "static_host_call_arguments_sm.h"
#include "static_host_call_sm.h"
#include "static_invoke_hybrid_saved_sm.h"
#include "static_invoke_hybrid_sm.h"
#include "static_invoke_params_sm.h"
#include "static_invoke_sm.h"
#include "static_invoke_string_sm.h"
#include "static_list_sm.h"
#include "static_overflow_sm.h"
#include "static_payload_bytes_sm.h"
#include "static_payload_enum_sm.h"
#include "static_payload_relay_sm.h"
#include "static_payload_sm.h"
#include "static_real32_sm.h"
#include "static_real_sm.h"
#include "static_record_bytes_sm.h"
#include "static_record_enum_sm.h"
#include "static_record_fields_sm.h"
#include "static_record_list_sm.h"
#include "static_record_real32_sm.h"
#include "static_record_real_sm.h"
#include "static_record_sm.h"
#include "static_record_string_sm.h"
#include "static_send_content_sm.h"
#include "static_send_delay_sm.h"
#include "static_send_event_sm.h"
#include "static_send_http_sm.h"
#include "static_send_idlocation_sm.h"
#include "static_send_namelist_sm.h"
#include "static_send_params_sm.h"
#include "static_send_target_sm.h"
#include "static_send_type_sm.h"
#include "static_string_capacity_sm.h"
#include "static_timers_sm.h"
#include "static_whole_payload_sm.h"
#include "static_wire_enum_sm.h"
#include "sync_client_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <set>
#include <sstream>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using json = nlohmann::json;

const std::string kStaticFixtures = std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/static_datamodel";

/// A byte string as its byte-exact Latin-1 text, each byte the character of that
/// code point (docs/adr/0005, decision 2), written in the UTF-8 a JSON string is
/// held in: a byte past 0x7F is the two-byte sequence of its code point.
json latin1Text(const std::vector<std::uint8_t> &bytes) {
    std::string text;
    for (const std::uint8_t byte : bytes) {
        if (byte < 0x80) {
            text.push_back(static_cast<char>(byte));
        } else {
            text.push_back(static_cast<char>(0xC0 | (byte >> 6)));
            text.push_back(static_cast<char>(0x80 | (byte & 0x3F)));
        }
    }
    return text;
}

json readScenario(const std::string &machine) {
    const auto path = std::filesystem::path(kStaticFixtures) / "scenarios" / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    return json::parse(in);
}

/// How many rounds a host gives a machine that needs `tick` after an event before
/// it reads the machine back: two for each level of child session, and a round
/// that finds nothing to do changes nothing.
constexpr int kSettleTicks = 5;

/// One generated machine as a scenario sees it: events by their document name,
/// and the published variables by theirs.
template <typename Machine> class Driver {
public:
    using Reader = std::function<json(const Machine &)>;

    explicit Driver(std::map<std::string, Reader> variables) : variables_(std::move(variables)) {}

    /// The machine runs on a manual clock, so a delayed send waits exactly the
    /// time a step names (`advance`) and not the time the test happened to take.
    void start() {
        machine_.setClock(std::make_shared<SCE::ManualClock>(0));
        machine_.initialize();
        // A machine that starts a child session in its first state has that child
        // running, and maybe ended, before any event arrives; it is read as the
        // events leave it, after the rounds that take.
        if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
            settle();
        }
    }

    /// The machine's time moves on by `ms`, and what that made due runs.
    void advance(uint64_t ms) {
        machine_.advanceTimeMs(ms);
    }

    /// An event goes in, and the macrostep it starts runs to the end. `data` is
    /// the event's payload as JSON text, from which the machine lifts the typed
    /// fields its schema names.
    void send(const std::string &name, const std::string &data = "") {
        machine_.raiseExternal(name, data);
        settle();
    }

    /// Whether an event arriving under `name` reaches the machine at all, as the
    /// engine decides it (§scxml-3.12.1): false is a name it drops.
    bool resolves(const std::string &name) const {
        return machine_.resolveEventByName(name).has_value();
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine_.getCurrentState());
    }

    /// Every active state, a compound or a parallel one with the atomic ones below
    /// it, by the id the document gives it.
    std::set<std::string> configuration() const {
        std::set<std::string> ids;
        for (const auto &active : machine_.getActiveStates()) {
            ids.insert(Machine::PolicyType::getStateName(active));
        }
        return ids;
    }

    bool ended() const {
        return machine_.isInFinalState();
    }

    /// The data the top-level final's `<donedata>` left for the invoking parent,
    /// as the JSON text the done event carries.
    std::string donedata() const {
        return machine_.donedataAtFinal();
    }

    json variable(const std::string &name) const {
        const auto found = variables_.find(name);
        EXPECT_NE(found, variables_.end()) << "the driver publishes no variable '" << name << "'";
        return found == variables_.end() ? json() : found->second(machine_);
    }

private:
    /// The rounds the machine needs before it is read back. The generated policy
    /// says which call it needs: `step` drains the queues and nothing else, and a
    /// machine with a delayed send or a child session is driven by `tick`, which
    /// also runs those. The engine reports no point at which it has settled, so a
    /// host runs rounds: a child takes an event the parent forwarded in one round
    /// and its end reaches the parent in the next.
    void settle() {
        if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
            for (int round = 0; round < kSettleTicks; ++round) {
                machine_.tick();
            }
        } else {
            machine_.step();
        }
    }

    Machine machine_;
    std::map<std::string, Reader> variables_;
};

/// Replay `machine`'s scenario file against `driver`. Every step names an event
/// (or none, for the machine as started) and what it must hold afterwards. A name
/// no event of the machine matches is a misspelt step unless the step says it
/// expects the drop (`"dropped": true`), which is then what is held.
template <typename Machine> void replay(const std::string &machine, Driver<Machine> &driver) {
    const json scenario = readScenario(machine);
    ASSERT_TRUE(scenario.contains("steps")) << machine;
    driver.start();
    int index = 0;
    for (const auto &step : scenario["steps"]) {
        SCOPED_TRACE(machine + " step " + std::to_string(index++) + ": " + step.value("note", std::string{}));
        if (step.contains("advance_ms")) {
            driver.advance(step["advance_ms"].get<uint64_t>());
        }
        if (step.contains("event")) {
            const auto event = step["event"].get<std::string>();
            const bool dropped = step.value("dropped", false);
            EXPECT_EQ(driver.resolves(event), !dropped)
                << "the machine's events " << (dropped ? "match" : "do not match") << " '" << event << "'";
            driver.send(event, step.contains("data") ? step["data"].dump() : "");
        }
        const auto &expect = step["expect"];
        if (expect.value("ended", false)) {
            EXPECT_TRUE(driver.ended());
            // Compared as a value, so the order the pairs were written in is
            // not part of the answer.
            if (expect.contains("donedata")) {
                EXPECT_EQ(json::parse(driver.donedata()), expect["donedata"])
                    << "the data the final's done event carries";
            }
            continue;
        }
        if (expect.contains("state")) {
            EXPECT_EQ(driver.state(), expect["state"].get<std::string>());
        }
        // A set: the order a machine lists its active states in is not part of
        // the answer.
        if (expect.contains("configuration")) {
            const auto want = expect["configuration"].get<std::set<std::string>>();
            EXPECT_EQ(driver.configuration(), want) << "the active states";
        }
        if (expect.contains("variables")) {
            for (const auto &[name, value] : expect["variables"].items()) {
                EXPECT_EQ(driver.variable(name), value) << "variable '" << name << "'";
            }
        }
    }
}

/// A `record:Day` value as a scenario states it: its fields by the schema's ids.
template <typename Day> json dayJson(const Day &day) {
    return json{{"year", day.year}, {"month", day.month}, {"dayOfMonth", day.dayOfMonth}};
}

/// A `record:Reading` value as a scenario states it: its fields by the schema's ids.
template <typename Reading> json readingJson(const Reading &reading) {
    return json{{"sensor", reading.sensor}, {"value", reading.value}};
}

/// A list of such values, in order.
template <typename Days> json daysJson(const Days &days) {
    json listed = json::array();
    for (const auto &day : days) {
        listed.push_back(dayJson(day));
    }
    return listed;
}

/// A `record:Framed` value as a scenario states it: its fields by the schema's ids,
/// the byte string as its byte-exact Latin-1 text.
template <typename Framed> json framedJson(const Framed &framed) {
    return json{{"sensor", framed.sensor}, {"frame", latin1Text(framed.frame)}};
}

/// A list of such values, in order.
template <typename Frameds> json framedsJson(const Frameds &frameds) {
    json listed = json::array();
    for (const auto &framed : frameds) {
        listed.push_back(framedJson(framed));
    }
    return listed;
}

/// A `record:Labelled` value as a scenario states it: its fields by the schema's ids.
template <typename Labelled> json labelledJson(const Labelled &labelled) {
    return json{{"sensor", labelled.sensor}, {"label", labelled.label}};
}

/// A list of such values, in order.
template <typename Labelleds> json labelledsJson(const Labelleds &labelleds) {
    json listed = json::array();
    for (const auto &labelled : labelleds) {
        listed.push_back(labelledJson(labelled));
    }
    return listed;
}

}  // namespace

namespace G = SCE::Generated;

/// The counter's variables as a host reads them. One machine, two scenarios.
Driver<G::static_counter::static_counter> counterDriver() {
    using Machine = G::static_counter::static_counter;
    return Driver<Machine>({
        {"count", [](const Machine &m) { return json(m.count()); }},
        {"ready", [](const Machine &m) { return json(m.ready()); }},
    });
}

TEST(AStaticDatamodelRunsGeneratedCppTest, TheCounterCountsToItsFlagAndLetsGo) {
    auto driver = counterDriver();
    replay("static_counter", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, TheCounterStopsAtItsBoundAndRefusesGo) {
    auto driver = counterDriver();
    replay("static_counter_bound", driver);
}

/// An event arrives by name from outside the document, so the names it can arrive
/// under are open (§scxml-3.12.1): a name the document never writes reaches the
/// transition whose descriptor is a token prefix of it, and one no descriptor
/// matches is dropped.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventArrivesUnderANameTheDocumentDoesNotWrite) {
    using Machine = G::static_event_arrival::static_event_arrival;
    Driver<Machine> driver({
        {"requests", [](const Machine &m) { return json(m.requests()); }},
        {"specials", [](const Machine &m) { return json(m.specials()); }},
    });
    replay("static_event_arrival", driver);
}

/// ...and where the document listens with `event="*"`, a name no descriptor it
/// writes extends is delivered as the wildcard event instead of being dropped.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventArrivesUnderANameOnlyTheWildcardTakes) {
    using Machine = G::static_event_wildcard::static_event_wildcard;
    Driver<Machine> driver({
        {"requests", [](const Machine &m) { return json(m.requests()); }},
        {"strays", [](const Machine &m) { return json(m.strays()); }},
    });
    replay("static_event_wildcard", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnOverflowingOperationFailsInsteadOfWrapping) {
    using Machine = G::static_overflow::static_overflow;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_overflow", driver);
}

// An event's typed payload is read in a guard and in assignments, and a
// computation that does not fit skips its assignment and says so.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventsTypedPayloadIsReadInAGuardAndInContent) {
    using Machine = G::static_payload::static_payload;
    Driver<Machine> driver({
        {"day", [](const Machine &m) { return json(m.day()); }},
        {"late", [](const Machine &m) { return json(m.late()); }},
        {"sinceEpoch", [](const Machine &m) { return json(m.sinceEpoch()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_payload", driver);
}

// An enum variable holds a variant of its enum, is compared with `===` and
// `!==` to a variant or to another variable of the same enum, and takes a
// conditional of two variants. The scenario names a value as the enum document
// does, so the host's enum is read back by `sceLogName`.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEnumVariableHoldsAVariantOfItsEnum) {
    namespace E = G::static_enum;

    // `previous` is the machine's own, which the scenario states anyway: the
    // probe reaches the policy that holds it.
    struct Probe : E::static_enum {
        const E::static_enumPolicy &policy() const {
            return this->policy_;
        }
    };

    Driver<Probe> driver({
        {"layout", [](const Probe &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"previous", [](const Probe &m) { return json(std::string(E::sceLogName(m.policy().v_previous))); }},
        {"changes", [](const Probe &m) { return json(m.changes()); }},
    });
    replay("static_enum", driver);
}

// An event's payload carries an enum field, the variant's declared name: a guard
// compares it to a variant and an assignment stores it in a variable of the enum.
// The scenario names a value as the enum document does, so the host's enum is
// read back by `sceLogName`.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventsPayloadCarriesAnEnumField) {
    namespace E = G::static_payload_enum;
    using Machine = E::static_payload_enum;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"zoom", [](const Machine &m) { return json(m.zoom()); }},
        {"agenda", [](const Machine &m) { return json(m.agenda()); }},
        {"shown", [](const Machine &m) { return json(m.shown()); }},
    });
    replay("static_payload_enum", driver);
}

// The payload of the event a transition is on is carried on as the `<param>`s
// of a `<send>`: an enum field as the name its enum declares and an integer,
// read where the send runs.
TEST(AStaticDatamodelRunsGeneratedCppTest, ThePayloadOfAnEventIsCarriedOnAsParams) {
    namespace E = G::static_payload_relay;
    using Machine = E::static_payload_relay;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"zoom", [](const Machine &m) { return json(m.zoom()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_payload_relay", driver);
}

// A list is a bounded `std::vector`: appended to while it has room, and a full
// list is an error rather than a longer list; `len` measures it and a clear
// empties it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AListIsFilledToItsBoundAndEmptied) {
    using Machine = G::static_list::static_list;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_list", driver);
}

// A `<foreach>` walks the list as it was when the loop began, binds its item
// and index, and an error in its body ends the block that holds it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AForeachWalksAListVariable) {
    using Machine = G::static_foreach::static_foreach;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"weighted", [](const Machine &m) { return json(m.weighted()); }},
        {"small", [](const Machine &m) { return json(m.small()); }},
        {"crossings", [](const Machine &m) { return json(m.crossings()); }},
        {"visited", [](const Machine &m) { return json(m.visited()); }},
        {"finished", [](const Machine &m) { return json(m.finished()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_foreach", driver);
}

// A 64-bit real is a native double field: a product and a sum, a quotient, a
// guard comparing it with a literal, and a `<foreach>` summing a list of reals.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARealIsANativeBinary64Field) {
    using Machine = G::static_real::static_real;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"drift", [](const Machine &m) { return json(m.drift()); }},
        {"samples", [](const Machine &m) { return json(m.samples()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_real", driver);
}

// A 32-bit real is a native float field: every operation on it is rounded to
// binary32 where it is made. A host reads it as the double it widens to, which is
// exact, and that is the number the scenario states.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARealIsANativeBinary32Field) {
    using Machine = G::static_real32::static_real32;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(static_cast<double>(m.level())); }},
        {"drift", [](const Machine &m) { return json(static_cast<double>(m.drift())); }},
        {"wide", [](const Machine &m) { return json(m.wide()); }},
        {"total", [](const Machine &m) { return json(static_cast<double>(m.total())); }},
        // A list of floats: each element is stored by the JSON value as the
        // double it widens to.
        {"samples", [](const Machine &m) { return json(m.samples()); }},
    });
    replay("static_real32", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnAppendThatFailsEndsItsBlock) {
    using Machine = G::static_block_ends_list::static_block_ends_list;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"afterAppend", [](const Machine &m) { return json(m.afterAppend()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_block_ends_list", driver);
}

// A record variable is built whole from its `<sce:set>`s, read field by field,
// and updated a field at a time — from the machine's own value and from a typed
// event payload. A field assignment that does not fit is skipped, and the one
// after it in the same block is not processed.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordIsBuiltWholeAndUpdatedAFieldAtATime) {
    using Machine = G::static_record_fields::static_record_fields;
    Driver<Machine> driver({
        {"shown", [](const Machine &m) { return dayJson(m.shown()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_record_fields", driver);
}

// A record with a 64-bit real field is built whole, written a field at a time,
// and replaced from a typed payload without losing a bit of the real it carried.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsARealFieldToTheBit) {
    using Machine = G::static_record_real::static_record_real;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return readingJson(m.last()); }},
        {"sum", [](const Machine &m) { return json(m.sum()); }},
    });
    replay("static_record_real", driver);
}

// A record with a 32-bit real field: the double a payload carries lands as the
// single nearest it. A host reads a float as the double it widens to, which the
// JSON value does by itself.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsASingleFieldAsTheSingleNearestThePayload) {
    using Machine = G::static_record_real32::static_record_real32;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return readingJson(m.last()); }},
        {"sum", [](const Machine &m) { return json(static_cast<double>(m.sum())); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_real32", driver);
}

// A record's bytes field is held to the bytes its schema declares: an assignment
// past the bound — from a literal or a bytes variable — writes nothing, raises
// `error.execution` and ends its block, and a list of such records holds copies with
// their bytes. A scenario states a byte string as its byte-exact Latin-1 text.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsABytesFieldWithinTheBoundItsSchemaDeclares) {
    using Machine = G::static_record_bytes::static_record_bytes;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return framedJson(m.last()); }},
        {"spare", [](const Machine &m) { return latin1Text(m.spare()); }},
        {"frames", [](const Machine &m) { return framedsJson(m.frames()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"matches", [](const Machine &m) { return json(m.matches()); }},
        {"misses", [](const Machine &m) { return json(m.misses()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_bytes", driver);
}

// A host is handed a record by value, and the records of a list as a constant
// reference to the machine's own vector: it can write into the first only into its
// own copy, and into the second not at all. Checked where it is compiled, since a
// reader that returned a mutable reference would be another type.
static_assert(std::is_same_v<decltype(std::declval<const G::static_record_bytes::static_record_bytes &>().last()),
                             G::static_record_bytes::StaticRecordBytesFramedRecord>,
              "a published record is handed out by value");
static_assert(std::is_same_v<decltype(std::declval<const G::static_record_bytes::static_record_bytes &>().frames()),
                             const std::vector<G::static_record_bytes::StaticRecordBytesFramedRecord> &>,
              "a published list of records is lent as a constant reference");

// The bytes an event's payload carries are read into a bytes variable, into a record's
// field and into a whole record, each held to its own bound: a value past it writes
// nothing, raises `error.execution` and ends its block. The wire spells a byte string
// as its byte-exact Latin-1 text, which a scenario states it as.
TEST(AStaticDatamodelRunsGeneratedCppTest, ThePayloadOfAnEventCarriesBytesHeldWithinTheirBounds) {
    using Machine = G::static_payload_bytes::static_payload_bytes;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return framedJson(m.last()); }},
        {"held", [](const Machine &m) { return latin1Text(m.held()); }},
        {"frames", [](const Machine &m) { return framedsJson(m.frames()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"matches", [](const Machine &m) { return json(m.matches()); }},
        {"misses", [](const Machine &m) { return json(m.misses()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_payload_bytes", driver);
}

// A record's string field is held to the UTF-8 bytes its schema declares: an
// assignment past the bound — from a literal, a string variable or a payload —
// writes nothing, raises error.execution and ends its block, and a list of such
// records holds copies with their text.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsAStringFieldWithinTheBoundItsSchemaDeclares) {
    using Machine = G::static_record_string::static_record_string;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return labelledJson(m.last()); }},
        {"note", [](const Machine &m) { return json(m.note()); }},
        {"labels", [](const Machine &m) { return labelledsJson(m.labels()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_string", driver);
}

// A byte string crosses a `<param>` and a `<donedata>` as its byte-exact Latin-1 text:
// sent to itself and read back through the typed payload, and carried by the done
// event's data.
TEST(AStaticDatamodelRunsGeneratedCppTest, AByteStringCrossesAParamAsItsLatin1Text) {
    using Machine = G::static_bytes_wire::static_bytes_wire;
    Driver<Machine> driver({
        {"held", [](const Machine &m) { return latin1Text(m.held()); }},
        {"echo", [](const Machine &m) { return latin1Text(m.echo()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_bytes_wire", driver);
}

// A top-level final's `<donedata>` params are computed from the machine's own
// fields when it is entered, and the pair whose value does not fit is left out.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventItsParams) {
    using Machine = G::static_donedata::static_donedata;
    Driver<Machine> driver({
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_donedata", driver);
}

// A top-level final whose `<donedata>` names a record in its `<content expr>` hands
// its done event the pairs of the record's fields, read when the state is entered.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventTheRecordItsContentNames) {
    namespace E = G::static_donedata_record;
    using Machine = E::static_donedata_record;
    Driver<Machine> driver({
        {"shown",
         [](const Machine &m) {
             return json{{"layout", std::string(E::sceLogName(m.shown().layout))}, {"zoom", m.shown().zoom}};
         }},
    });
    replay("static_donedata_record", driver);
}

// A top-level final whose `<donedata>` is inline `<content>` hands its done event
// the text as the string it spells, with no script engine to read it as a number.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventTheTextItsContentSpells) {
    using Machine = G::static_donedata_content::static_donedata_content;
    Driver<Machine> driver({
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_donedata_content", driver);
}

// A top-level final whose `<donedata>` carries a `<content expr>` that names one
// value hands its done event that value as its whole data: a number as its digits,
// a string quoted, and one that cannot be computed as the empty string.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventTheValueItsContentNames) {
    using Machine = G::static_donedata_content::static_donedata_content;
    {
        Driver<Machine> driver({
            {"count", [](const Machine &m) { return json(m.count()); }},
        });
        replay("static_donedata_content_value", driver);
    }
    {
        Driver<Machine> driver({
            {"count", [](const Machine &m) { return json(m.count()); }},
        });
        replay("static_donedata_content_text", driver);
    }
    {
        Driver<Machine> driver({
            {"count", [](const Machine &m) { return json(m.count()); }},
        });
        replay("static_donedata_content_lost", driver);
    }
}

// The `eventexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, and names the event the send delivers.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsEventIsNamedWhenItRuns) {
    using Machine = G::static_send_event::static_send_event;
    Driver<Machine> driver({
        {"pings", [](const Machine &m) { return json(m.pings()); }},
        {"pongs", [](const Machine &m) { return json(m.pongs()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_event", driver);
}

// The `targetexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, held to the routes the document declares as `sce:targets`
// (docs/adr/0005, decision 3): the send goes by the entry that matches, and a value
// in none of them is error.communication with nothing sent.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsTargetIsChosenAmongTheDeclaredRoutes) {
    using Machine = G::static_send_target::static_send_target;
    Driver<Machine> driver({
        {"landed", [](const Machine &m) { return json(m.landed()); }},
        {"refused", [](const Machine &m) { return json(m.refused()); }},
    });
    replay("static_send_target", driver);
}

// The `typeexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, held to the processors the document declares as `sce:types`
// (docs/adr/0005, decision 3): the send is delivered by the processor the matching
// entry names, and a value in none of them is error.execution with nothing sent.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsTypeIsChosenAmongTheDeclaredProcessors) {
    using Machine = G::static_send_type::static_send_type;
    Driver<Machine> driver({
        {"landed", [](const Machine &m) { return json(m.landed()); }},
        {"refused", [](const Machine &m) { return json(m.refused()); }},
    });
    replay("static_send_type", driver);
}

// The `<param>`s of a BasicHTTP `<send>` are read from the machine's own fields when
// the send runs, and cross as the text a form carries (docs/adr/0005, decision 4,
// §scxml-C-2). The request is observed where the engine hands it to its transport,
// so no listener is involved. A scenario states what a machine's fields hold and not
// what it sent over a wire, so `static_send_http` has no scenario: each engine holds
// it with a test of its own that records the request.
namespace {

using HttpMachine = G::static_send_http::static_send_http;
using HttpParams = std::map<std::string, std::vector<std::string>>;

/// A machine whose transport keeps what it was handed.
struct HttpHarness {
    HttpMachine sm;
    std::vector<SCE::Static::HttpSendRequest> posted;

    HttpHarness() {
        sm.setHttpSendCallback([this](const SCE::Static::HttpSendRequest &request) { posted.push_back(request); });
        sm.initialize();
        sm.step();
    }

    void raise(const std::string &event) {
        sm.raiseExternal(event);
        sm.step();
    }
};

}  // namespace

TEST(AStaticDatamodelRunsGeneratedCppTest, ASendOverHttpCarriesTheTextTheFieldsHoldWhenItRuns) {
    HttpHarness h;
    h.raise("bump");
    h.raise("go");

    ASSERT_EQ(h.posted.size(), 1u) << "one request is handed to the transport";
    const auto &request = h.posted[0];
    EXPECT_EQ(request.target, "http://example.invalid/hook");
    EXPECT_EQ(request.eventName, "note");
    EXPECT_EQ(request.content, "") << "no <content>, so the body is the pairs";
    const HttpParams wanted = {{"count", {"4"}}, {"ready", {"true"}}, {"label", {"busy"}},
                               {"twice", {"8"}}, {"delta", {"-5"}},   {"ratio", {"1.5"}}};
    EXPECT_EQ(request.params, wanted) << "each value is the text it spells: an integer's digits, `true`, the "
                                         "string, a negative number, a real's String()";
}

TEST(AStaticDatamodelRunsGeneratedCppTest, TheHttpPairsAreTheFieldsAsTheyStandAndNotACopyFromStartUp) {
    HttpHarness h;
    h.raise("go");

    ASSERT_EQ(h.posted.size(), 1u);
    const HttpParams wanted = {{"count", {"3"}}, {"ready", {"false"}}, {"label", {"idle"}},
                               {"twice", {"6"}}, {"delta", {"-5"}},    {"ratio", {"1.5"}}};
    EXPECT_EQ(h.posted[0].params, wanted) << "without `bump` the fields hold their initial values, and the request "
                                             "carries those: it is read when the send runs";
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AHttpParamThatCannotBeReadIsLeftOutAndTheRequestStillGoes) {
    HttpHarness h;
    h.raise("bump");
    h.raise("boom");

    ASSERT_EQ(h.posted.size(), 1u) << "the request goes with the pair that could be read";
    const HttpParams wanted = {{"count", {"4"}}};
    EXPECT_EQ(h.posted[0].params, wanted) << "`big` is `count * 2000000000`, which a 32-bit field cannot hold: its "
                                             "pair is left out, not carried as a zero";
    EXPECT_EQ(h.sm.errors(), 1u) << "§scxml-5.7.1: the failed pair is reported as error.execution, once";
}

// The `delayexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, and read as the CSS2 time it must be; the machine runs on a manual
// clock, which the scenario's `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsDelayIsComputedWhenItRuns) {
    using Machine = G::static_send_delay::static_send_delay;
    Driver<Machine> driver({
        {"wait", [](const Machine &m) { return json(m.wait()); }},
        {"beats", [](const Machine &m) { return json(m.beats()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_delay", driver);
}

// The `sendidexpr` of a `<cancel>` is a string computed from the machine's fields
// when the cancel runs, the id of the delayed send it removes; the machine runs on
// a manual clock, which the scenario's `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, ACancelRemovesTheSendItsIdNames) {
    using Machine = G::static_cancel_expr::static_cancel_expr;
    Driver<Machine> driver({
        {"a_fired", [](const Machine &m) { return json(m.a_fired()); }},
        {"b_fired", [](const Machine &m) { return json(m.b_fired()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_cancel_expr", driver);
}

// A child session an `<invoke>` started is driven through its parent by
// autoforward, takes the events it waits for in order, and its end reaches the
// parent as `done.invoke`, which the parent counts.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnInvokedChildIsDrivenAndCounted) {
    using Machine = G::static_invoke::static_invoke;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
    });
    replay("static_invoke", driver);
}

// An `<invoke>` hands its child the values its `<param>`s and `namelist` name, as
// they stand when the invoke executes, after the entry actions, and once.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnInvokeHandsItsChildItsValuesOnce) {
    using Machine = G::static_invoke_params::static_invoke_params;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
    });
    replay("static_invoke_params", driver);
}

// A string an `<invoke>` hands its child is held to the bound the child declared,
// in bytes: a value past it is left out and raises `error.execution`, and the
// child still starts.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStringHandedToAChildIsHeldToItsBoundInBytes) {
    using Machine = G::static_invoke_string::static_invoke_string;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_invoke_string", driver);
}

// Leaving the state that holds an `<invoke>` cancels the child, which then ends
// nothing and counts nothing.
TEST(AStaticDatamodelRunsGeneratedCppTest, LeavingTheStateOfAnInvokeCancelsTheChild) {
    using Machine = G::static_invoke::static_invoke;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
    });
    replay("static_invoke_abort", driver);
}

// A `<history>` remembers what its parent held when it was left, and entering it
// brings that back: the shallow one the child that was active, the deep one the
// atomic states below, and a deep one of a `<parallel>` both regions at once. The
// scenario states the whole active configuration of each step.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHistoryBringsBackWhatItsParentHeld) {
    using Machine = G::static_history::static_history;
    Driver<Machine> driver({
        {"resumed", [](const Machine &m) { return json(m.resumed()); }},
    });
    replay("static_history", driver);
}

// Four delayed sends armed on entering a state are delivered when each is due, two
// due the same moment in the order they were sent, and the last takes the machine
// to its final state; the machine runs on a manual clock, which the scenario's
// `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, TimersDeliverEachSendWhenItIsDue) {
    using Machine = G::static_timers::static_timers;
    Driver<Machine> driver({
        {"trace", [](const Machine &m) { return json(m.trace()); }},
    });
    replay("static_timers", driver);
}

// The same machine stopped: a `<cancel>` by the id of the longest send removes that
// one and no other, so the machine never ends.
TEST(AStaticDatamodelRunsGeneratedCppTest, StoppingCancelsTheSendItsIdNamesAndNoOther) {
    using Machine = G::static_timers::static_timers;
    Driver<Machine> driver({
        {"trace", [](const Machine &m) { return json(m.trace()); }},
    });
    replay("static_timers_stop", driver);
}

// The `idlocation` of a `<send>` names a string variable the machine writes the id
// it generates for the send to, which a later `<cancel sendidexpr>` names; the
// machine runs on a manual clock, which the scenario's `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendHandsTheDocumentAnIdACancelCanName) {
    using Machine = G::static_send_idlocation::static_send_idlocation;
    Driver<Machine> driver({
        {"first_fired", [](const Machine &m) { return json(m.first_fired()); }},
        {"second_fired", [](const Machine &m) { return json(m.second_fired()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_idlocation", driver);
}

// The `<content expr>` of a `<send>` names a record, which crosses as the pairs of
// its fields: a record variable and the payload of the event the transition is
// on, taken whole.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendCarriesTheRecordItsContentNames) {
    namespace E = G::static_send_content;
    using Machine = E::static_send_content;
    Driver<Machine> driver({
        {"shown",
         [](const Machine &m) {
             return json{{"layout", std::string(E::sceLogName(m.shown().layout))}, {"zoom", m.shown().zoom}};
         }},
        {"received", [](const Machine &m) { return json(std::string(E::sceLogName(m.received()))); }},
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
    });
    replay("static_send_content", driver);
}

// The `namelist` of a `<send>` names variables the machine holds, each carried as
// the pair `<param name="x" expr="x"/>` it abbreviates, an enum value among them
// as the name its enum declares.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendCarriesTheVariablesItsNamelistNames) {
    namespace E = G::static_send_namelist;
    using Machine = E::static_send_namelist;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"zoom", [](const Machine &m) { return json(m.zoom()); }},
        {"received", [](const Machine &m) { return json(std::string(E::sceLogName(m.received()))); }},
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"deliveries", [](const Machine &m) { return json(m.deliveries()); }},
    });
    replay("static_send_namelist", driver);
}

// A `<send>` hands its event the pairs of its `<param>`s, each computed from the
// machine's own fields when the send runs. A pair whose value does not fit is
// reported and left out, the message still goes, and the receiver, finding the
// field missing, raises an `error.execution` of its own and takes no transition.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendHandsItsEventThePairsOfItsParams) {
    using Machine = G::static_send_params::static_send_params;
    Driver<Machine> driver({
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"ok", [](const Machine &m) { return json(m.ok()); }},
        {"tag", [](const Machine &m) { return json(m.tag()); }},
        {"partialTotal", [](const Machine &m) { return json(m.partialTotal()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_params", driver);
}

// A string variable is held to the UTF-8 bytes it declares: an assignment past the
// bound writes nothing, raises `error.execution` and ends its block, and the bound
// is a count of bytes — a `std::string` is made of them — not of characters.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStringIsHeldToItsBytes) {
    using Machine = G::static_string_capacity::static_string_capacity;
    Driver<Machine> driver({
        {"title", [](const Machine &m) { return json(m.title()); }},
        {"body", [](const Machine &m) { return json(m.body()); }},
        {"copied", [](const Machine &m) { return json(m.copied()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_string_capacity", driver);
}

// A bytes variable is held to the bytes it declares, as a string is to its UTF-8
// bytes: an assignment past the bound writes nothing, raises `error.execution` and
// ends its block. A scenario states a byte string as its byte-exact Latin-1 text.
TEST(AStaticDatamodelRunsGeneratedCppTest, AByteStringIsHeldToItsBound) {
    using Machine = G::static_bytes::static_bytes;
    Driver<Machine> driver({
        {"frame", [](const Machine &m) { return latin1Text(m.frame()); }},
        {"tail", [](const Machine &m) { return latin1Text(m.tail()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"matches", [](const Machine &m) { return json(m.matches()); }},
        {"misses", [](const Machine &m) { return json(m.misses()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_bytes", driver);
}

// A host is handed a byte string as a constant reference: the vector is the
// machine's own, and a write into it would change the variable behind its bound.
// Checked where it is compiled, since a reader that returned a copy or a mutable
// reference would be another type.
static_assert(std::is_same_v<decltype(std::declval<const G::static_bytes::static_bytes &>().frame()),
                             const std::vector<std::uint8_t> &>,
              "a published byte string is lent as a constant reference");

// An `<invoke type="scxml">` hands its child the values its `<param>`s and
// `namelist` name (§scxml-6.4.1), each to the child's variable of the same
// name. `worker` ends the moment it holds `start = 7` (a `<param>` reading
// `base`, which is 4 when `working` is entered and 7 once its entry action ran:
// the value is read when the invoke executes, at the end of the macrostep) and
// `enabled = true` (the `namelist`); handed less, it would wait and the parent
// would stay in `working`. `control`, the same child handed nothing, keeps what
// its `<data>` gave it and never ends.
class AStaticChildIsHandedItsParams : public ::testing::Test {
protected:
    using Machine = G::static_invoke_params::static_invoke_params;

    /// Let the child run and report to its parent.
    void settle() {
        for (int i = 0; i < 5; ++i) {
            machine.tick();
        }
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine.getCurrentState());
    }

    void SetUp() override {
        machine.initialize();
        settle();
    }

    Machine machine;
};

TEST_F(AStaticChildIsHandedItsParams, AChildEndsOnceItHoldsTheValuesItsInvokeNames) {
    // `worker` ended, so it held both values: the parent left `working` and
    // counted it.
    EXPECT_EQ(state(), "plain");
    EXPECT_EQ(machine.completed(), 1u);
}

TEST_F(AStaticChildIsHandedItsParams, AChildHandedNothingKeepsTheValuesItsDataGaveIt) {
    settle();
    // `control` still waits for 7 and true, so `done.invoke.control` never
    // counted.
    EXPECT_EQ(state(), "plain");
    EXPECT_EQ(machine.completed(), 1u);
}

// A string an `<invoke type="scxml">` hands its child is held to the bound the
// child declared for that variable, in UTF-8 bytes, as an `<assign>` to it would
// be: a value past it is left out and reported (§scxml-5.7.1), and the child
// still starts, holding the one its `<data>` gave it. `fits` is handed the four
// bytes its `title` holds and ends on them; `over` is handed eight bytes and
// `wide` two characters of five bytes, so each starts with the 'ab' it ends on:
// 1 + 10 + 100, and two errors.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStringHandedToAChildIsHeldToTheChildsBound) {
    G::static_invoke_string::static_invoke_string machine;
    machine.initialize();
    for (int i = 0; i < 5; ++i) {
        machine.tick();
    }
    EXPECT_EQ(machine.completed(), 111u);
    EXPECT_EQ(machine.errors(), 2u);
}

// An `<invoke srcexpr>` that declares `sce:candidates` starts the document its
// value names (§scxml-6.4), by the document's stem, and hands it the invoke's
// arguments, each to the variable of the same name that candidate declares
// (§scxml-6.4.3). The fixture runs four phases: `first` (a `file:` value),
// `second` (an absolute path), `lossy` (an `extra` no 32-bit field can hold:
// reported, left out, the child still ends) and `missing` (a document the invoke
// did not declare: reported, nothing starts). Each `done.invoke` adds a power of
// ten of its own, so 111 says which candidates ended, and the two errors are the
// `lossy` argument and the `missing` document. A candidate handed what the OTHER
// declares would never end.
class AStaticHybridInvokeStartsTheCandidateItsValueNames : public ::testing::Test {
protected:
    using Machine = G::static_invoke_hybrid::static_invoke_hybrid;

    void SetUp() override {
        machine.initialize();
        for (int i = 0; i < 40; ++i) {
            machine.tick();
        }
    }

    Machine machine;
};

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, EachPhaseStartsTheCandidateItsValueNamesAndEnds) {
    EXPECT_EQ(machine.completed(), 111u);
    EXPECT_TRUE(machine.isInFinalState()) << "the last phase named no declared candidate, so the run is over";
}

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, AnArgumentIsEvaluatedWhateverTheCandidateKeeps) {
    EXPECT_EQ(machine.errors(), 2u);
}

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, AValueNamingNoDeclaredCandidateStartsNothing) {
    // `done.invoke.missing_run` would add 1000: nothing started to send it.
    EXPECT_LT(machine.completed(), 1000u);
}

// A hybrid `<invoke>` of a document that never mentions `error.execution` has no
// such event in its enum, and what the invoke does where it would raise one
// (a value naming no declared candidate, an argument that cannot be computed) is
// to start nothing and say nothing, not to name a member the document never
// declared. `static_invoke_hybrid_saved` is such a document: the watcher it
// starts is handed 7 and waits for an 8 nothing gives it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHybridInvokeOfADocumentWithNoErrorEventStartsItsCandidate) {
    G::static_invoke_hybrid_saved::static_invoke_hybrid_saved machine;
    machine.initialize();
    for (int i = 0; i < 5; ++i) {
        machine.tick();
    }
    EXPECT_EQ(machine.completed(), 0u);
    EXPECT_FALSE(machine.isInFinalState());
}

// A guard calls an imported algorithm with the record's own fields, and the
// bound it answers is computed on every step, not folded to a constant.
TEST(AStaticDatamodelRunsGeneratedCppTest, AGuardCallsAnImportedAlgorithmWithARecordsFields) {
    using Machine = G::static_record::static_record;
    Driver<Machine> driver({
        {"shown", [](const Machine &m) { return dayJson(m.shown()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_record", driver);
}

// A sync run composed of the standard sync rules: each rule an imported
// algorithm that can fail, called in a guard and in an assignment with the
// event's typed payload as its arguments.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASyncRunIsComposedOfTheStandardAlgorithms) {
    using Machine = G::sync_client::sync_client;
    Driver<Machine> driver({
        {"byToken", [](const Machine &m) { return json(m.byToken()); }},
        {"fullListing", [](const Machine &m) { return json(m.fullListing()); }},
        {"outcome", [](const Machine &m) { return json(m.outcome()); }},
        {"retryAt", [](const Machine &m) { return json(m.retryAt()); }},
        {"deleted", [](const Machine &m) { return json(m.deleted()); }},
        {"uploaded", [](const Machine &m) { return json(m.uploaded()); }},
        {"discarded", [](const Machine &m) { return json(m.discarded()); }},
        {"pages", [](const Machine &m) { return json(m.pages()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("sync_client", driver);
}

// A list of records is filled by name from a record variable or a loop's item,
// walked by a `<foreach>`, and a record is taken whole.
TEST(AStaticDatamodelRunsGeneratedCppTest, AListHoldsRecordsAndAForeachWalksThem) {
    using Machine = G::static_record_list::static_record_list;
    Driver<Machine> driver({
        {"days", [](const Machine &m) { return daysJson(m.days()); }},
        {"copies", [](const Machine &m) { return daysJson(m.copies()); }},
        {"draft", [](const Machine &m) { return dayJson(m.draft()); }},
        {"last", [](const Machine &m) { return dayJson(m.last()); }},
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_list", driver);
}

// A record may hold an enum: its field is read back by the name the enum
// document declares.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsAnEnumField) {
    namespace E = G::static_record_enum;
    using Machine = E::static_record_enum;
    auto viewJson = [](const auto &view) {
        return json{{"layout", std::string(E::sceLogName(view.layout))}, {"zoom", view.zoom}};
    };
    Driver<Machine> driver({
        {"shown", [&](const Machine &m) { return viewJson(m.shown()); }},
        {"seen",
         [&](const Machine &m) {
             json listed = json::array();
             for (const auto &view : m.seen()) {
                 listed.push_back(viewJson(view));
             }
             return listed;
         }},
        {"weeks", [](const Machine &m) { return json(m.weeks()); }},
        {"flips", [](const Machine &m) { return json(m.flips()); }},
    });
    replay("static_record_enum", driver);
}

// The payload of an event is a record of its schema taken whole: it replaces a
// record variable in one assignment and is appended whole to a list, the enum
// field read back by the name the enum document declares.
TEST(AStaticDatamodelRunsGeneratedCppTest, ThePayloadOfAnEventIsTakenWholeAsARecord) {
    namespace E = G::static_whole_payload;
    using Machine = E::static_whole_payload;
    auto viewJson = [](const auto &view) {
        return json{{"layout", std::string(E::sceLogName(view.layout))}, {"zoom", view.zoom}};
    };
    Driver<Machine> driver({
        {"shown", [&](const Machine &m) { return viewJson(m.shown()); }},
        {"seen",
         [&](const Machine &m) {
             json listed = json::array();
             for (const auto &view : m.seen()) {
                 listed.push_back(viewJson(view));
             }
             return listed;
         }},
        {"updates", [](const Machine &m) { return json(m.updates()); }},
        {"agendas", [](const Machine &m) { return json(m.agendas()); }},
        {"others", [](const Machine &m) { return json(m.others()); }},
    });
    replay("static_whole_payload", driver);
}

// An enum value as a `<param>` crosses as the name its enum declares for it: a
// variable, a field of a record variable and a conditional, sent and read back
// through the schema.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEnumValueCrossesAsTheNameItsEnumDeclares) {
    namespace E = G::static_wire_enum;
    using Machine = E::static_wire_enum;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"shown",
         [](const Machine &m) {
             return json{{"layout", std::string(E::sceLogName(m.shown().layout))}, {"zoom", m.shown().zoom}};
         }},
        {"received", [](const Machine &m) { return json(std::string(E::sceLogName(m.received()))); }},
        {"deliveries", [](const Machine &m) { return json(m.deliveries()); }},
    });
    replay("static_wire_enum", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnErrorEndsTheBlockItStandsIn) {
    using Machine = G::static_block_ends::static_block_ends;
    Driver<Machine> driver({
        {"a", [](const Machine &m) { return json(m.a()); }},
        {"b", [](const Machine &m) { return json(m.b()); }},
        {"afterAssign", [](const Machine &m) { return json(m.afterAssign()); }},
        {"thenRan", [](const Machine &m) { return json(m.thenRan()); }},
        {"elseRan", [](const Machine &m) { return json(m.elseRan()); }},
        {"afterIf", [](const Machine &m) { return json(m.afterIf()); }},
        {"inBranch", [](const Machine &m) { return json(m.inBranch()); }},
        {"afterBranch", [](const Machine &m) { return json(m.afterBranch()); }},
        {"afterOk", [](const Machine &m) { return json(m.afterOk()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_block_ends", driver);
}

namespace {

/// What a host recorded of the calls a machine made, as `(name, value…)` text.
using Calls = std::vector<std::string>;

}  // namespace

// A host action takes the machine's variables as typed arguments, each read
// when the call is made: one call per entry of `idle`, with the datamodel as it
// stood.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHostActionTakesTypedDatamodelArguments) {
    namespace Hc = G::static_host_call;

    struct Host : Hc::StaticHostCallActions {
        Calls calls;

        void showAttempts(uint32_t count, bool exhausted) override {
            calls.push_back(std::to_string(count) + (exhausted ? ",true" : ",false"));
        }
    } host;

    Hc::static_host_call machine(host);
    machine.initialize();
    for (int i = 0; i < 4; ++i) {
        machine.raiseExternal("retry");
        machine.step();
    }
    // The fourth retry finds `attempts < 3` false and re-enters nothing.
    EXPECT_EQ(host.calls, (Calls{"0,false", "1,false", "2,false", "3,true"}));
}

// An argument that cannot be computed is a failure, not a wrapped value: the
// host is not called, and `error.execution` is raised in the call's place.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnArgumentThatOverflowsStopsTheCallAndRaisesAnError) {
    namespace Hc = G::static_host_call_arguments;

    struct Host : Hc::StaticHostCallArgumentsActions {
        Calls calls;

        void report(uint8_t next) override {
            calls.push_back(std::to_string(next));
        }
    } host;

    Hc::static_host_call_arguments machine(host);
    machine.initialize();

    machine.raiseExternal("fine");
    machine.step();
    EXPECT_EQ(host.calls, (Calls{"251"})) << "250 + 1 fits a uint8";
    EXPECT_EQ(machine.errors(), 0);

    machine.raiseExternal("overflow");
    machine.step();
    EXPECT_EQ(host.calls, (Calls{"251"})) << "250 + 10 does not fit, so the host is not called with 4";
    EXPECT_EQ(machine.errors(), 1) << "error.execution was raised and the machine saw it";
}

}  // namespace Tests
}  // namespace SCE
