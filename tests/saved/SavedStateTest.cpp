// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The saved-state model of the C++ runtime: the texts every backend reads (docs/adr/0045,
// section 8), the refusals each member that breaks a format is held to, an event's origin as
// each format holds it (docs/adr/0090, section 7), the value types a state is made of, and the
// upgrade of a state saved before a record schema or an enum was revised (docs/adr/0044).
// Cross-language siblings: backends/go/runtime/saved_state_test.go and
// backends/python/tests/saved_state/test_saved_state.py.

#include "saved/SavedState.h"
#include "common/IOProcessorHelper.h"
#include "common/SCXMLConstants.h"
#include "core/EventOrigin.h"
#include "saved/SavedJson.h"
#include "saved/SavedValues.h"
#include "saved/SessionId.h"

#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <optional>
#include <regex>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace {

using namespace SCE::Saved;

std::string readTrimmed(const std::filesystem::path &path) {
    std::ifstream file(path);
    std::stringstream buffer;
    buffer << file.rdbuf();
    std::string text = buffer.str();
    while (!text.empty() && (text.back() == '\n' || text.back() == '\r' || text.back() == ' ')) {
        text.pop_back();
    }
    return text;
}

/// The shared saved-state texts every backend reads and writes back as they are: format 3 in
/// `saved/` and the unchanged format 1 in `saved/format1/`. The format-2 texts of
/// `saved/format2/` are written back as format 3 (formatTwoReadingTexts).
std::map<std::string, std::string> sharedSavedTexts() {
    const std::filesystem::path root = SCE_SAVED_FIXTURES_DIR;
    std::map<std::string, std::string> texts;
    for (const auto &directory : {root, root / "format1"}) {
        for (const auto &entry : std::filesystem::directory_iterator(directory)) {
            if (entry.is_regular_file() && entry.path().extension() == ".json") {
                texts[entry.path().string()] = readTrimmed(entry.path());
            }
        }
    }
    return texts;
}

/// Each format-2 reading fixture of `saved/format2/`, by its file's name, with the format-3
/// text of the same name in `saved/format2/read_as/` that it is read as (docs/adr/0090,
/// section 7).
std::map<std::string, std::pair<std::string, std::string>> formatTwoReadingTexts() {
    const std::filesystem::path directory = std::filesystem::path(SCE_SAVED_FIXTURES_DIR) / "format2";
    std::map<std::string, std::pair<std::string, std::string>> texts;
    for (const auto &entry : std::filesystem::directory_iterator(directory)) {
        if (entry.is_regular_file() && entry.path().extension() == ".json") {
            const std::string name = entry.path().filename().string();
            texts[name] = {readTrimmed(entry.path()), readTrimmed(directory / "read_as" / name)};
        }
    }
    return texts;
}

std::string replaceFirst(std::string text, const std::string &from, const std::string &to) {
    const auto at = text.find(from);
    if (at != std::string::npos) {
        text.replace(at, from.size(), to);
    }
    return text;
}

std::string refusalOf(const std::string &text) {
    try {
        static_cast<void>(SavedState::fromJson(text));
    } catch (const StateRefusal &refused) {
        return refused.what();
    }
    return "";
}

const std::string kMinimal =
    R"("shape":"s","sessionid":"root","configuration":["a"],"current":"a","variables":{},)"
    R"("history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0",)"
    R"("sendseq":"0","external":[])";

}  // namespace

TEST(SavedState, EverySharedSavedTextReadsAndWritesBackAsItWas) {
    const auto texts = sharedSavedTexts();
    ASSERT_GE(texts.size(), 20U) << "the shared saved states lost instances";
    for (const auto &[path, text] : texts) {
        EXPECT_EQ(SavedState::fromJson(text).toJson(), text) << path;
    }
}

TEST(SavedState, AFormatOneTextIsOfFormatOneAndHoldsNoSession) {
    const std::filesystem::path root = SCE_SAVED_FIXTURES_DIR;
    const SavedState saved = SavedState::fromJson(readTrimmed(root / "format1" / "static_invoke_working.json"));
    EXPECT_EQ(saved.format(), kSavedFormat1);
    EXPECT_FALSE(saved.invokes.ids.empty());
    EXPECT_TRUE(saved.sessionId.empty());
    EXPECT_FALSE(saved.boundSessionId().has_value());
}

TEST(SavedState, ATreeTextHoldsEveryChildWithItsSession) {
    const std::filesystem::path root = SCE_SAVED_FIXTURES_DIR;
    const SavedState saved = SavedState::fromJson(readTrimmed(root / "static_invoke_tree_working.json"));
    ASSERT_EQ(saved.format(), kSavedFormat3);
    ASSERT_FALSE(saved.invokes.children.empty());
    const SavedChild &child = saved.invokes.children[0];
    EXPECT_EQ(child.session->sessionId, saved.sessionId + "." + child.invokeId);
    EXPECT_EQ(saved.invokeStart(child.id), child.start);
}

TEST(SavedState, ATextTheFormatsDoNotCarryIsRefusedForTheMemberThatBreaksIt) {
    const std::vector<std::pair<std::string, std::string>> cases = {
        {R"({"format":)", "the saved state is not JSON (it ends where a value was expected)"},
        {"[]", "the saved state is not an object"},
        {"{}", "the saved state has no 'format'"},
        {R"({"format":4})", "the saved state is format 4, and this runtime reads formats 1, 2 and 3"},
        {R"({"format":"3"})", R"(the saved state is format "3", and this runtime reads formats 1, 2 and 3)"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("root")", R"("a b")") + "}",
         "'sessionid' (a b) is not an XML Nmtoken, which a session id is"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("invokestarts":{})", R"("invokestarts":{"w":"01"})") + "}",
         "'invokestarts.w' (01) is not a count of starts, a whole number from 1"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("hostinvoketoken":"0")", R"("hostinvoketoken":"x")") + "}",
         "'hostinvoketoken' (x) is not a whole number a token can be"},
        {R"({"format":2,)" + kMinimal + R"(,"revisions":{}})",
         "'revisions' names no schema; a state whose record schemas are all in their first revision leaves it out"},
        {R"({"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},)"
         R"("pending":[{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":""}],"invokes":[],)"
         R"("hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]})",
         "'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1"},
        {R"({"format":2,)" + replaceFirst(kMinimal, R"("pending":[])", R"("pending":[{"due":"5","act":"fax"}])") + "}",
         "'pending[0].act' is 'fax', which is not raise, internal, host, parent or child"},
    };
    for (const auto &[text, reason] : cases) {
        EXPECT_EQ(refusalOf(text), reason) << text;
    }
}

