// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RetryingDispatcher unit tests — SCE Mesh §16.7 row 3 retry layer.
//
// Covers the contract pinned in `sce/include/mesh/RetryingDispatcher.h`:
//   * First-failure-then-success: inner dispatcher fails once,
//     then succeeds on retry → no DELIVERY_EXHAUSTED, state erased.
//   * Exhaustion: every attempt fails → DELIVERY_EXHAUSTED with
//     `attempts == max_retries + 1`.
//   * Terminal fast-fail: inner returns `retryable=false` →
//     DELIVERY_EXHAUSTED with `attempts == 1` (no retries consumed).
//   * Cancel preempts retry: `cancelEnvelopeRetry(id)` erases state
//     and the scheduler entry; no DELIVERY_EXHAUSTED fires.
//   * Defensive `max_retries == 0` pathway: returns the inner's
//     `SendResult::failure()` unchanged so SEND_FAILED would fire
//     (the deploy.yaml validator rejects this configuration, so
//     this path is only reachable via programmatic construction).
//   * Backoff timing: exponential with multiplier; capped at
//     `max_backoff`.
//   * Jitter: bounded — every observed interval lies within
//     `[base*(1-pct/100), base*(1+pct/100)]`.
//
// Uses a `MockTransport` that records calls + lets the test drive
// per-attempt success/failure. The MeshDeadlineScheduler is real —
// timing tests use a short backoff so total runtime stays sub-second.

#include "mesh/OutboundBuffer.h"
#include "mesh/RetryingDispatcher.h"

#include <gtest/gtest.h>

#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <vector>

using SCE::Mesh::CommunicationError;
using SCE::Mesh::MeshDeadlineScheduler;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::RetryingDispatcher;
using SCE::Mesh::SendResult;
using namespace std::chrono_literals;

namespace {

// A mock that returns a sequence of pre-programmed SendResults and
// records the timestamps of each call. Thread-safe.
class MockTransport {
public:
    void program(std::vector<SendResult> sequence) {
        std::lock_guard<std::mutex> lock(mu_);
        sequence_ = std::move(sequence);
        idx_ = 0;
    }

    SendResult operator()(const MeshEnvelope& env) {
        std::lock_guard<std::mutex> lock(mu_);
        calls_.push_back(std::chrono::steady_clock::now());
        last_env_id_ = env.id;
        if (idx_ < sequence_.size()) {
            return sequence_[idx_++];
        }
        // Out of programmed responses ⇒ behave as success.
        return SendResult::success();
    }

    std::size_t callCount() const {
        std::lock_guard<std::mutex> lock(mu_);
        return calls_.size();
    }

    std::vector<std::chrono::steady_clock::time_point> callTimes() const {
        std::lock_guard<std::mutex> lock(mu_);
        return calls_;
    }

    std::array<std::uint8_t, 16> lastEnvId() const {
        std::lock_guard<std::mutex> lock(mu_);
        return last_env_id_;
    }

private:
    mutable std::mutex mu_;
    std::vector<SendResult> sequence_;
    std::size_t idx_ = 0;
    std::vector<std::chrono::steady_clock::time_point> calls_;
    std::array<std::uint8_t, 16> last_env_id_{};
};

// Captures CommunicationError raises with a condition_variable so
// tests can wait deterministically rather than sleeping.
class RaiseRecorder {
public:
    void operator()(CommunicationError err) {
        std::unique_lock<std::mutex> lock(mu_);
        raised_.push_back(std::move(err));
        cv_.notify_all();
    }

    [[nodiscard]] bool waitFor(std::size_t count, std::chrono::milliseconds timeout) {
        std::unique_lock<std::mutex> lock(mu_);
        return cv_.wait_for(lock, timeout, [this, count] {
            return raised_.size() >= count;
        });
    }

    std::vector<CommunicationError> snapshot() const {
        std::lock_guard<std::mutex> lock(mu_);
        return raised_;
    }

    std::size_t count() const {
        std::lock_guard<std::mutex> lock(mu_);
        return raised_.size();
    }

private:
    mutable std::mutex mu_;
    std::condition_variable cv_;
    std::vector<CommunicationError> raised_;
};

RetryingDispatcher::Policy small_policy(std::uint32_t max_retries) {
    return RetryingDispatcher::Policy{
        max_retries,
        /*initial_backoff=*/std::chrono::milliseconds(10),
        /*backoff_multiplier=*/2.0,
        /*max_backoff=*/std::chrono::milliseconds(100),
        /*jitter_pct=*/0,  // deterministic for timing tests
        /*transport=*/"zenoh",
        /*target=*/"motor",
    };
}

MeshEnvelope make_envelope() {
    MeshEnvelope env;
    // Distinguishable id so the lastEnvId() probe is meaningful.
    env.id = {0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
              0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10};
    env.source = "test";
    env.type = "evt";
    return env;
}

}  // namespace

