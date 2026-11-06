// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045, section 8): the
// saved states the shared fixtures hold are the text every backend writes and reads, so
// a state saved by one backend is one another restores. Each format-3 text is restored
// by the generated C++ machine of its document and saved again, which is the same text
// byte for byte; each format-1 text, which every backend still reads, is restored; and
// each format-2 text a build before format 3 wrote is restored and saved as the format-3
// text it is read as (docs/adr/0090, section 7).
//
// The Rust, Kotlin, Go and Python halves are `static_datamodel.rs`,
// `StaticDatamodelTest.kt`, `a_state_another_backend_saved_is_restored_test.go` and
// `test_a_state_another_backend_saved_is_restored.py`; the shared texts are
// `sce-build/tests/fixtures/static_datamodel/saved/`.

#include "common/ForwardedEvent.h"
#include "common/SceClock.h"
#include "core/EventOrigin.h"
#include "saved/SavedState.h"
#include "static_enum_sm.h"
#include "static_history_sm.h"
#include "static_invoke_sm.h"
#include "static_invoke_tree_sm.h"
#include "static_list_sm.h"
#include "static_optional_record_sm.h"
#include "static_optional_sm.h"
#include "static_record_sm.h"
#include "static_schema_pending_sm.h"
#include "static_schema_revision_sm.h"
#include "static_send_idlocation_sm.h"
#include "static_timers_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <optional>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

namespace G = ::SCE::Generated;

/// The wall-clock moment the shared fixtures were saved at, as the Rust, Kotlin, Go and
/// Python suites have it: 2023-11-14T22:13:20Z.
constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

const std::filesystem::path kSaved =
    std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved";

/// The text a machine of `Machine` saves after it restored `text`, at the moment it was
/// restored at.
template <typename Machine> std::string restoredAndSavedAgain(const std::string &text) {
    Machine machine;
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    machine.restoreAt(::SCE::Saved::SavedState::fromJson(text), kSavedAtMs);
    return machine.saveAt(kSavedAtMs).toJson();
}

using Restorer = std::function<std::string(const std::string &)>;

/// The documents a shared saved state is of, by the stem the file's name begins with.
const std::map<std::string, Restorer> &machines() {
    static const std::map<std::string, Restorer> all = {
        {"static_enum", restoredAndSavedAgain<G::static_enum::static_enum>},
        {"static_history", restoredAndSavedAgain<G::static_history::static_history>},
        {"static_invoke", restoredAndSavedAgain<G::static_invoke::static_invoke>},
        {"static_invoke_tree", restoredAndSavedAgain<G::static_invoke_tree::static_invoke_tree>},
        {"static_list", restoredAndSavedAgain<G::static_list::static_list>},
        {"static_optional", restoredAndSavedAgain<G::static_optional::static_optional>},
        {"static_optional_record", restoredAndSavedAgain<G::static_optional_record::static_optional_record>},
        {"static_record", restoredAndSavedAgain<G::static_record::static_record>},
        {"static_schema_pending", restoredAndSavedAgain<G::static_schema_pending::static_schema_pending>},
        {"static_schema_revision", restoredAndSavedAgain<G::static_schema_revision::static_schema_revision>},
        {"static_send_idlocation", restoredAndSavedAgain<G::static_send_idlocation::static_send_idlocation>},
        {"static_timers", restoredAndSavedAgain<G::static_timers::static_timers>},
    };
    return all;
}

/// The stem of the document a shared text is of: the file's own name up to the part that
/// says which moment of the run it is (`static_schema_pending_first` and
/// `static_schema_revision_second` are moments of one document each); empty for a text of
/// no document this suite holds.
std::string machineOf(const std::string &name) {
    // The longest stem that names it: `static_invoke_tree_working` is a moment of
    // `static_invoke_tree`, though `static_invoke` begins it too.
    std::string found;
    for (const auto &[stem, restorer] : machines()) {
        if ((name == stem || name.rfind(stem + "_", 0) == 0) && stem.size() > found.size()) {
            found = stem;
        }
    }
    return found;
}

/// Every shared text of `directory` this suite holds a machine of, by the file's name
/// without its extension, each as the file holds it without its closing newline.
std::map<std::string, std::string> textsOf(const std::filesystem::path &directory) {
    std::map<std::string, std::string> texts;
    for (const auto &entry : std::filesystem::directory_iterator(directory)) {
        if (!entry.is_regular_file() || entry.path().extension() != ".json" ||
            machineOf(entry.path().stem().string()).empty()) {
            continue;
        }
        std::ifstream in(entry.path());
        std::stringstream held;
        held << in.rdbuf();
        std::string text = held.str();
        while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) {
            text.pop_back();
        }
        texts[entry.path().stem().string()] = std::move(text);
    }
    return texts;
}

}  // namespace