TEST(SavedState, AChildSessionIsRefusedByItsPath) {
    const auto withSession = [](const std::string &session) {
        return R"({"id":"w","start":"1","invokeid":"w","document":"w","session":{)" + session + "}}";
    };
    const auto holding = [](const std::string &child) {
        return replaceFirst(kMinimal, R"("invokes":[])", R"("invokes":[)" + child + "]");
    };
    const std::string bad = withSession(replaceFirst(kMinimal, R"("configuration":["a"])", R"("configuration":[1])"));
    EXPECT_EQ(refusalOf(R"({"format":2,)" + holding(bad) + "}"),
              "the session at invokes[0].session: 'configuration' is not a text");
    EXPECT_EQ(refusalOf(R"({"format":2,)" + holding(withSession(holding(bad))) + "}"),
              "the session at invokes[0].session.invokes[0].session: 'configuration' is not a text");
}

TEST(SavedState, AChildThatStatesAFormatIsRefused) {
    const std::string child =
        R"({"id":"w","start":"1","invokeid":"w","document":"w","session":{"format":2,)" + kMinimal + "}}";
    const std::string text =
        R"({"format":2,)" + replaceFirst(kMinimal, R"("invokes":[])", R"("invokes":[)" + child + "]") + "}";
    EXPECT_EQ(refusalOf(text), "'invokes[0].session' states a format, which only the root of a tree does");
}

namespace {

const std::string kScxmlType = SCE::Constants::SCXML_EVENT_PROCESSOR_TYPE;

/// A format-3 text of the minimal session whose external queue holds one event whose origin
/// members are `origin`, the members between `sendid` and `invokeid`.
std::string format3Queuing(const std::string &origin) {
    return R"({"format":3,)" +
           replaceFirst(kMinimal, R"("external":[])",
                        R"("external":[{"name":"e","data":"","type":"external","sendid":"",)" + origin +
                            R"(,"invokeid":"","hostinvoketoken":null}])") +
           "}";
}

/// A format-3 text of the minimal session waiting on one send of `act`, whose members after
/// `sendid` are `origin`.
std::string format3Waiting(const std::string &act, const std::string &origin) {
    const std::string head = act == "child" ? R"("act":"child","invokeid":"w",)" : R"("act":")" + act + R"(",)";
    return R"({"format":3,)" +
           replaceFirst(kMinimal, R"("pending":[])",
                        R"("pending":[{"due":"5",)" + head + R"("event":"e","data":"","sendid":"",)" + origin + "}]") +
           "}";
}

/// `{"processor":...,"address":...}` of the pair of `processor` and `address`.
std::string pairText(const std::string &processor, const std::string &address) {
    return R"({"processor":")" + processor + R"(","address":")" + address + R"("})";
}

}  // namespace

// docs/adr/0090, section 7: a format-3 reader takes `origin` and `sender` as they are written
// and refuses, naming the record, every text in which they are not a pair or null, a session
// id or null, and in agreement with each other and with the act that holds them.
TEST(SavedState, AFormatThreeOriginThatIsNotOneTheFormatHoldsIsRefusedNamingItsRecord) {
    const std::string root = pairText(kScxmlType, "sce://scxml/root");
    const std::vector<std::pair<std::string, std::string>> cases = {
        {format3Queuing(R"("sender":null)"), "'external[0]' has no 'origin'"},
        {format3Queuing(R"("origin":null)"), "'external[0]' has no 'sender'"},
        {format3Queuing(R"("origin":"root","sender":"root")"),
         "'external[0].origin' is neither null nor a pair of a 'processor' and an 'address'"},
        {format3Queuing(R"("origin":{"processor":"x-sce-host"},"sender":null)"),
         "'external[0].origin' has no 'address'"},
        {format3Queuing(R"("origin":{"processor":1,"address":"motor"},"sender":null)"),
         "'external[0].origin.processor' is not a text"},
        {format3Queuing(R"("origin":)" + pairText("", "motor") + R"(,"sender":null)"),
         "'external[0].origin.processor' is empty"},
        {format3Queuing(R"("origin":)" + pairText("x-sce-host", "") + R"(,"sender":null)"),
         "'external[0].origin.address' is empty"},
        {format3Queuing(R"("origin":{"processor":"x-sce-host","address":"motor","port":"1"},"sender":null)"),
         "'external[0].origin' holds 'port', and a pair holds a 'processor' and an 'address' alone"},
        {format3Queuing(R"("origin":null,"sender":1)"), "'external[0].sender' is neither null nor a text"},
        {format3Queuing(R"("origin":null,"sender":"a b")"),
         "'external[0].sender' (a b) is not an XML Nmtoken, which a session id is"},
        // A session that sent an event gave a reply address.
        {format3Queuing(R"("origin":null,"sender":"root")"),
         "'external[0]' has the sender 'root' and no origin, and a session that sends an event gives a reply "
         "address"},
        {format3Waiting("raise", R"("origin":)" + root + R"(,"sender":null)"),
         "'pending[0]' is a raise act with no sender, and it is a send of the session that saved it"},
        {format3Waiting("parent", R"("origin":null,"sender":null)"),
         "'pending[0]' is a parent act with no sender, and it is a send of the session that saved it"},
        {format3Waiting("child", R"("origin":null)"), "'pending[0]' has no 'sender'"},
        // A session's own act carries its own pair, its id percent-encoded byte for byte.
        {format3Waiting("child", R"("origin":)" + pairText("x-sce-host", "motor") + R"(,"sender":"root")"),
         "'pending[0]' has the sender 'root', and its origin is not the SCXML pair of that session, whose "
         "address is sce://scxml/root"},
        {format3Waiting("raise", R"("origin":)" + pairText(kScxmlType, "sce://scxml/peer:1") + R"(,"sender":"peer:1")"),
         "'pending[0]' has the sender 'peer:1', and its origin is not the SCXML pair of that session, whose "
         "address is sce://scxml/peer%3A1"},
        {format3Waiting("internal", R"("origin":null,"sender":"root")"),
         "'pending[0]' is an internal act and holds an origin or a sender, and the internal event it delivers "
         "has neither"},
        {format3Waiting("internal", R"("origin":)" + root + R"(,"sender":null)"),
         "'pending[0]' is an internal act and holds an origin or a sender, and the internal event it delivers "
         "has neither"},
        {format3Waiting("internal", R"("sender":null)"), "'pending[0]' has no 'origin'"},
    };
    for (const auto &[text, reason] : cases) {
        EXPECT_EQ(refusalOf(text), reason) << text;
    }
}