TEST(RetryingDispatcherTest, FirstFailureThenSuccessNoExhaustionRaise) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    mock.program({
        SendResult::failure("transient", /*retryable=*/true),
        SendResult::success(),
    });
    RetryingDispatcher dispatcher(scheduler, small_policy(/*max_retries=*/3),
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    SendResult initial = dispatcher.send_with_retry(env);
    EXPECT_TRUE(initial.ok);  // wrapper hides the transient failure

    // Wait for the retry callback to fire. Backoff=10ms, so 200ms is plenty.
    std::this_thread::sleep_for(200ms);
    EXPECT_EQ(mock.callCount(), 2u);
    EXPECT_EQ(recorder.count(), 0u);
    EXPECT_EQ(dispatcher.pendingCount(), 0u);
}

TEST(RetryingDispatcherTest, ExhaustionRaisesDeliveryExhaustedWithAttempts) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    // max_retries=2 ⇒ attempts budget = 3 (1 initial + 2 retries).
    mock.program({
        SendResult::failure("retryable1", /*retryable=*/true),
        SendResult::failure("retryable2", /*retryable=*/true),
        SendResult::failure("final", /*retryable=*/true),
    });
    RetryingDispatcher dispatcher(scheduler, small_policy(/*max_retries=*/2),
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    SendResult initial = dispatcher.send_with_retry(env);
    EXPECT_TRUE(initial.ok);

    ASSERT_TRUE(recorder.waitFor(1, 1000ms));
    auto raised = recorder.snapshot();
    ASSERT_EQ(raised.size(), 1u);
    EXPECT_EQ(raised[0].reason, "DELIVERY_EXHAUSTED");
    EXPECT_EQ(raised[0].target, "motor");
    EXPECT_EQ(raised[0].transport, "zenoh");
    ASSERT_TRUE(raised[0].attempts.has_value());
    EXPECT_EQ(*raised[0].attempts, 3);
    ASSERT_TRUE(raised[0].transport_error.has_value());
    EXPECT_EQ(*raised[0].transport_error, "final");
    EXPECT_EQ(mock.callCount(), 3u);
    EXPECT_EQ(dispatcher.pendingCount(), 0u);
}

TEST(RetryingDispatcherTest, TerminalFailureFastFailsWithAttemptsOne) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    mock.program({
        SendResult::failure("vsomeip app not initialized", /*retryable=*/false),
    });
    RetryingDispatcher dispatcher(scheduler, small_policy(/*max_retries=*/5),
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    SendResult initial = dispatcher.send_with_retry(env);
    EXPECT_TRUE(initial.ok);  // wrapper still hides from OutboundBuffer

    auto raised = recorder.snapshot();
    ASSERT_EQ(raised.size(), 1u);
    EXPECT_EQ(raised[0].reason, "DELIVERY_EXHAUSTED");
    ASSERT_TRUE(raised[0].attempts.has_value());
    EXPECT_EQ(*raised[0].attempts, 1);
    EXPECT_EQ(*raised[0].transport_error, "vsomeip app not initialized");
    EXPECT_EQ(mock.callCount(), 1u);
    EXPECT_EQ(dispatcher.pendingCount(), 0u);
}

TEST(RetryingDispatcherTest, CancelEnvelopeRetryPreemptsScheduledRetry) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    // First call fails (retryable). The retry would succeed if it ever
    // fired — but the test cancels first.
    mock.program({
        SendResult::failure("transient", /*retryable=*/true),
        SendResult::success(),
    });
    // Long backoff so the cancel has time to land before the retry fires.
    auto policy = small_policy(/*max_retries=*/5);
    policy.initial_backoff = std::chrono::milliseconds(500);
    policy.max_backoff = std::chrono::milliseconds(500);
    RetryingDispatcher dispatcher(scheduler, policy,
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    dispatcher.send_with_retry(env);
    EXPECT_EQ(dispatcher.pendingCount(), 1u);
    EXPECT_TRUE(dispatcher.cancelEnvelopeRetry(env.id));
    EXPECT_EQ(dispatcher.pendingCount(), 0u);

    // Wait past the original backoff window. The retry must NOT fire.
    std::this_thread::sleep_for(700ms);
    EXPECT_EQ(mock.callCount(), 1u);  // only the initial attempt
    EXPECT_EQ(recorder.count(), 0u);
}

