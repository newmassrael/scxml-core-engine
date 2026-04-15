// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
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
    0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12,
    0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
};

constexpr InvokeCorrelation::Key kUuidB = {
    0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x70, 0x01,
    0x90, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
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

    EXPECT_TRUE(table.registerInvoke(kUuidA, sink.callback()));
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
    ASSERT_TRUE(table.registerInvoke(kUuidA, sink.callback()));

    EXPECT_TRUE(table.handleReply(kUuidA, RpcStatus::Unavailable, {}));

    EXPECT_EQ(sink.calls, 1);
    EXPECT_EQ(sink.status, RpcStatus::Unavailable);
    EXPECT_TRUE(sink.data.empty());
    EXPECT_EQ(table.size(), 0u);
}

TEST(InvokeCorrelation, DeadlineFires_DeliveredAsDeadlineExceeded) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, sink.callback()));

    EXPECT_TRUE(table.handleDeadline(kUuidA));

    EXPECT_EQ(sink.calls, 1);
    EXPECT_EQ(sink.status, RpcStatus::DeadlineExceeded);
    EXPECT_TRUE(sink.data.empty());
    EXPECT_EQ(table.size(), 0u);
}

TEST(InvokeCorrelation, CancelErasesWithoutFiringCallback) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, sink.callback()));

    EXPECT_TRUE(table.handleCancel(kUuidA));

    // §9.5: <cancel> does NOT raise done/error on the cancelled invoke.
    EXPECT_EQ(sink.calls, 0);
    EXPECT_EQ(table.size(), 0u);
    EXPECT_FALSE(table.contains(kUuidA));
}

TEST(InvokeCorrelation, LateReplyAfterCancel_IsSilentlyDropped) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, sink.callback()));
    ASSERT_TRUE(table.handleCancel(kUuidA));

    // Reply arrives after cancel — the entry is gone, so nothing
    // fires and the table reports "not found".
    EXPECT_FALSE(table.handleReply(kUuidA, RpcStatus::Ok, {0x99}));
    EXPECT_EQ(sink.calls, 0);
}

TEST(InvokeCorrelation, LateReplyAfterDeadline_IsSilentlyDropped) {
    InvokeCorrelation table;
    Sink sink;
    ASSERT_TRUE(table.registerInvoke(kUuidA, sink.callback()));
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

    ASSERT_TRUE(table.registerInvoke(kUuidA, first.callback()));
    EXPECT_FALSE(table.registerInvoke(kUuidA, second.callback()));

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

    ASSERT_TRUE(table.registerInvoke(kUuidA, sinkA.callback()));
    ASSERT_TRUE(table.registerInvoke(kUuidB, sinkB.callback()));
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
    ASSERT_TRUE(table.registerInvoke(kUuidA, sink.callback()));

    EXPECT_FALSE(table.handleReply(kUuidB, RpcStatus::Ok, {0xff}));
    EXPECT_EQ(sink.calls, 0);
    EXPECT_EQ(table.size(), 1u);
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

        ASSERT_TRUE(table.registerInvoke(uuid, [&](RpcStatus, auto) {
            ++deliver_calls;
        }));

        std::thread t_reply(
            [&] { if (table.handleReply(uuid, RpcStatus::Ok, {})) ++reply_winners; });
        std::thread t_cancel(
            [&] { if (table.handleCancel(uuid)) ++cancel_winners; });

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