// docs/adr/0090, section 7: what format 3 holds is read as it is written, derived from nothing,
// and written back byte for byte: an SCXML pair whose address names no session has no sender,
// a host's pair keeps its own type, and a queued event's sender is independent of its address.
TEST(SavedState, AFormatThreeOriginIsReadAsItIsWrittenAndWrittenBackAsItWas) {
    const std::string mesh = format3Queuing(R"("origin":)" + pairText(kScxmlType, "mesh://ecu") + R"(,"sender":null)");
    const SavedState meshRead = SavedState::fromJson(mesh);
    ASSERT_EQ(meshRead.external.size(), 1U);
    EXPECT_EQ(meshRead.external[0].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin(kScxmlType, "mesh://ecu")));
    EXPECT_EQ(meshRead.external[0].sender, "");
    EXPECT_EQ(meshRead.toJson(), mesh);

    const std::string host = format3Queuing(R"("origin":)" + pairText("x-sce-host", "motor") + R"(,"sender":null)");
    const SavedState hostRead = SavedState::fromJson(host);
    EXPECT_EQ(hostRead.external[0].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin("x-sce-host", "motor")));
    EXPECT_EQ(hostRead.toJson(), host);

    const std::string sent =
        format3Queuing(R"("origin":)" + pairText(kScxmlType, "sce://scxml/peer%3A1") + R"(,"sender":"peer:1")");
    const SavedState sentRead = SavedState::fromJson(sent);
    EXPECT_EQ(sentRead.external[0].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("peer:1")));
    EXPECT_EQ(sentRead.external[0].sender, "peer:1");
    EXPECT_EQ(sentRead.toJson(), sent);

    // A Mesh remote child's event: the session that sent it beside the endpoint it answers at.
    const std::string remote = format3Queuing(R"("origin":)" + pairText(kScxmlType, "mesh://dev/machine/child") +
                                              R"(,"sender":"root.work.1.child")");
    const SavedState remoteRead = SavedState::fromJson(remote);
    EXPECT_EQ(remoteRead.external[0].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin(kScxmlType, "mesh://dev/machine/child")));
    EXPECT_EQ(remoteRead.external[0].sender, "root.work.1.child");
    EXPECT_EQ(remoteRead.toJson(), remote);

    const std::string waiting =
        format3Waiting("raise", R"("origin":)" + pairText(kScxmlType, "sce://scxml/root") + R"(,"sender":"root")");
    const SavedState waitingRead = SavedState::fromJson(waiting);
    ASSERT_EQ(waitingRead.pending.size(), 1U);
    EXPECT_EQ(waitingRead.pending[0].act.sender, "root");
    EXPECT_EQ(waitingRead.toJson(), waiting);
}

// docs/adr/0090, section 7: a format-3 raise, parent or child act carries this session's pair
// and this session's id, so an act that names another session, even with that session's own
// pair beside it, is refused, in the root and in every child session.
TEST(SavedState, AFormatThreeActThatNamesAnotherSessionIsRefused) {
    const std::string other = R"("origin":)" + pairText(kScxmlType, "sce://scxml/other") + R"(,"sender":"other")";
    EXPECT_EQ(refusalOf(format3Waiting("raise", other)),
              "'pending[0].sender' is 'other', and this session is 'root': a raise act is this session's own send");
    EXPECT_EQ(refusalOf(format3Waiting("child", other)),
              "'pending[0].sender' is 'other', and this session is 'root': a child act is this session's own send");

    // In a child session, the child's own id is the one its acts carry.
    const std::string parentAct = R"({"due":"5","act":"parent","event":"e","data":"","sendid":"",)" + other + "}";
    const std::string childSession = replaceFirst(replaceFirst(kMinimal, R"("root")", R"("root.w")"), R"("pending":[])",
                                                  R"("pending":[)" + parentAct + "]");
    const std::string child =
        R"({"id":"w","start":"1","invokeid":"w","document":"w","session":{)" + childSession + "}}";
    EXPECT_EQ(
        refusalOf(R"({"format":3,)" + replaceFirst(kMinimal, R"("invokes":[])", R"("invokes":[)" + child + "]") + "}"),
        "the session at invokes[0].session: 'pending[0].sender' is 'other', and this session is 'root.w': a "
        "parent act is this session's own send");

    // The session's own id is read.
    const std::string own = R"("origin":)" + pairText(kScxmlType, "sce://scxml/root") + R"(,"sender":"root")";
    EXPECT_EQ(refusalOf(format3Waiting("raise", own)), "");
}

// docs/adr/0090, section 7: a tree is written as format 3 whatever its acts were given. An
// internal act delivers an internal event, which has no origin, so it is written with neither
// member even where the entry it was saved from kept a sender.
TEST(SavedState, AnInternalActIsWrittenWithNoOriginAndNoSender) {
    SavedState state;
    state.shape = "s";
    state.configuration = {"a"};
    state.current = "a";
    state.sessionId = "root";
    state.invokes.resumed = true;
    SavedSend send;
    send.due = 5;
    send.act.kind = SavedActKind::Internal;
    send.act.event = "e";
    send.act.sendId = "s";
    send.act.origin = SCE::IOProcessorHelper::scxmlOrigin("root");
    send.act.sender = "root";
    state.pending.push_back(send);
    const std::string text = state.toJson();
    EXPECT_EQ(text.rfind(R"({"format":3,)", 0), 0U) << text;
    EXPECT_NE(
        text.find(R"({"due":"5","act":"internal","event":"e","data":"","sendid":"s","origin":null,"sender":null})"),
        std::string::npos)
        << text;
    EXPECT_EQ(SavedState::fromJson(text).toJson(), text);
}