TEST(AStateAnotherBackendSavedIsRestored, AFormatThreeStateIsRestoredAndSavedAsItWas) {
    for (const auto &[name, text] : textsOf(kSaved)) {
        SCOPED_TRACE(name);
        EXPECT_EQ(machines().at(machineOf(name))(text), text);
    }
}

TEST(AStateAnotherBackendSavedIsRestored, EveryFormatThreeTextOfADocumentIsHeldToItsMachine) {
    EXPECT_GE(textsOf(kSaved).size(), 8u) << "the shared format-3 texts lost instances";
}

TEST(AStateAnotherBackendSavedIsRestored, AFormatOneStateIsRestoredAndTakesTheMachinesOwnSession) {
    for (const auto &[name, text] : textsOf(kSaved / "format1")) {
        SCOPED_TRACE(name);
        // What a format-1 text holds is what a restore reads; saving it again writes
        // format 3, with the machine's own session id.
        const std::string again = machines().at(machineOf(name))(text);
        EXPECT_EQ(again.rfind("{\"format\":3,", 0), 0u) << again;
    }
}

TEST(AStateAnotherBackendSavedIsRestored, EveryFormatOneTextOfADocumentIsHeldToItsMachine) {
    EXPECT_GE(textsOf(kSaved / "format1").size(), 10u) << "the shared format-1 texts lost instances";
}

// docs/adr/0090, section 7: a format-2 text, which every build before format 3 wrote, is
// restored at the moment the shared fixtures were saved at and saved at once, and the text
// is the format-3 text of the same name in `saved/format2/read_as/`, byte for byte: each
// origin read by the table of section 7, and written back as the pair and the sender.
TEST(AStateAnotherBackendSavedIsRestored, AFormatTwoStateIsRestoredAndSavedAsTheFormatThreeTextItIsReadAs) {
    const std::map<std::string, std::string> written = textsOf(kSaved / "format2");
    const std::map<std::string, std::string> readAs = textsOf(kSaved / "format2" / "read_as");
    for (const auto &[name, text] : written) {
        SCOPED_TRACE(name);
        ASSERT_EQ(readAs.count(name), 1u) << "no text in read_as/ for " << name;
        EXPECT_EQ(text.rfind("{\"format\":2,", 0), 0u) << text;
        EXPECT_EQ(machines().at(machineOf(name))(text), readAs.at(name));
    }
}

TEST(AStateAnotherBackendSavedIsRestored, EveryFormatTwoReadingTextIsHeldToItsMachine) {
    EXPECT_GE(textsOf(kSaved / "format2").size(), 4u) << "the format-2 reading texts lost instances";
}

// docs/adr/0090, section 7: a queued event's sender is the session that sent it and its
// origin the reply address the processor that carried it wrote, two fields no rule ties
// together. A Mesh remote child's event (MeshDispatch) names the child's session beside the
// endpoint it answers at; it is saved as it is queued, read back unchanged, and restored and
// saved again as the same text.
TEST(AStateAnotherBackendSavedIsRestored, AQueuedEventWhoseSenderStandsBesideAMeshEndpointIsSavedAsItIs) {
    std::ifstream in(kSaved / "static_timers.json");
    std::stringstream held;
    held << in.rdbuf();
    const std::string scxml = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";
    const std::string endpoint = "mesh://dev/machine/child";
    G::static_timers::static_timers machine;
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    machine.restoreAt(::SCE::Saved::SavedState::fromJson(held.str()), kSavedAtMs);
    machine.raiseExternal(::SCE::Common::ForwardedEvent{"beat", "", ::SCE::Core::EventOrigin(scxml, endpoint),
                                                        "fixture-root.work.1.child", "", "external", ""});
    const ::SCE::Saved::SavedState saved = machine.saveAt(kSavedAtMs);
    ASSERT_EQ(saved.external.size(), 1u);
    EXPECT_EQ(saved.external[0].origin,
              std::optional<::SCE::Core::EventOrigin>(::SCE::Core::EventOrigin(scxml, endpoint)));
    EXPECT_EQ(saved.external[0].sender, "fixture-root.work.1.child");

    const std::string text = saved.toJson();
    EXPECT_NE(
        text.find(R"("external":[{"name":"beat","data":"","type":"external","sendid":"","origin":{"processor":")" +
                  scxml + R"(","address":")" + endpoint +
                  R"("},"sender":"fixture-root.work.1.child","invokeid":"","hostinvoketoken":null}])"),
        std::string::npos)
        << text;
    const ::SCE::Saved::SavedState read = ::SCE::Saved::SavedState::fromJson(text);
    ASSERT_EQ(read.external.size(), 1u);
    EXPECT_EQ(read.external[0].origin, saved.external[0].origin);
    EXPECT_EQ(read.external[0].sender, saved.external[0].sender);
    EXPECT_EQ(machines().at("static_timers")(text), text);
}

}  // namespace Tests
}  // namespace SCE