TEST(RetryingDispatcherTest, MaxRetriesZeroDefensivePathPassesThroughFailure) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    mock.program({
        SendResult::failure("just fail", /*retryable=*/true),
    });
    auto policy = small_policy(/*max_retries=*/0);  // defensive — validator rejects
    RetryingDispatcher dispatcher(scheduler, policy,
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    SendResult initial = dispatcher.send_with_retry(env);
    EXPECT_FALSE(initial.ok);  // pass-through, OutboundBuffer would raise SEND_FAILED
    ASSERT_TRUE(initial.transport_error.has_value());
    EXPECT_EQ(*initial.transport_error, "just fail");
    EXPECT_EQ(recorder.count(), 0u);  // no DELIVERY_EXHAUSTED from this path
    EXPECT_EQ(dispatcher.pendingCount(), 0u);
}

TEST(RetryingDispatcherTest, ExponentialBackoffGrowsWithMultiplier) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    // 3 retryable failures + 1 success ⇒ 4 inner calls total.
    mock.program({
        SendResult::failure("f1", /*retryable=*/true),
        SendResult::failure("f2", /*retryable=*/true),
        SendResult::failure("f3", /*retryable=*/true),
        SendResult::success(),
    });
    auto policy = small_policy(/*max_retries=*/5);
    policy.initial_backoff = std::chrono::milliseconds(20);
    policy.backoff_multiplier = 2.0;
    policy.max_backoff = std::chrono::milliseconds(200);
    policy.jitter_pct = 0;  // deterministic
    RetryingDispatcher dispatcher(scheduler, policy,
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    dispatcher.send_with_retry(env);

    // Wait long enough for all 4 calls: 20 + 40 + 80 = 140ms total,
    // plus scheduler overhead.
    std::this_thread::sleep_for(500ms);
    ASSERT_EQ(mock.callCount(), 4u);
    auto times = mock.callTimes();
    auto gap1 = std::chrono::duration_cast<std::chrono::milliseconds>(times[1] - times[0]).count();
    auto gap2 = std::chrono::duration_cast<std::chrono::milliseconds>(times[2] - times[1]).count();
    auto gap3 = std::chrono::duration_cast<std::chrono::milliseconds>(times[3] - times[2]).count();
    // Each gap should be roughly 2x the previous one. Allow a wide
    // tolerance for scheduler latency (the test must be robust under
    // CI load) but reject obviously-wrong orderings.
    EXPECT_GE(gap1, 15);
    EXPECT_GE(gap2, gap1);  // exponential growth
    EXPECT_GE(gap3, gap2);
    EXPECT_LE(gap3, 220);   // cap at max_backoff (with slack)
}

// Integration test mirroring the codegen wiring: RetryingDispatcher
// wraps the transport-send closure, and the OutboundBuffer's dispatcher
// routes through `send_with_retry`. The OutboundBuffer must never
// raise SEND_FAILED on the transient-then-success path because the
// retry wrapper hides the inner failure; only DELIVERY_EXHAUSTED
// fires (or nothing, on success). This pins the seam the codegen
// emits at `<machine>_outbound_(... wrapped_dispatch ...)`.
TEST(RetryingDispatcherTest, OutboundBufferWiringHidesTransientFailuresAndRaisesExhaustion) {
    using SCE::Mesh::OutboundBuffer;
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder send_failed_recorder;  // OutboundBuffer's row 2 raise sink
    RaiseRecorder delivery_recorder;     // RetryingDispatcher's row 3 raise sink
    mock.program({
        SendResult::failure("transient1", /*retryable=*/true),
        SendResult::failure("transient2", /*retryable=*/true),
        SendResult::success(),
    });
    RetryingDispatcher retrying(scheduler, small_policy(/*max_retries=*/3),
                                [&mock](const MeshEnvelope& env) { return mock(env); },
                                [&delivery_recorder](CommunicationError err) {
                                    delivery_recorder(std::move(err));
                                });

    OutboundBuffer outbound(
        /*target=*/"motor", /*max_pending=*/4, /*transport=*/"zenoh",
        [&retrying](const MeshEnvelope& env) {
            return retrying.send_with_retry(env);
        },
        [&send_failed_recorder](CommunicationError err) {
            send_failed_recorder(std::move(err));
        });

    outbound.markReady();  // simulate the transport availability anchor
    EXPECT_TRUE(outbound.admit(make_envelope()));

    // Allow the retry chain to complete (transient → transient → success).
    std::this_thread::sleep_for(500ms);
    EXPECT_EQ(mock.callCount(), 3u);
    // Critical: OutboundBuffer must NOT have raised SEND_FAILED — the
    // retry wrapper hid the per-attempt failures behind its own
    // success() return.
    EXPECT_EQ(send_failed_recorder.count(), 0u);
    // And the success means no DELIVERY_EXHAUSTED either.
    EXPECT_EQ(delivery_recorder.count(), 0u);
    EXPECT_EQ(retrying.pendingCount(), 0u);
}