// docs/adr/0090, section 7: each format-2 reading fixture is read by the table of section 7 and
// written back as the format-3 text of `saved/format2/read_as/`, byte for byte, by the model
// alone. The machine that restores each is held to the same text in
// AStateAnotherBackendSavedIsRestoredTest.cpp.
TEST(SavedState, EveryFormatTwoTextIsWrittenBackAsTheFormatThreeTextItIsReadAs) {
    const auto texts = formatTwoReadingTexts();
    ASSERT_GE(texts.size(), 4U) << "the format-2 reading fixtures lost instances";
    for (const auto &[name, read] : texts) {
        const auto &[format2, format3] = read;
        ASSERT_FALSE(format3.empty()) << name << " has no text in read_as/";
        EXPECT_EQ(format2.rfind(R"({"format":2,)", 0), 0U) << name;
        EXPECT_EQ(SavedState::fromJson(format2).toJson(), format3) << name;
    }
}

// docs/adr/0090, section 7: the rows a reader can get wrong, asked of the model directly
// (`saved/format2/README.md`). An unencoded SCXML location is read by the id it names; an
// address with no scheme beside a type that is not the SCXML type is the host's own pair; an
// internal act's stored sender is not read.
TEST(SavedState, AFormatTwoOriginIsReadAsThePairTheTableGives) {
    const std::filesystem::path format2 = std::filesystem::path(SCE_SAVED_FIXTURES_DIR) / "format2";
    const SavedState origins = SavedState::fromJson(readTrimmed(format2 / "static_invoke_tree_working_origins.json"));
    const auto named = [&](const std::string &name) -> const SavedEvent & {
        for (const SavedEvent &event : origins.external) {
            if (event.name == name) {
                return event;
            }
        }
        throw std::runtime_error("no queued event '" + name + "'");
    };
    EXPECT_EQ(named("stamped").origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("peer:1")));
    EXPECT_EQ(named("stamped").sender, "peer:1");
    EXPECT_EQ(named("rest").origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin("x-sce-host", "motor")));
    EXPECT_EQ(named("rest").sender, "");
    EXPECT_FALSE(named("swap").origin.has_value());
    EXPECT_EQ(named("arm").origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin(kScxmlType, "mesh://ecu")));
    EXPECT_EQ(named("arm").sender, "");

    const SavedState timers = SavedState::fromJson(readTrimmed(format2 / "static_timers.json"));
    ASSERT_FALSE(timers.pending.empty());
    for (const SavedSend &send : timers.pending) {
        if (send.act.kind == SavedActKind::Internal) {
            EXPECT_FALSE(send.act.origin.has_value()) << send.act.event;
            EXPECT_EQ(send.act.sender, "") << send.act.event;
        } else {
            EXPECT_EQ(send.act.origin,
                      std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("fixture-root")))
                << send.act.event;
            EXPECT_EQ(send.act.sender, "fixture-root") << send.act.event;
        }
    }
}

// docs/adr/0090, section 7: a raise, parent or child act of a format-2 text whose stored origin
// is empty is read as sent by the session that holds it, the `sessionid` of its own session
// record, and is written back with that session's pair; an internal act stays without one.
TEST(SavedState, AFormatTwoActWithAnEmptyOriginIsTheSendOfTheSessionThatHoldsIt) {
    const std::string raise = R"({"due":"5","act":"raise","event":"e","data":"","sendid":"a","origin":""})";
    const std::string internal = R"({"due":"6","act":"internal","event":"i","data":"","sendid":"b","origin":""})";
    const std::string parent = R"({"due":"7","act":"parent","event":"p","data":"","sendid":"c","origin":""})";
    const std::string childSession = replaceFirst(replaceFirst(kMinimal, R"("root")", R"("root.w")"), R"("pending":[])",
                                                  R"("pending":[)" + parent + "]");
    const std::string child =
        R"({"id":"w","start":"1","invokeid":"w","document":"w","session":{)" + childSession + "}}";
    std::string rootSession =
        replaceFirst(kMinimal, R"("pending":[])", R"("pending":[)" + raise + "," + internal + "]");
    rootSession = replaceFirst(rootSession, R"("invokes":[])", R"("invokes":[)" + child + "]");
    const SavedState read = SavedState::fromJson(R"({"format":2,)" + rootSession + "}");

    ASSERT_EQ(read.pending.size(), 2U);
    EXPECT_EQ(read.pending[0].act.origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("root")));
    EXPECT_EQ(read.pending[0].act.sender, "root");
    EXPECT_FALSE(read.pending[1].act.origin.has_value());
    EXPECT_EQ(read.pending[1].act.sender, "");
    ASSERT_EQ(read.invokes.children.size(), 1U);
    const SavedState &held = *read.invokes.children[0].session;
    ASSERT_EQ(held.pending.size(), 1U);
    EXPECT_EQ(held.pending[0].act.origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("root.w")));
    EXPECT_EQ(held.pending[0].act.sender, "root.w");

    // Written back as format 3, which reads again as it was written.
    const std::string text = read.toJson();
    EXPECT_NE(text.find(R"("sendid":"a","origin":{"processor":")" + kScxmlType +
                        R"(","address":"sce://scxml/root"},"sender":"root"})"),
              std::string::npos)
        << text;
    EXPECT_NE(text.find(R"("sendid":"b","origin":null,"sender":null})"), std::string::npos) << text;
    EXPECT_NE(text.find(R"("sendid":"c","origin":{"processor":")" + kScxmlType +
                        R"(","address":"sce://scxml/root.w"},"sender":"root.w"})"),
              std::string::npos)
        << text;
    EXPECT_EQ(SavedState::fromJson(text).toJson(), text);
}

