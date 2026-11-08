// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh WorkerSessionHost: the child session id a wire-14 `InvokeStart`
// makes.
//
// The worker assembles a remote child's session id from two values the wire
// carries, the parent machine (`source`) and the invoke id (`subject`), and the
// parent writes that id as the sender of every event the child sends it
// (docs/adr/0090). A session id is an XML Nmtoken (saved/SessionId.h), so a
// wire-14 whose two halves make none brings no session up: the parent is told
// as it is told of a child that failed to initialize (SCE_MESH.md §9.3),
// wire-20 carrying the refusal, then wire-18.
//
// The host is driven directly, with a publisher that records what it sends; no
// router is constructed and no transport is opened. The child machine is the
// `worker_session_f_wired` fixture, whose initial state is a `<final>`, so a
// child brought up completes at once.

#include "mesh/ChildSessionAdapter.h"
#include "mesh/MeshEnvelope.h"
#include "saved/SessionId.h"
#include "worker_session_f_wired_sm.h"
#include "worker_session_f_wired_transport.h"

#include <gtest/gtest.h>

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace {

using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::PatternKind;
using WorkerEngine = SCE::Generated::worker_session_f_wired::worker_session_f_wired;
using Host = SCE::Generated::worker_session_f_wired::TransportRouter<WorkerEngine>::WorkerSessionHost;

constexpr const char *kParentMachine = "parent_session_f_wired";

constexpr std::array<std::uint8_t, 16> kInvokeUuid{
    0x01, 0x8f, 0x2a, 0x10, 0x00, 0x00, 0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14,
};

MeshEnvelope makeInvokeStart(const std::string &invokeId) {
    MeshEnvelope env;
    env.pattern = PatternKind::InvokeStart;
    env.type = "scxml";
    env.source = kParentMachine;
    env.subject = invokeId;
    env.invoke_id = kInvokeUuid;
    return env;
}

/// What the host sends its parent when a wire-14 for @p invokeId arrives.
std::vector<MeshEnvelope> startChild(Host &host, const std::string &invokeId) {
    std::vector<MeshEnvelope> published;
    host.onWire14(makeInvokeStart(invokeId), [&published](const MeshEnvelope &env) {
        published.push_back(env);
        return true;
    });
    return published;
}

}  // namespace

// The control: an invoke id that makes a session id brings the child up, and
// the parent learns its id from wire-15 before anything else.
TEST(WorkerSessionHostTest, AnInvokeIdThatMakesASessionIdBringsTheChildUp) {
    Host host("ecu1", "worker_session_f_wired");

    const std::vector<MeshEnvelope> published = startChild(host, "remote_inv");

    ASSERT_FALSE(published.empty());
    EXPECT_EQ(published.front().pattern, PatternKind::InvokeStarted);
    EXPECT_EQ(published.front().child_session_id.value_or(""), "parent_session_f_wired:remote_inv");
    for (const MeshEnvelope &env : published) {
        EXPECT_NE(env.pattern, PatternKind::InvokeError) << "the child failed to come up";
    }
    // The child's initial state is final, so it completes as it comes up.
    EXPECT_EQ(published.back().pattern, PatternKind::InvokeDone);
}

TEST(WorkerSessionHostTest, AnInvokeIdThatMakesNoSessionIdBringsNoSessionUp) {
    Host host("ecu1", "worker_session_f_wired");
    const std::optional<std::string> refusal =
        SCE::Saved::SessionId::refusalOf(std::string(kParentMachine) + ":remote inv");
    ASSERT_TRUE(refusal.has_value()) << "the fixture's id is an Nmtoken, so it tests nothing";

    const std::vector<MeshEnvelope> published = startChild(host, "remote inv");

    ASSERT_EQ(published.size(), 2u) << "the parent is told of a failed bring-up as wire-20 then wire-18";
    EXPECT_EQ(published[0].pattern, PatternKind::InvokeError);
    EXPECT_EQ(published[0].subject.value_or(""), "remote inv");
    ASSERT_TRUE(published[0].invoke_id.has_value());
    EXPECT_TRUE(*published[0].invoke_id == kInvokeUuid);
    const std::string message = published[0].rpc_error_message.value_or("");
    EXPECT_NE(message.find(SCE::Mesh::kInvokeChildInitFailed), std::string::npos) << message;
    EXPECT_NE(message.find(*refusal), std::string::npos) << message;
    EXPECT_EQ(published[1].pattern, PatternKind::InvokeDone);
    EXPECT_EQ(published[1].subject.value_or(""), "remote inv");
    EXPECT_EQ(host.activeCount(), 0u);
}
