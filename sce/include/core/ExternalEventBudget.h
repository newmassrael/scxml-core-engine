// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <atomic>
#include <cstdint>
#include <optional>
#include <utility>

namespace SCE::Core {

/**
 * @brief How many external events one invocation of the main event loop may
 *        take off the queue before an engine hands control back
 *
 * ARCHITECTURE.md "External-Event Budget" is the contract; this is the number
 * it states, and the one place it is spelled for the two C++ engines. A machine
 * that answers an event by sending itself the next one, with no target, never
 * lets the external queue empty. Every macrostep of it ends, so
 * `MAX_MACROSTEP_MICROSTEPS` never applies, and the loop takes the next event
 * whenever the queue is not empty: the host call that drove it did not return.
 * §scxml-3.13 lets a macrostep fail to end and says nothing of a chain of
 * macrosteps, so, as with the microstep ceiling, the number is an engine's to
 * choose and the decline has to be visible.
 *
 * It is three orders of magnitude above the longest invocation measured over
 * the authoring suite (6 external events). It is a margin and not a proof, and
 * a host that hands a machine a backlog it means to work through in one call can
 * choose another (`ExternalEventBudget::setLimit`).
 */
inline constexpr uint32_t DEFAULT_MAX_EXTERNAL_EVENTS_PER_CALL = 10000;

/**
 * @brief The budget of one invocation of the main event loop, and the record of
 *        the invocations an engine handed back because they had spent it
 *
 * Both C++ engines run the same loop shape, so the arithmetic is written once:
 * the AOT engine over its generated `Event` enum, the Interpreter over the names
 * its raiser queues. An engine holds one of these, asks `admitNext` before it
 * takes each external event off the queue, and reads the count and the head back
 * through its own accessors.
 *
 * @tparam Head What names the event an invocation was still taking when it was
 *              cut: the generated `Event` for the AOT engine, the event's name
 *              for the Interpreter
 */
template <typename Head> class ExternalEventBudget {
public:
    ExternalEventBudget() = default;

    // An atomic member deletes the implicit copy, and an engine holding this has
    // never been uncopyable on its account.
    ExternalEventBudget(const ExternalEventBudget &other)
        : limit_(other.limit_.load(std::memory_order_relaxed)), cuts_(other.cuts_), head_(other.head_) {}

    ExternalEventBudget &operator=(const ExternalEventBudget &other) {
        limit_.store(other.limit_.load(std::memory_order_relaxed), std::memory_order_relaxed);
        cuts_ = other.cuts_;
        head_ = other.head_;
        return *this;
    }

    /// @brief The most external events one invocation may take
    uint32_t limit() const noexcept {
        return limit_.load(std::memory_order_relaxed);
    }

    /**
     * @brief Let a host choose the budget of one invocation
     *
     * A host that hands the machine a backlog it means the machine to work
     * through in one call knows its size, and the engine does not. Settable at
     * any time and from any thread: the engine reads it once per event it is
     * about to take, so a value set while a call is running governs the rest of
     * that call.
     *
     * Refused below one: returns false and changes nothing. A budget that takes
     * no event is a machine that cannot run, not a stricter one, since every
     * call would hand control back with the queue untouched and say it had been
     * cut.
     */
    bool setLimit(uint32_t limit) noexcept {
        if (limit < 1) {
            return false;
        }
        limit_.store(limit, std::memory_order_relaxed);
        return true;
    }

    /// @brief Whether an invocation that has taken @p taken events has used up the budget
    bool spentBy(uint32_t taken) const noexcept {
        return taken >= limit();
    }

    /**
     * @brief Ask for leave to take one more event, counting it when granted
     *
     * Asked once the engine has seen that an event is waiting, and BEFORE it
     * leaves the queue: a refusal leaves the queue exactly as it is, so the
     * event it declined is still there for the next invocation, which gets a
     * budget of its own. An invocation that takes exactly the budget and empties
     * the queue is never refused anything, because it never asks again.
     *
     * When refused the cut is recorded here, with whatever @p headOfQueue
     * answers: a callable returning `std::optional<Head>`, called only then, so
     * the engine builds the event it names on the path that is cut and not on
     * every event it takes. Empty for a due scheduler entry that delivers no
     * event of this machine's.
     *
     * @param taken The events this invocation has taken so far; incremented when granted
     * @param headOfQueue Names the event that was still waiting
     * @return true when the engine may take the event, false when it must hand control back
     */
    template <typename HeadOfQueue> bool admitNext(uint32_t &taken, HeadOfQueue &&headOfQueue) {
        if (spentBy(taken)) {
            recordCut(std::forward<HeadOfQueue>(headOfQueue)());
            return false;
        }
        ++taken;
        return true;
    }

    /// @brief Invocations handed back with work still waiting, since the engine was built
    uint32_t cuts() const noexcept {
        return cuts_;
    }

    /**
     * @brief What the last cut invocation was still taking
     *
     * Empty while `cuts()` is zero, and for a cut that had no event of this
     * machine's to name. Held as an optional because the generated `Event`
     * enum's zero value is a real event and cannot stand in for "none".
     */
    const std::optional<Head> &lastCutHead() const noexcept {
        return head_;
    }

private:
    void recordCut(std::optional<Head> head) {
        ++cuts_;
        head_ = std::move(head);
    }

    std::atomic<uint32_t> limit_{DEFAULT_MAX_EXTERNAL_EVENTS_PER_CALL};
    uint32_t cuts_ = 0;
    std::optional<Head> head_{};
};

}  // namespace SCE::Core