// docs/adr/0090, section 7: a format-1 or format-2 act whose stored origin is not empty must
// name the act's session, by an id or by an SCXML location with an id: the sender it is read
// as is an XML Nmtoken, or the text is refused. The SCXML location prefix alone, an address
// of another processor and a location whose id decodes to no Nmtoken are each refused; an id
// written as a location is read as that id. A queued event stored with the prefix alone is
// read as the pair as saved, with no sender: the prefix names no session to be its sender.
TEST(SavedState, AStoredActOriginThatNamesNoSessionIsRefused) {
    const auto acting = [](const std::string &act, const std::string &origin) {
        const std::string head = act == "child" ? R"("act":"child","invokeid":"w",)" : R"("act":")" + act + R"(",)";
        return R"({"format":2,)" +
               replaceFirst(kMinimal, R"("pending":[])",
                            R"("pending":[{"due":"5",)" + head + R"("event":"e","data":"","sendid":"","origin":")" +
                                origin + R"("}])") +
               "}";
    };
    const auto namesNoSession = [](const std::string &origin, const std::string &act) {
        return "'pending[0].origin' is '" + origin + "', which names no session: a " + act +
               " act is a session's own send, and its stored origin is that session's id or its SCXML location";
    };
    for (const std::string origin : {"sce://scxml/", "mesh://elsewhere", "sce://scxml/a%20b"}) {
        for (const std::string act : {"raise", "parent", "child"}) {
            EXPECT_EQ(refusalOf(acting(act, origin)), namesNoSession(origin, act)) << act << " " << origin;
        }
        // Format 1 stores a raise act's origin the same way, and it names no session there
        // either.
        EXPECT_EQ(
            refusalOf(R"({"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},)"
                      R"("pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":"","origin":")" +
                      origin +
                      R"("}],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]})"),
            namesNoSession(origin, "raise"))
            << origin;
        // An internal act's stored origin is not read.
        EXPECT_EQ(refusalOf(acting("internal", origin)), "") << origin;
    }

    // A location whose id decodes to an Nmtoken names that session, and is read as its sender.
    const SavedState located = SavedState::fromJson(acting("raise", "sce://scxml/peer%3A1"));
    ASSERT_EQ(located.pending.size(), 1U);
    EXPECT_EQ(located.pending[0].act.sender, "peer:1");
    EXPECT_EQ(located.pending[0].act.origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("peer:1")));

    const SavedState queued = SavedState::fromJson(
        R"({"format":2,)" +
        replaceFirst(kMinimal, R"("external":[])",
                     R"("external":[{"name":"go","data":"","type":"external","sendid":"","origin":"sce://scxml/",)"
                     R"("origintype":"","invokeid":"","hostinvoketoken":null}])") +
        "}");
    ASSERT_EQ(queued.external.size(), 1U);
    EXPECT_EQ(queued.external[0].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin(kScxmlType, "sce://scxml/")));
    EXPECT_EQ(queued.external[0].sender, "");
}

// docs/adr/0090, section 7: a format-1 text holds `origin` and `origintype` with format 2's
// meaning, and written back without a machine it stays format 1, its origins written by the
// rule format 2 used: the sender's id for a session's send, the address otherwise, then the
// pair's processor.
TEST(SavedState, AFormatOneTextIsReadThroughTheTableAndWrittenBackAsFormatOne) {
    const std::string text =
        R"({"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},)"
        R"("pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":"t","origin":"root"}],"invokes":[],)"
        R"("hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[)"
        R"({"name":"a","data":"","type":"external","sendid":"","origin":"kid","origintype":")" +
        kScxmlType +
        R"(","invokeid":"","hostinvoketoken":null},)"
        R"({"name":"b","data":"","type":"external","sendid":"","origin":"motor","origintype":"x-sce-host",)"
        R"("invokeid":"","hostinvoketoken":null}]})";
    const SavedState read = SavedState::fromJson(text);
    ASSERT_EQ(read.format(), kSavedFormat1);
    ASSERT_EQ(read.external.size(), 2U);
    EXPECT_EQ(read.external[0].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("kid")));
    EXPECT_EQ(read.external[0].sender, "kid");
    EXPECT_EQ(read.external[1].origin,
              std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin("x-sce-host", "motor")));
    EXPECT_EQ(read.external[1].sender, "");
    ASSERT_EQ(read.pending.size(), 1U);
    EXPECT_EQ(read.pending[0].act.origin,
              std::optional<SCE::Core::EventOrigin>(SCE::IOProcessorHelper::scxmlOrigin("root")));
    EXPECT_EQ(read.pending[0].act.sender, "root");
    EXPECT_EQ(read.toJson(), text);
}

TEST(SavedState, AJsonNumberKeepsTheSpellingItWasWrittenIn) {
    const std::string text = R"({"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"\u00e9\ud83d\ude00"})";
    // A character written as a \u escape is written as the character.
    EXPECT_EQ(writeJson(parseJson(text)),
              "{\"a\":[1,-0,1.50,1E+2,9223372036854775808],\"b\":null,\"c\":\"\xC3\xA9\xF0\x9F\x98\x80\"}");
}

TEST(SavedState, AJsonObjectKeepsTheOrderItsMembersWereWrittenIn) {
    const JsonValue value = parseJson(R"({"z":1,"a":2,"m":3})");
    std::string order;
    for (const std::string &key : value.asObject().keys()) {
        order += key;
    }
    EXPECT_EQ(order, "zam");
}

TEST(SavedState, ATextThatIsNotOneJsonValueIsRefused) {
    const std::vector<std::pair<std::string, std::string>> cases = {
        {R"({"a":1,"a":2})", R"(not JSON (the field "a" appears twice))"},
        {"1 2", "more than one JSON value"},
        {"[1,]", "not JSON (unexpected ']')"},
        {R"("\q")", R"(not JSON (unknown escape '\q'))"},
        {R"("\u12")", R"(not JSON (a \u escape is short))"},
        {R"("\u12zz")", R"(not JSON (a \u escape is not hex))"},
        {"-", "not JSON (a number has no digits)"},
        {R"({"a" 1})", "not JSON (a ':' was expected)"},
        {"tru", "not JSON (expected 'true')"},
    };
    for (const auto &[text, want] : cases) {
        try {
            parseJson(text);
            FAIL() << text << " was read";
        } catch (const JsonError &err) {
            EXPECT_EQ(std::string(err.what()), want) << text;
        }
    }
}

TEST(SavedState, ALoneSurrogateReadsAsTheReplacementCharacter) {
    EXPECT_EQ(parseJson(R"("\ud800x\udc00")").asString(), "\xEF\xBF\xBD"
                                                          "x"
                                                          "\xEF\xBF\xBD");
}

