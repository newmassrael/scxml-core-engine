// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <chrono>
#include <cstdint>
#include <memory>
#include <stdexcept>

namespace SCE {

/**
 * @brief The source of "now" behind every `<send delay>` an engine arms
 *
 * §scxml-6.2.2 says a delay "indicates how long the processor should wait
 * before dispatching the message", and says nothing about where the processor
 * reads the time from. Leaving that hardwired to the wall answers a question
 * the spec left to the host, and answers it the one way that cannot be
 * reproduced: a host descheduled between two statements of the same `<onentry>`
 * gets two different readings for one instant, and the deadlines it computes
 * from them can order the sends differently on every run.
 *
 * So the reading is a seam, not a constant. `MonotonicClock` is the default and
 * is what a production host wants; `ManualClock` hands the clock to the host
 * outright, which is what a simulation, a replay, a discrete-event scheduler
 * and a deterministic test all want. Both are shipped runtime types — a
 * consumer can install either, or write a third.
 *
 * Not to be confused with `LogicalTimeScheduler` / `GameLoopTimer`, which give
 * a host logical time in a queue of *their* own: a document's `<send delay>` is
 * armed by generated code calling the engine's `scheduleEvent`, so only a clock
 * the engine itself reads can reach it.
 */
class ISceClock {
public:
    virtual ~ISceClock() = default;

    /**
     * @brief Milliseconds elapsed since this clock's origin
     *
     * Must be non-decreasing: the scheduler compares readings taken at
     * different moments, and a reading that went backwards would make an entry
     * that was due stop being due.
     */
    virtual uint64_t elapsedMs() const = 0;
};

/**
 * @brief The default `ISceClock` — a monotonic reading of the host's clock
 *
 * Measured from the moment this clock was constructed. This is what an engine
 * gets when nothing else is installed, and what a production host running
 * against real time should keep.
 */
class MonotonicClock : public ISceClock {
public:
    MonotonicClock() : origin_(std::chrono::steady_clock::now()) {}

    uint64_t elapsedMs() const override {
        auto delta = std::chrono::steady_clock::now() - origin_;
        return static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::milliseconds>(delta).count());
    }

private:
    std::chrono::steady_clock::time_point origin_;
};

/**
 * @brief An `ISceClock` the host moves by hand
 *
 * Time advances only when `advance()` is called, so a machine driven through
 * one of these reaches the same configuration on every run regardless of what
 * else the machine it runs on is doing. That is what makes it the right clock
 * for a simulation, for replaying a recorded trace at a speed of the host's
 * choosing, and for a test that wants a verdict about the engine rather than
 * about the load on the build machine.
 *
 * Install it before `initialize()`, and drive the machine with
 * `advanceTimeMs()` rather than calling `advance()` directly — the engine's
 * entry point moves this clock and then runs whatever that made due, which is
 * the whole of the contract.
 *
 * One instance may be shared by several engines, so a parent and the sessions
 * it invokes read the same absolute time (§scxml-6.4).
 */
class ManualClock : public ISceClock {
public:
    explicit ManualClock(uint64_t startMs = 0) : nowMs_(startMs) {}

    uint64_t elapsedMs() const override {
        return nowMs_;
    }

    /**
     * @brief Move this clock forward by `ms` milliseconds
     *
     * The parameter is unsigned, so a clock that went backwards — which would
     * un-due an entry the scheduler had already judged ready — is not
     * expressible rather than merely rejected.
     */
    void advance(uint64_t ms) {
        nowMs_ += ms;
    }

private:
    uint64_t nowMs_ = 0;
};

}  // namespace SCE