TEST(RetryingDispatcherTest, OutboundBufferWiringExhaustionRaisesDeliveryExhausted) {
    using SCE::Mesh::OutboundBuffer;
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder send_failed_recorder;
    RaiseRecorder delivery_recorder;
    // max_retries=1 ⇒ 2 attempts max. Both fail (retryable).
    mock.program({
        SendResult::failure("f1", /*retryable=*/true),
        SendResult::failure("f2", /*retryable=*/true),
    });
    RetryingDispatcher retrying(scheduler, small_policy(/*max_retries=*/1),
                                [&mock](const MeshEnvelope& env) { return mock(env); },
                                [&delivery_recorder](CommunicationError err) {
                                    delivery_recorder(std::move(err));
                                });

    OutboundBuffer outbound(
        /*target=*/"motor", /*max_pending=*/4, /*transport=*/"zenoh",
        [&retrying](const MeshEnvelope& env) {
            return retrying.send_with_retry(env);
        },
        [&send_failed_recorder](CommunicationError err) {
            send_failed_recorder(std::move(err));
        });

    outbound.markReady();
    EXPECT_TRUE(outbound.admit(make_envelope()));

    ASSERT_TRUE(delivery_recorder.waitFor(1, 1000ms));
    auto raised = delivery_recorder.snapshot();
    ASSERT_EQ(raised.size(), 1u);
    EXPECT_EQ(raised[0].reason, "DELIVERY_EXHAUSTED");
    EXPECT_TRUE(raised[0].attempts.has_value());
    EXPECT_EQ(*raised[0].attempts, 2);
    // SEND_FAILED must NOT have fired (per Q6=(c) "DELIVERY_EXHAUSTED
    // only when max_retries>0").
    EXPECT_EQ(send_failed_recorder.count(), 0u);
}

TEST(RetryingDispatcherTest, JitterStaysWithinDeclaredBand) {
    MeshDeadlineScheduler scheduler;
    MockTransport mock;
    RaiseRecorder recorder;
    // Drive many retries to sample the jitter distribution.
    std::vector<SendResult> seq;
    for (int i = 0; i < 8; ++i) {
        seq.push_back(SendResult::failure("f", /*retryable=*/true));
    }
    seq.push_back(SendResult::success());
    mock.program(std::move(seq));

    auto policy = small_policy(/*max_retries=*/8);
    policy.initial_backoff = std::chrono::milliseconds(40);
    policy.backoff_multiplier = 1.0;  // fixed base
    policy.max_backoff = std::chrono::milliseconds(40);
    policy.jitter_pct = 50;  // ±50%

    RetryingDispatcher dispatcher(scheduler, policy,
                                  [&mock](const MeshEnvelope& env) {
                                      return mock(env);
                                  },
                                  [&recorder](CommunicationError err) {
                                      recorder(std::move(err));
                                  });

    auto env = make_envelope();
    dispatcher.send_with_retry(env);

    // Allow up to 8 retries × max 60ms = 480ms + scheduler overhead.
    std::this_thread::sleep_for(1500ms);
    ASSERT_GE(mock.callCount(), 2u);
    auto times = mock.callTimes();
    for (std::size_t i = 1; i < times.size(); ++i) {
        auto gap_ms = std::chrono::duration_cast<std::chrono::milliseconds>(times[i] - times[i - 1]).count();
        // ±50% of 40ms ⇒ [20, 60]. Add scheduler-overhead slack on
        // the upper bound (the lower bound is jitter's hard floor of
        // 1ms in jitter(), but only after the floor clamp kicks in).
        EXPECT_GE(gap_ms, 15);
        EXPECT_LE(gap_ms, 100);
    }
}