TEST(SavedState, EveryValueTypeIsWrittenAsItsSavedFormAndReadBackAsItWas) {
    EXPECT_EQ(writeJson(savedWriteUint8(200)), "200");
    EXPECT_EQ(writeJson(savedWriteInt64(INT64_MIN)), "\"-9223372036854775808\"");
    EXPECT_EQ(writeJson(savedWriteUint64(UINT64_MAX)), "\"18446744073709551615\"");
    EXPECT_EQ(writeJson(savedWriteFloat32(0.3F)), "0.30000001192092896");
    EXPECT_EQ(writeJson(savedWriteFloat64(5.0)), "5");
    EXPECT_EQ(writeJson(savedWriteFloat64(1e21)), "1e+21");
    EXPECT_EQ(writeJson(savedWriteBytes({0, 0xE9, 0xFF})), "\"\\u0000\xC3\xA9\xC3\xBF\"");
    EXPECT_EQ(savedReadUint8(parseJson("255"), "x"), 255);
    EXPECT_EQ(savedReadInt8(parseJson("-128"), "x"), -128);
    EXPECT_EQ(savedReadUint32(parseJson("4294967295"), "x"), 4294967295U);
    EXPECT_EQ(savedReadInt64(parseJson("\"-9223372036854775808\""), "x"), INT64_MIN);
    EXPECT_EQ(savedReadUint64(parseJson("\"18446744073709551615\""), "x"), UINT64_MAX);
    EXPECT_EQ(savedReadFloat64(parseJson("1.5"), "x"), 1.5);
    EXPECT_TRUE(std::isinf(savedReadFloat32(parseJson("\"-Infinity\""), "x")));
    EXPECT_TRUE(std::isinf(savedReadFloat64(parseJson("1e999"), "x")));
    EXPECT_TRUE(savedReadBool(parseJson("true"), "x"));
    // A 32-bit real is rounded to the single it is.
    EXPECT_EQ(savedReadFloat32(parseJson("0.3"), "x"), 0.3F);
}

TEST(SavedState, AValueOfAnotherKindOrAnotherWidthIsRefusedNotConverted) {
    const auto refused = [](const std::string &text, const auto &read) {
        try {
            read(parseJson(text));
        } catch (const StateRefusal &err) {
            return std::string(err.what());
        }
        return std::string();
    };
    const auto uint8Of = [](const JsonValue &v) { return savedReadUint8(v, "x"); };
    const auto int64Of = [](const JsonValue &v) { return savedReadInt64(v, "x"); };
    const auto stringOf = [](const JsonValue &v) { return savedReadString(v, "x"); };
    const auto boolOf = [](const JsonValue &v) { return savedReadBool(v, "x"); };
    EXPECT_EQ(refused("256", uint8Of), "'x' (256) is not a whole number its type can hold");
    EXPECT_EQ(refused("-1", uint8Of), "'x' (-1) is not a whole number its type can hold");
    EXPECT_EQ(refused("1.5", uint8Of), "'x' (1.5) is not a whole number its type can hold");
    EXPECT_EQ(refused("\"1\"", uint8Of), "'x' is not a number");
    EXPECT_EQ(refused("null", uint8Of), "'x' is absent (null), and its type is not optional");
    EXPECT_EQ(refused("5", int64Of), "'x' is not a 64-bit whole number written as a text");
    EXPECT_EQ(refused("\"9223372036854775808\"", int64Of),
              "'x' (9223372036854775808) is not a whole number its type can hold");
    EXPECT_EQ(refused("1", stringOf), "'x' is not a text");
    EXPECT_EQ(refused("\"true\"", boolOf), "'x' is not a truth value");
}

TEST(SavedState, ATextAndAListAreHeldToTheBoundsTheMachineHas) {
    const auto message = [](const auto &read) {
        try {
            read();
        } catch (const StateRefusal &err) {
            return std::string(err.what());
        }
        return std::string();
    };
    EXPECT_EQ(message([] { savedReadBoundedString(JsonValue("h\xC3\xA9llo"), "t", 5); }),
              "'t' holds 6 UTF-8 bytes, past the 5 it is bounded by");
    EXPECT_EQ(message([] { savedReadBytes(JsonValue("a\xC4\x80"), "b", 8); }),
              "'b' holds U+0100, which is past U+00FF and so no byte");
    EXPECT_EQ(message([] { savedReadBytes(JsonValue("abc"), "b", 2); }),
              "'b' holds 3 bytes, past the 2 it is bounded by");
    const JsonValue items = parseJson("[1,2,300]");
    EXPECT_EQ(message([&] {
                  savedReadList(items, "l", 8,
                                [](const JsonValue &v, const std::string &w) { return savedReadUint8(v, w); });
              }),
              "'l[2]' (300) is not a whole number its type can hold");
    EXPECT_EQ(message([&] {
                  savedReadList(items, "l", 2,
                                [](const JsonValue &v, const std::string &w) { return savedReadUint8(v, w); });
              }),
              "'l' holds 3 elements, past the 2 it is bounded by");
}

TEST(SavedState, AnUpgradeGivesARecordTheDefaultsOfTheFieldsALaterRevisionAdded) {
    const std::string digest(64, 'a');
    SavedShape shape;
    shape.first = "first";
    shape.revised = "revised";
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    RecordLineage record;
    record.event = "shown";
    record.revision = 2;
    record.revisions = {digest};
    record.added = {AddedField{"weekday", 2, "0"}};
    record.holders = {RecordHolder{"one", false}, RecordHolder{"many", true}};
    shape.records = {record};

    const std::string given = R"({"one":{"year":2027},"many":[{"year":1},{"year":2}]})";
    SavedState saved;
    saved.shape = "first2";
    saved.variables = parseJson(given).asObject();
    saved.invokes.resumed = true;
    const SavedState upgraded = upgradeSavedState(saved, shape);
    EXPECT_EQ(writeJson(JsonValue(upgraded.variables)),
              R"({"one":{"year":2027,"weekday":0},"many":[{"year":1,"weekday":0},{"year":2,"weekday":0}]})");
    EXPECT_EQ(writeJson(JsonValue(saved.variables)), given) << "the upgrade changed the state it was given";

    // A record that already holds a field its revision does not declare.
    saved.variables = parseJson(R"({"one":{"year":1,"weekday":3},"many":[]})").asObject();
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(std::string(err.what()),
                  "'one' holds 'weekday', which revision 1 of 'shown' does not declare: revision 2 added it");
    }
    // A state of another shape.
    saved.shape = "other";
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(std::string(err.what()),
                  "the saved state is of a document of shape other, and this machine's is first2");
    }
    // A revision later than the machine's.
    saved.shape = "revised2";
    saved.revisions = {SavedRevision{"shown", 3, digest}};
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(std::string(err.what()),
                  "the saved state was saved at revision 3 of 'shown', later than this machine's 2: a machine does "
                  "not restore what it does not know");
    }
}

