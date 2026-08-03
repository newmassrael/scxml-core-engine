// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 3 follow-up — deadline-preempts-retry composition.
//
// The codegen-emitted deadline lambda fires the sequence
//   invoke_correlation_.handleDeadline(invoke_uuid);
//   cancelEnvelopeRetryById(envelope_uuid);
// when a binding declares both an `<invoke type="sce:mesh-rpc">` with
// `_mesh_deadline_ms` AND a `retry:` block. This unit-integration test
// exercises that composition end-to-end against the real
// RetryingDispatcher + InvokeCorrelation + MeshDeadlineScheduler
// classes — proving the contract without needing a full someip /
// zenoh transport in the loop.
//
// The row 3 atomic originally aliased the deadline scheduler key
// with the retry scheduler key (both used the single `uuid` that
// invokeMeshRpc assigned to env.id AND env.invoke_id). The row 3
// follow-up splits the two: invoke_uuid keys correlation + deadline,
// envelope_uuid keys retry + dedup. The two key spaces are disjoint,
// so a deadline armed with invoke_uuid does not collide with a
// retry-backoff scheduler entry keyed by envelope_uuid.
//
// Assertions:
//   * RetryingDispatcher queues a retry attempt without colliding
//     with the invoke deadline (split-uuid invariant).
//   * Deliver callback fires once with DeadlineExceeded — the §9.5
//     error.invoke.<id> raise path.
//   * RetryingDispatcher pendingCount drops to zero after the deadline
//     lambda runs (the cancel half of the composition).
//   * No DELIVERY_EXHAUSTED CommunicationError is raised — the retry
//     chain was preempted before its exhaustion path could fire.

#include "mesh/InvokeCorrelation.h"
#include "mesh/MeshDeadlineScheduler.h"
#include "mesh/OutboundBuffer.h"
#include "mesh/RetryingDispatcher.h"

#include <gtest/gtest.h>

#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <thread>
#include <vector>

using SCE::Mesh::CommunicationError;
using SCE::Mesh::InvokeCorrelation;
using SCE::Mesh::MeshDeadlineScheduler;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::RetryingDispatcher;
using SCE::Mesh::RpcStatus;
using SCE::Mesh::SendResult;
using namespace std::chrono_literals;

namespace {

// Records every DELIVERY_EXHAUSTED raise so the test can assert it
// did NOT fire. Mirrors the recorder shape from RetryingDispatcherTest.
class RaiseRecorder {
public:
    void operator()(CommunicationError err) {
        std::lock_guard<std::mutex> lock(mu_);
        raised_.push_back(std::move(err));
    }

    std::size_t count() const {
        std::lock_guard<std::mutex> lock(mu_);
        return raised_.size();
    }

private:
    mutable std::mutex mu_;
    std::vector<CommunicationError> raised_;
};

// Records deliver-callback invocations from InvokeCorrelation.
class DeliverRecorder {
public:
    void operator()(RpcStatus status, std::vector<std::uint8_t> /*data*/) {
        std::unique_lock<std::mutex> lock(mu_);
        statuses_.push_back(status);
        cv_.notify_all();
    }

    bool wait_for_status(RpcStatus expected, std::chrono::milliseconds timeout) {
        std::unique_lock<std::mutex> lock(mu_);
        return cv_.wait_for(lock, timeout, [&] { return !statuses_.empty() && statuses_.front() == expected; });
    }

    std::size_t count() const {
        std::lock_guard<std::mutex> lock(mu_);
        return statuses_.size();
    }

private:
    mutable std::mutex mu_;
    std::condition_variable cv_;
    std::vector<RpcStatus> statuses_;
};

// Distinct envelope and invoke uuids — mirrors the split-uuid layout
// invokeMeshRpc emits per §16.7 row 3 follow-up.
constexpr std::array<std::uint8_t, 16> kInvokeUuid = {0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11,
                                                      0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99};
constexpr std::array<std::uint8_t, 16> kEnvelopeUuid = {0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                                                        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00};

MeshEnvelope make_envelope() {
    MeshEnvelope env;
    env.id = kEnvelopeUuid;
    env.invoke_id = kInvokeUuid;
    env.type = "service.request.compute";
    env.pattern = SCE::Mesh::PatternKind::RpcRequest;
    return env;
}

RetryingDispatcher::Policy make_policy() {
    RetryingDispatcher::Policy policy{};
    policy.max_retries = 5;
    // Backoff longer than the deadline so the retry never fires on
    // its own — the test must observe the deadline preempting the
    // queued retry rather than the retry winning the race.
    policy.initial_backoff = std::chrono::milliseconds(2000);
    policy.backoff_multiplier = 2.0;
    policy.max_backoff = std::chrono::milliseconds(2000);
    policy.jitter_pct = 0;
    policy.transport = "someip";
    policy.target = "#motor";
    return policy;
}

}  // namespace

