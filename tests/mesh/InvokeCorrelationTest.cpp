// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE-VERIFIES: mesh-9.5
//
// InvokeCorrelation unit tests — SCE Mesh §9.5 lifecycle bookkeeping.
//
// Covers the four outcome states a registered invoke can reach:
//   1. Reply with RpcStatus::Ok   → callback fired with Ok + payload
//   2. Reply with non-Ok status   → callback fired with the status
//   3. Deadline expiry            → callback fired with DeadlineExceeded
//   4. Author <cancel>            → entry erased, callback NOT fired
//
// Plus the three race conditions §9.5 defines as "benign drop":
//   * late reply after cancel     → second handleReply returns false
//   * late reply after deadline   → second handleReply returns false
//   * duplicate registerInvoke    → second call returns false, first
//     registration preserved untouched

#include "mesh/InvokeCorrelation.h"

#include <gtest/gtest.h>

#include <array>
#include <atomic>
#include <cstdint>
#include <mutex>
#include <thread>
#include <vector>

using SCE::Mesh::InvokeCorrelation;
using SCE::Mesh::RpcStatus;

namespace {

constexpr InvokeCorrelation::Key kUuidA = {
    0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12, 0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
};

constexpr InvokeCorrelation::Key kUuidB = {
    0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x70, 0x01, 0x90, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
};

/// Recording callback: captures status + payload into test-local state
/// so assertions can check what the table delivered, including how
/// many times it fired (must be exactly once per §9.5).
struct Sink {
    int calls = 0;
    RpcStatus status = RpcStatus::Ok;
    std::vector<std::uint8_t> data;

    InvokeCorrelation::DeliverCallback callback() {
        return [this](RpcStatus s, std::vector<std::uint8_t> d) {
            ++calls;
            status = s;
            data = std::move(d);
        };
    }
};

}  // namespace

TEST(InvokeCorrelation, RegisterThenReplyOk_FiresCallbackWithPayload) {
    InvokeCorrelation table;
    Sink sink;

    EXPECT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));
    EXPECT_EQ(table.size(), 1u);
    EXPECT_TRUE(table.contains(kUuidA));

    const std::vector<std::uint8_t> payload{0x01, 0x02, 0x03};
    EXPECT_TRUE(table.handleReply(kUuidA, RpcStatus::Ok, payload));

    EXPECT_EQ(sink.calls, 1);
    EXPECT_EQ(sink.status, RpcStatus::Ok);
    EXPECT_EQ(sink.data, payload);
    EXPECT_EQ(table.size(), 0u);
    EXPECT_FALSE(table.contains(kUuidA));
}

TEST(InvokeCorrelation, RegisterThenReplyError_PropagatesStatus) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));

    EXPECT_TRUE(table.handleReply(kUuidA, RpcStatus::Unavailable, {}));

    EXPECT_EQ(sink.calls, 1);
    EXPECT_EQ(sink.status, RpcStatus::Unavailable);
    EXPECT_TRUE(sink.data.empty());
    EXPECT_EQ(table.size(), 0u);
}

TEST(InvokeCorrelation, DeadlineFires_DeliveredAsDeadlineExceeded) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));

    EXPECT_TRUE(table.handleDeadline(kUuidA));

    EXPECT_EQ(sink.calls, 1);
    EXPECT_EQ(sink.status, RpcStatus::DeadlineExceeded);
    EXPECT_TRUE(sink.data.empty());
    EXPECT_EQ(table.size(), 0u);
}

TEST(InvokeCorrelation, CancelErasesWithoutFiringCallback) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));

    EXPECT_TRUE(table.handleCancel(kUuidA));

    // §9.5: <cancel> does NOT raise done/error on the cancelled invoke.
    EXPECT_EQ(sink.calls, 0);
    EXPECT_EQ(table.size(), 0u);
    EXPECT_FALSE(table.contains(kUuidA));
}

TEST(InvokeCorrelation, LateReplyAfterCancel_IsSilentlyDropped) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));
    ASSERT_TRUE(table.handleCancel(kUuidA));

    // Reply arrives after cancel — the entry is gone, so nothing
    // fires and the table reports "not found".
    EXPECT_FALSE(table.handleReply(kUuidA, RpcStatus::Ok, {0x99}));
    EXPECT_EQ(sink.calls, 0);
}

TEST(InvokeCorrelation, LateReplyAfterDeadline_IsSilentlyDropped) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));
    ASSERT_TRUE(table.handleDeadline(kUuidA));
    ASSERT_EQ(sink.calls, 1);  // deadline fired

    // Reply arrives after the deadline already fired and erased —
    // drop silently, do not invoke the callback a second time.
    EXPECT_FALSE(table.handleReply(kUuidA, RpcStatus::Ok, {0x99}));
    EXPECT_EQ(sink.calls, 1);
}