// docs/adr/0055, section 6: the third revision of `shown` widened `year`, a field of the first,
// and `weekday`, which the second added, to optional. A record saved before the widening holds
// the field as a value, so a null in it is refused, naming the field and the revision; saved at
// the widening, null is absent.
TEST(SavedState, ANullSavedBeforeAFieldWasWidenedIsRefused) {
    const std::string second(64, '2');
    const std::string third(64, '3');
    SavedShape shape;
    shape.first = "first";
    shape.revised = "revised";
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    RecordLineage record;
    record.event = "shown";
    record.revision = 3;
    record.revisions = {second, third};
    record.added = {AddedField{"weekday", 2, "0"}};
    record.widened = {WidenedField{"year", 1, 3}, WidenedField{"weekday", 2, 3}};
    record.holders = {RecordHolder{"one", false}, RecordHolder{"many", true}};
    shape.records = {record};

    const auto savedAt = [&](int revision, const std::string &one, const std::string &many) {
        SavedState saved;
        saved.shape = revision == 1 ? "first2" : "revised2";
        saved.variables = parseJson(R"({"one":)" + one + R"(,"many":)" + many + "}").asObject();
        saved.invokes.resumed = true;
        if (revision > 1) {
            saved.revisions = {SavedRevision{"shown", revision, revision == 2 ? second : third}};
        }
        return saved;
    };
    const auto refusal = [&](const SavedState &saved) {
        try {
            static_cast<void>(upgradeSavedState(saved, shape));
        } catch (const StateRefusal &err) {
            return std::string(err.what());
        }
        return std::string();
    };

    // A record variable, a field of the first revision, saved at the first.
    EXPECT_EQ(refusal(savedAt(1, R"({"year":null})", "[]")),
              "'one.year' is null, which revision 1 of 'shown' holds as a value: revision 3 widened it");
    // An element of a list of records.
    EXPECT_NE(refusal(savedAt(1, R"({"year":4})", R"([{"year":1},{"year":null}])")).find("'many[1].year' is null"),
              std::string::npos);
    // A field the second revision added, saved at the second.
    EXPECT_EQ(refusal(savedAt(2, R"({"year":4,"weekday":null})", "[]")),
              "'one.weekday' is null, which revision 2 of 'shown' holds as a value: revision 3 widened it");
    // The field the second added, held by a record saved at the first, is refused as a field that
    // revision does not declare, which is the truer reason.
    EXPECT_NE(refusal(savedAt(1, R"({"year":4,"weekday":null})", "[]")).find("'one' holds 'weekday'"),
              std::string::npos);

    // Saved at the revision that widened both, null is absent.
    const SavedState kept = savedAt(3, R"({"year":null,"weekday":null})", R"([{"year":null,"weekday":2}])");
    EXPECT_EQ(writeJson(JsonValue(upgradeSavedState(kept, shape).variables)), writeJson(JsonValue(kept.variables)));
    // A record saved before the widening, each field present, restores as it was.
    EXPECT_EQ(writeJson(JsonValue(upgradeSavedState(savedAt(1, R"({"year":4})", R"([{"year":1}])"), shape).variables)),
              R"({"one":{"year":4,"weekday":0},"many":[{"year":1,"weekday":0}]})");
    // What is no record where one is held is left to the variable's own read.
    EXPECT_EQ(writeJson(JsonValue(upgradeSavedState(savedAt(1, "null", "[{}]"), shape).variables)),
              R"({"one":null,"many":[{"weekday":0}]})");
}

TEST(SavedState, AnEnumVariantALaterRevisionAddedIsRefusedInAStateSavedBeforeIt) {
    SavedShape shape;
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    EnumLineage lineage;
    lineage.key = "enum:status";
    lineage.revision = 2;
    lineage.revisions = {std::string(64, 'b')};
    lineage.added = {AddedVariant{"Done", 2}};
    lineage.holders = {EnumHolder{"item", std::string("status"), false}, EnumHolder{"state", std::nullopt, false}};
    shape.enums = {lineage};
    SavedState saved;
    saved.shape = "first2";
    saved.variables = parseJson(R"({"item":{"status":"Done"},"state":"Open"})").asObject();
    saved.invokes.resumed = true;
    try {
        upgradeSavedState(saved, shape);
        FAIL() << "upgraded";
    } catch (const StateRefusal &err) {
        EXPECT_EQ(
            std::string(err.what()),
            "'item.status' holds 'Done', which revision 1 of 'enum:status' does not declare: revision 2 added it");
    }
}

TEST(SavedState, ASessionIdIsAnXmlNmtokenAndADrawnOneIsAVersion4Uuid) {
    for (const std::string id : {"fixture-root", "a.b:c_d",
                                 "\xC3\xA9"
                                 "1",
                                 "x\xC2\xB7y"}) {
        EXPECT_FALSE(SessionId::refusalOf(id).has_value()) << id;
    }
    EXPECT_EQ(SessionId::refusalOf("").value(),
              "the session id is empty, and a session id is one name character or more (an XML Nmtoken)");
    EXPECT_EQ(SessionId::refusalOf("a b").value(),
              "the session id holds U+0020, which is not a name character: a session id is an XML Nmtoken "
              "(letters, digits, '.', '-', '_', ':')");
    EXPECT_EQ(SessionId::refusalOf("\xFF").value(),
              "the session id holds U+FFFD, which is not a name character: a session id is an XML Nmtoken "
              "(letters, digits, '.', '-', '_', ':')");
    const std::regex pattern("^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$");
    const std::string first = SessionId::draw();
    const std::string second = SessionId::draw();
    EXPECT_TRUE(std::regex_match(first, pattern)) << first;
    EXPECT_TRUE(SessionId::isNmtoken(first));
    EXPECT_NE(first, second);
}

TEST(SavedState, ARefusalInsideAChildNamesTheSessionItIsAbout) {
    const StateRefusal inner = refusalWithin(StateRefusal("'x' is not a text"), "invokes[1].session");
    EXPECT_EQ(std::string(inner.what()), "the session at invokes[1].session: 'x' is not a text");
    const StateRefusal outer = refusalWithin(inner, "invokes[0].session");
    EXPECT_EQ(std::string(outer.what()), "the session at invokes[0].session.invokes[1].session: 'x' is not a text");
}

