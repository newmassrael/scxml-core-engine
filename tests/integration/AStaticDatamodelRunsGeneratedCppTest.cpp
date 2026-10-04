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

#include "static_block_ends_list_sm.h"
#include "static_block_ends_sm.h"
#include "static_counter_sm.h"
#include "static_donedata_content_sm.h"
#include "static_donedata_sm.h"
#include "static_enum_sm.h"
#include "static_event_arrival_sm.h"
#include "static_event_wildcard_sm.h"
#include "static_foreach_sm.h"
#include "static_host_call_arguments_sm.h"
#include "static_host_call_sm.h"
#include "static_invoke_params_sm.h"
#include "static_invoke_string_sm.h"
#include "static_list_sm.h"
#include "static_overflow_sm.h"
#include "static_payload_enum_sm.h"
#include "static_payload_relay_sm.h"
#include "static_payload_sm.h"
#include "static_real_sm.h"
#include "static_record_enum_sm.h"
#include "static_record_fields_sm.h"
#include "static_record_list_sm.h"
#include "static_record_real_sm.h"
#include "static_record_sm.h"
#include "static_send_namelist_sm.h"
#include "static_send_params_sm.h"
#include "static_string_capacity_sm.h"
#include "static_whole_payload_sm.h"
#include "static_wire_enum_sm.h"
#include "sync_client_sm.h"

#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using json = nlohmann::json;

const std::string kStaticFixtures = std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/static_datamodel";

json readScenario(const std::string &machine) {
    const auto path = std::filesystem::path(kStaticFixtures) / "scenarios" / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    return json::parse(in);
}

/// One generated machine as a scenario sees it: events by their document name,
/// and the published variables by theirs.
template <typename Machine> class Driver {
public:
    using Reader = std::function<json(const Machine &)>;

    explicit Driver(std::map<std::string, Reader> variables) : variables_(std::move(variables)) {}

    void start() {
        machine_.initialize();
    }

    /// An event goes in, and the macrostep it starts runs to the end. `data` is
    /// the event's payload as JSON text, from which the machine lifts the typed
    /// fields its schema names.
    void send(const std::string &name, const std::string &data = "") {
        machine_.raiseExternal(name, data);
        machine_.step();
    }

    /// Whether an event arriving under `name` reaches the machine at all, as the
    /// engine decides it (§scxml-3.12.1): false is a name it drops.
    bool resolves(const std::string &name) const {
        return machine_.resolveEventByName(name).has_value();
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine_.getCurrentState());
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

// A top-level final's `<donedata>` params are computed from the machine's own
// fields when it is entered, and the pair whose value does not fit is left out.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventItsParams) {
    using Machine = G::static_donedata::static_donedata;
    Driver<Machine> driver({
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_donedata", driver);
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