TEST(InvokeCorrelation, DuplicateRegister_IsContractViolationReturningFalse) {
    InvokeCorrelation table;
    Sink first;
    Sink second;

    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", first.callback()));
    EXPECT_FALSE(table.registerInvoke(kUuidA, "motor", second.callback()));

    // The first registration is preserved. A reply fires only the
    // first callback, not the duplicate.
    EXPECT_TRUE(table.handleReply(kUuidA, RpcStatus::Ok, {}));
    EXPECT_EQ(first.calls, 1);
    EXPECT_EQ(second.calls, 0);
}

TEST(InvokeCorrelation, IndependentUuids_DoNotInterfere) {
    InvokeCorrelation table;
    Sink sinkA;
    Sink sinkB;

    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sinkA.callback()));
    ASSERT_TRUE(table.registerInvoke(kUuidB, "brake", sinkB.callback()));
    EXPECT_EQ(table.size(), 2u);

    // Reply to A leaves B untouched.
    ASSERT_TRUE(table.handleReply(kUuidA, RpcStatus::Ok, {0xaa}));
    EXPECT_EQ(sinkA.calls, 1);
    EXPECT_EQ(sinkB.calls, 0);
    EXPECT_EQ(table.size(), 1u);
    EXPECT_TRUE(table.contains(kUuidB));
}

TEST(InvokeCorrelation, HandleReply_OnUnknownId_ReturnsFalse) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));

    EXPECT_FALSE(table.handleReply(kUuidB, RpcStatus::Ok, {0xff}));
    EXPECT_EQ(sink.calls, 0);
    EXPECT_EQ(table.size(), 1u);
}

TEST(InvokeCorrelation, CancelAllPending_IteratesEntriesWithTargetAndErases) {
    // §10.4.1 row 1704 + §16.7 row 5: the TransportRouter shutdown
    // path needs an enumerate-and-cancel API that delivers
    // `(uuid, target)` per outstanding entry so the row-5 raise can
    // surface both fields without a parallel reverse index. Pins:
    //   1. on_each fires once per registered entry.
    //   2. the captured target matches what registerInvoke received.
    //   3. neither deliver callback fires (cancelAllPending follows
    //      handleCancel's erase-without-delivery contract).
    //   4. the table is empty after the call.
    InvokeCorrelation table;
    Sink sinkA;
    Sink sinkB;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sinkA.callback()));
    ASSERT_TRUE(table.registerInvoke(kUuidB, "brake", sinkB.callback()));
    EXPECT_EQ(table.size(), 2u);

    std::vector<std::pair<InvokeCorrelation::Key, std::string>> seen;
    table.cancelAllPending(
        [&](const InvokeCorrelation::Key &uuid, const std::string &target) { seen.emplace_back(uuid, target); });

    EXPECT_EQ(table.size(), 0u) << "table empty after cancelAllPending";
    EXPECT_EQ(sinkA.calls, 0) << "deliver callbacks NOT fired (cancel-style)";
    EXPECT_EQ(sinkB.calls, 0);
    ASSERT_EQ(seen.size(), 2u);

    // Order across the unordered_map is implementation-defined, so
    // assert membership rather than positional equality.
    bool found_a = false;
    bool found_b = false;
    for (const auto &[uuid, target] : seen) {
        if (uuid == kUuidA && target == "motor") {
            found_a = true;
        }
        if (uuid == kUuidB && target == "brake") {
            found_b = true;
        }
    }
    EXPECT_TRUE(found_a) << "kUuidA → motor surfaced";
    EXPECT_TRUE(found_b) << "kUuidB → brake surfaced";
}

TEST(InvokeCorrelation, CancelAllPending_OnEmptyTable_IsNoOp) {
    // No-op contract: calling cancelAllPending on an empty registry
    // does not fire the callback. Used by TransportRouter::shutdown
    // when no mesh-rpcs were ever issued — the shutdown path should
    // not synthesize spurious row-5 events.
    InvokeCorrelation table;
    int callback_count = 0;
    table.cancelAllPending([&](const InvokeCorrelation::Key &, const std::string &) { ++callback_count; });
    EXPECT_EQ(callback_count, 0);
    EXPECT_EQ(table.size(), 0u);
}

TEST(InvokeCorrelation, CancelAllPending_NullCallback_IsTolerated) {
    // Tolerant contract: passing an empty std::function still erases
    // the entries (the cancel-like erase semantic is unconditional);
    // the null callback is silently skipped. Lets callers wire the
    // erase path even before they have a notification target wired.
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));
    table.cancelAllPending({});
    EXPECT_EQ(table.size(), 0u);
    EXPECT_EQ(sink.calls, 0);
}