namespace {

/// A configuration of a product line, or none: nullopt is a machine of no product line, and an
/// empty list the configuration that turns no feature on.
using Features = std::optional<std::vector<std::string>>;

/// The configuration that turns on `names`, which may be none.
Features on(std::vector<std::string> names) {
    return names;
}

/// A format-2 session of shape `a` taken in the configuration `features`.
SavedState takenWith(Features features) {
    SavedState saved;
    saved.shape = "a";
    saved.configuration = {"a"};
    saved.current = "a";
    saved.sessionId = "root";
    saved.invokes.resumed = true;
    saved.features = std::move(features);
    return saved;
}

/// The refusal `judge` throws, or empty when it throws none.
template <typename Judge> std::string refusalBy(const Judge &judge) {
    try {
        judge();
    } catch (const StateRefusal &refused) {
        return refused.what();
    }
    return "";
}

}  // namespace

// docs/adr/0057, section 7: a saved state records the configuration it was taken in, as the last
// member of the session, after `revisions`; a machine of no product line records none, and its
// text is the one it was before configurations were recorded.
TEST(SavedState, AConfigurationIsReadBackAsItWasWritten) {
    SavedState state = takenWith(on({"session-unicast", "tls"}));
    state.revisions = {SavedRevision{"reading.taken", 3, std::string(64, '0')}};
    const std::string text = state.toJson();
    EXPECT_NE(text.find(R"("revisions":{"reading.taken":{"revision":3,"fields":")" + std::string(64, '0') +
                        R"("}},"features":["session-unicast","tls"]})"),
              std::string::npos)
        << text;
    const SavedState back = SavedState::fromJson(text);
    EXPECT_EQ(back.features, on({"session-unicast", "tls"}));
    EXPECT_EQ(back.toJson(), text);

    // The configuration that turns none on is written, and read back, as an empty list.
    const SavedState none = takenWith(on({}));
    EXPECT_NE(none.toJson().find(R"("external":[],"features":[]})"), std::string::npos) << none.toJson();
    EXPECT_EQ(SavedState::fromJson(none.toJson()).features, on({}));

    // A machine of no product line records no configuration.
    const SavedState plain = takenWith(std::nullopt);
    EXPECT_EQ(plain.toJson().find("features"), std::string::npos) << plain.toJson();
    EXPECT_FALSE(SavedState::fromJson(plain.toJson()).features.has_value());

    // Format 1 writes it last as well.
    SavedState format1 = takenWith(on({"tls"}));
    format1.invokes.resumed = false;
    format1.sessionId.clear();
    ASSERT_EQ(format1.format(), kSavedFormat1);
    EXPECT_NE(format1.toJson().find(R"("external":[],"features":["tls"]})"), std::string::npos) << format1.toJson();
    EXPECT_EQ(SavedState::fromJson(format1.toJson()).toJson(), format1.toJson());
}

// docs/adr/0057, section 7: a restore judges the configuration a state was taken in before its
// shape, and names the features that differ.
TEST(SavedState, AStateOfAnotherConfigurationIsRefusedByTheFeaturesThatDiffer) {
    const auto refused = [](const SavedState &saved, const Features &built) {
        return refusalBy([&] { checkSavedFeatures(saved, built); });
    };
    EXPECT_EQ(refused(takenWith(std::nullopt), std::nullopt), "");
    EXPECT_EQ(refused(takenWith(on({})), on({})), "");
    EXPECT_EQ(refused(takenWith(on({"a", "b"})), on({"a", "b"})), "");
    EXPECT_EQ(refused(takenWith(on({"a", "b"})), on({"a", "c"})),
              "the saved state was taken with the features [a, b] on, and this machine is built with [a, c]: [b] on "
              "only where it was saved, [c] on only in this build");
    EXPECT_EQ(refused(takenWith(on({"a"})), on({})),
              "the saved state was taken with the features [a] on, and this machine is built with []: [a] on only "
              "where it was saved");
    EXPECT_EQ(refused(takenWith(on({})), on({"a"})),
              "the saved state was taken with the features [] on, and this machine is built with [a]: [a] on only "
              "in this build");
    EXPECT_EQ(refused(takenWith(std::nullopt), on({"a"})),
              "the saved state records no features, and this machine is built with the features [a] on");
    EXPECT_EQ(refused(takenWith(on({})), std::nullopt),
              "the saved state was taken with the features [] on, and this machine is built from a document of no "
              "feature model");

    // Judged before the shape: a state of another build is refused for its features even where
    // its shape differs too.
    SavedShape shape;
    shape.first = "first";
    shape.revised = "revised";
    shape.format2First = "first2";
    shape.format2Revised = "revised2";
    shape.features = on({"a"});
    const std::string reason = refusalBy([&] { static_cast<void>(upgradeSavedState(takenWith(on({"b"})), shape)); });
    EXPECT_EQ(reason.rfind("the saved state was taken with the features [b] on", 0), 0U) << reason;
    // A state of this build's configuration is then judged by its shape.
    EXPECT_EQ(refusalBy([&] { static_cast<void>(upgradeSavedState(takenWith(on({"a"})), shape)); }),
              "the saved state is of a document of shape a, and this machine's is first2");
}

// docs/adr/0057, section 7: `features` is a set, written sorted, so a text that repeats a feature
// or names them out of order is refused as it is read.
TEST(SavedState, TheFeaturesOfASavedStateAreASortedSet) {
    const auto text = [](const std::string &features) {
        return R"({"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},"history":{},)"
               R"("pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[],)"
               R"("features":)" +
               features + "}";
    };
    EXPECT_EQ(SavedState::fromJson(text(R"(["a","b"])")).features, on({"a", "b"}));
    EXPECT_EQ(SavedState::fromJson(text("[]")).features, on({}));
    for (const std::string unordered : {R"(["b","a"])", R"(["a","a"])"}) {
        EXPECT_EQ(refusalOf(text(unordered)),
                  "'features' is not in order or names a feature twice; it is a set, sorted")
            << unordered;
    }
    EXPECT_EQ(refusalOf(text(R"("a")")), "'features' is not an array");
    EXPECT_EQ(refusalOf(text("[1]")), "'features' is not a text");
}