TEST(DeadlinePreemptsRetryTest, DeadlineLambdaCancelsPendingRetry) {
    MeshDeadlineScheduler scheduler;
    InvokeCorrelation correlation;
    RaiseRecorder recorder;
    DeliverRecorder deliver;

    // Inner closure: always fails (retryable) so send_with_retry queues
    // the retry attempt rather than completing synchronously.
    auto inner = [](const MeshEnvelope &) { return SendResult::failure("simulated transient", /*retryable=*/true); };

    RetryingDispatcher dispatcher(scheduler, make_policy(), inner,
                                  [&recorder](CommunicationError err) { recorder(std::move(err)); });

    auto env = make_envelope();
    ASSERT_NE(env.id, kInvokeUuid) << "fixture invariant — envelope and invoke uuids must differ";

    // Register the invoke correlation entry first — mirrors the
    // codegen order: registerInvoke BEFORE route_send so a synchronous
    // reply (or here, a synchronous deadline) cannot race the entry's
    // creation. Correlation keys off the INVOKE uuid (matches
    // env.invoke_id).
    // §14.6 responder set: this fixture models the same-target default,
    // so the only peer allowed to retire the entry is the invoke's own
    // target. The deadline path below does not consult it — a locally
    // observed failure has no replier.
    correlation.registerInvoke(
        kInvokeUuid, "#motor", {"motor"},
        [&deliver](RpcStatus status, std::vector<std::uint8_t> data) { deliver(status, std::move(data)); });

    // Arm the deadline BEFORE send_with_retry — same order the
    // codegen-emitted invokeMeshRpc uses (registerDeadline before
    // route_send). The deadline scheduler key is the INVOKE uuid;
    // the retry scheduler key (used by send_with_retry below) is
    // the ENVELOPE uuid. The split is what makes this composition
    // work — pre-row-3-followup the two were aliased and retry
    // would refuse to register, collapsing into SEND_FAILED.
    const bool scheduled =
        scheduler.registerDeadline(kInvokeUuid, std::chrono::milliseconds(50), [&correlation, &dispatcher]() {
            (void)correlation.handleDeadline(kInvokeUuid);
            (void)dispatcher.cancelEnvelopeRetry(kEnvelopeUuid);
        });
    ASSERT_TRUE(scheduled);

    // Drive the retry chain. First attempt fails → backoff queued.
    // Retry's internal registerDeadline keys off env.id (envelope_uuid)
    // and MUST succeed — disjoint from the invoke deadline above.
    SendResult initial = dispatcher.send_with_retry(env);
    ASSERT_TRUE(initial.ok) << "retry layer must suppress SEND_FAILED while retries remain "
                               "(the split-uuid invariant prevents the row 3 collision)";
    ASSERT_EQ(dispatcher.pendingCount(), 1u) << "retry layer must queue exactly one pending retry";

    // Wait for the deadline to fire and deliver DeadlineExceeded.
    ASSERT_TRUE(deliver.wait_for_status(RpcStatus::DeadlineExceeded, 2s))
        << "deadline did not surface DeadlineExceeded within 2s";

    // The cancel half must have erased the retry queue entry.
    EXPECT_EQ(dispatcher.pendingCount(), 0u) << "deadline lambda must call cancelEnvelopeRetry; retry entry "
                                                "should be erased";

    // Wait past one backoff window to prove the queued retry stays
    // cancelled rather than firing late.
    std::this_thread::sleep_for(2500ms);
    EXPECT_EQ(dispatcher.pendingCount(), 0u);
    EXPECT_EQ(recorder.count(), 0u) << "DELIVERY_EXHAUSTED must NOT fire after the deadline preempted "
                                       "the retry chain";
    EXPECT_EQ(deliver.count(), 1u) << "exactly one delivery (the DeadlineExceeded raise) must occur";
}