TEST(InvokeCorrelation, CancelAllPendingForTarget_OnlyErasesMatchingTarget) {
    // §16.7 row 5 post-init peer-drop fast-path: a Zenoh liveliness
    // DELETE or SOMEIP availability=false for peer X fails outstanding
    // §9.5 mesh-rpc entries targeting X without disturbing entries
    // targeting other peers. Pins:
    //   1. on_each fires exactly once per matching entry.
    //   2. return value matches the erased count.
    //   3. non-matching entries stay live and still deliverable.
    //   4. deliver callbacks for erased entries do NOT fire (same
    //      erase-without-delivery semantics as handleCancel).
    InvokeCorrelation table;
    Sink sinkMotorA;
    Sink sinkMotorB;
    Sink sinkBrake;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sinkMotorA.callback()));
    // kUuidB is a different uuid; alias kUuidC manufactured locally.
    InvokeCorrelation::Key kUuidC = kUuidA;
    kUuidC[0] = 0xff;
    ASSERT_TRUE(table.registerInvoke(kUuidC, "motor", sinkMotorB.callback()));
    ASSERT_TRUE(table.registerInvoke(kUuidB, "brake", sinkBrake.callback()));
    EXPECT_EQ(table.size(), 3u);

    std::vector<InvokeCorrelation::Key> seen;
    const std::size_t erased =
        table.cancelAllPendingForTarget("motor", [&](const InvokeCorrelation::Key &uuid) { seen.push_back(uuid); });

    EXPECT_EQ(erased, 2u) << "two motor entries erased";
    EXPECT_EQ(seen.size(), 2u);
    EXPECT_EQ(table.size(), 1u) << "brake entry survives";
    EXPECT_EQ(sinkMotorA.calls, 0) << "deliver NOT fired (cancel-style)";
    EXPECT_EQ(sinkMotorB.calls, 0);

    // The surviving brake entry must still be deliverable.
    EXPECT_TRUE(table.handleReply(kUuidB, RpcStatus::Ok, {0x01}));
    EXPECT_EQ(sinkBrake.calls, 1);
    EXPECT_EQ(sinkBrake.status, RpcStatus::Ok);
}

TEST(InvokeCorrelation, CancelAllPendingForTarget_NoMatch_ReturnsZero) {
    // Peer-down for a target with no outstanding invokes: the call
    // is a no-op, returns 0, fires the callback 0 times. Lets the
    // codegen wire the cancel call unconditionally for every
    // declared peer without checking whether the table has entries.
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));

    int callback_count = 0;
    const std::size_t erased =
        table.cancelAllPendingForTarget("unknown_peer", [&](const InvokeCorrelation::Key &) { ++callback_count; });

    EXPECT_EQ(erased, 0u);
    EXPECT_EQ(callback_count, 0);
    EXPECT_EQ(table.size(), 1u) << "motor entry untouched";
}

TEST(InvokeCorrelation, CancelAllPendingForTarget_NullCallback_IsTolerated) {
    // Null callback contract mirrors cancelAllPending: erase still
    // runs, callback skipped silently. Allows the codegen to wire
    // the erase path before the notification target is hooked up.
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, "motor", sink.callback()));
    EXPECT_EQ(table.cancelAllPendingForTarget("motor", {}), 1u);
    EXPECT_EQ(table.size(), 0u);
    EXPECT_EQ(sink.calls, 0);
}

TEST(InvokeCorrelation, ConcurrentReplyVsCancel_AtMostOneWinsExactlyOnce) {
    // §9.5 cancel-vs-reply race. The correlation table must ensure
    // that across two threads racing on the same uuid, at most one
    // operation succeeds, and the deliver callback fires at most
    // once. Repeating over many iterations exercises different
    // interleavings.
    constexpr int kIterations = 1000;
    std::atomic<int> deliver_calls{0};
    std::atomic<int> reply_winners{0};
    std::atomic<int> cancel_winners{0};

    for (int i = 0; i < kIterations; ++i) {
        InvokeCorrelation table;
        // Fresh uuid per iteration so the test sees a fresh race.
        InvokeCorrelation::Key uuid = kUuidA;
        uuid[0] = static_cast<std::uint8_t>(i & 0xff);
        uuid[1] = static_cast<std::uint8_t>((i >> 8) & 0xff);

        ASSERT_TRUE(table.registerInvoke(uuid, "motor", [&](RpcStatus, auto) { ++deliver_calls; }));

        std::thread t_reply([&] {
            if (table.handleReply(uuid, RpcStatus::Ok, {})) {
                ++reply_winners;
            }
        });
        std::thread t_cancel([&] {
            if (table.handleCancel(uuid)) {
                ++cancel_winners;
            }
        });

        t_reply.join();
        t_cancel.join();

        EXPECT_EQ(table.size(), 0u);
    }

    // Every iteration: exactly one of {reply, cancel} won.
    EXPECT_EQ(reply_winners + cancel_winners, kIterations);

    // deliver_calls == number of iterations where reply won (cancel
    // never fires the callback). reply_winners must equal deliver_calls
    // — if the callback ever fired after cancel, this assertion would
    // miscount.
    EXPECT_EQ(deliver_calls.load(), reply_winners.load());
}
