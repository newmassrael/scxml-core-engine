// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The bookkeeping of the host-run invocations of one session (docs/adr/0084, D5):
// which are running and under which token, the record made before the handler is
// asked, a refusal, an immediate completion, the deadline, the one ending, and what
// a save reads of a running invocation.
//
// The shapes a host sees are `core/HostProcessor.h`'s; this is what an engine keeps
// about them. A header of the shared layer because the bookkeeping is both C++
// engines': the generated engine holds one ledger per session, and the Interpreter
// holds the same one when it serves host-run invocations.
//
// The ledger owns no clock and raises no event. It arms and drops a deadline through
// the scheduler of the engine that holds it (`HostInvokeDeadlineSchedulerTrait`), and
// it answers each operation with what the engine is left to do (`HostInvokeReport`,
// `HostInvokeEnded`), which the engine does in its own spelling of an event.

#pragma once

#include "core/HostProcessor.h"
#include "core/InvokeHelper.h"

#include <algorithm>
#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <tuple>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace SCE::Core {

/**
 * @brief What a host-invoke ledger asks of the scheduler of the engine that holds it
 *
 * The deadline of a start is a moment of the engine's clock, and the clock is the
 * engine's, so the ledger hands the moment to the engine's scheduler and asks it
 * again by the start's token. Required, on an `S &s` and a `const S &c`:
 *
 * - `s.scheduleHostInvokeDeadlineAt(token, deadline, atMs)`: arm `deadline` (a
 *   `std::shared_ptr<const HostInvokeDeadline>`) to come due at `atMs`, and hand it
 *   back to the engine when it does, which ends the start through
 *   `HostInvokeLedger::expire`;
 * - `s.dropHostInvokeDeadline(token)`: drop the deadline of start `token`, if one is
 *   pending;
 * - `c.hostInvokeDeadlineDueAt(token)`: the moment the pending deadline of start
 *   `token` comes due, convertible to `std::optional<uint64_t>`, empty when none is
 *   pending.
 *
 * The `nowMs` an engine hands the ledger (`start`, `startRestored`, `running`) reads
 * the clock this scheduler fires by, and nothing else: a moment read off one clock and
 * fired by another comes due at a moment nobody named. A pull scheduler is told the
 * time on each pop, so the engine hands both the one clock it pops with; a scheduler
 * that keeps a clock of its own (the Interpreter's) answers the reading itself, and
 * the engine hands the ledger that. The generated engine's `PullScheduler` serves as
 * it is. A C++17 trait, so it means
 * the same thing in a consumer's C++17 translation unit and in SCE's C++20 ones
 * (docs/adr/0059).
 */
template <typename S, typename = void> struct HostInvokeDeadlineSchedulerTrait : std::false_type {};

template <typename S>
struct HostInvokeDeadlineSchedulerTrait<
    S, std::void_t<decltype(std::declval<S &>().scheduleHostInvokeDeadlineAt(
                       std::declval<uint64_t>(), std::declval<std::shared_ptr<const ::SCE::HostInvokeDeadline>>(),
                       std::declval<uint64_t>())),
                   decltype(std::declval<S &>().dropHostInvokeDeadline(std::declval<uint64_t>())),
                   decltype(std::declval<const S &>().hostInvokeDeadlineDueAt(std::declval<uint64_t>()))>>
    : std::is_convertible<decltype(std::declval<const S &>().hostInvokeDeadlineDueAt(std::declval<uint64_t>())),
                          std::optional<uint64_t>> {};

/**
 * @brief A host-run invocation that did not start: the engine raises
 *        `error.execution` with `reason` as `_event.data`
 *
 * An invocation that cannot be started is an error of the element, not the
 * `error.invoke` of a process that ran (`HostInvokeResponse::refusal`).
 */
struct HostInvokeNotStarted {
    /// What the host said, or what the engine found, as the event's data.
    std::string reason;
};

/**
 * @brief A host answered a start with its completion: the engine ends start `token`
 *        through the door a later completion takes (`HostInvokeLedger::endRun`)
 *
 * The ledger does not end the run itself, so a completion the host reports now and
 * one it reports later are one operation of the engine, and cannot disagree about
 * whether the invocation is over. No completion the host did not report is made up.
 */
struct HostInvokeCompletedAtOnce {
    /// The `type` the request named.
    std::string processorType;
    /// The invocation's id.
    std::string invokeId;
    /// The start the host completed.
    uint64_t token = 0;
    /// `_event.data` of the completion (`HostInvokeResponse::doneData`).
    std::string doneData;
};

/**
 * @brief The one ending of a start: the engine raises `done.invoke.<invokeId>` for a
 *        completion and `error.invoke.<invokeId>` for a failure, carrying `token`
 *
 * Answered by `HostInvokeLedger::endRun` only for a start that was still running, so
 * a start ends once, by whichever of completion, failure and deadline comes first.
 */
struct HostInvokeEnded {
    /// The invocation's id: the event's `_event.invokeid`, and the end of its name.
    std::string invokeId;
    /// The start that ended, which the event carries so the engine's queue can tell it
    /// from a completion raised any other way.
    uint64_t token = 0;
    /// A failure (`error.invoke`) rather than a completion (`done.invoke`).
    bool failed = false;
    /// `_event.data`.
    std::string data;
    /// `_event.origin`, empty for none.
    std::string origin;
    /// `_event.origintype`, empty for the SCXML processor's.
    std::string originType;

    /// The specific name of the event: `done.invoke.<id>` or `error.invoke.<id>`.
    std::string eventName() const {
        return failed ? std::string(InvokeHelper::ERROR_INVOKE_PREFIX) + invokeId
                      : InvokeHelper::createDoneInvokeEventName(invokeId);
    }
};

/**
 * @brief The deadline of a start came due while it ran: the host was told to stop,
 *        and the engine raises `error.invoke.<invokeId>` with `data()`
 */
struct HostInvokeTimedOut {
    /// The invocation's id.
    std::string invokeId;

    /// The specific name of the event.
    std::string eventName() const {
        return std::string(InvokeHelper::ERROR_INVOKE_PREFIX) + invokeId;
    }

    /// The JSON string `"deadline"`, as every backend carries it: event data is read
    /// as JSON first, so a bare word would reach `_event.data` only through the
    /// fallback.
    static std::string data() {
        return "\"deadline\"";
    }
};

/**
 * @brief What a start or a deadline of the ledger leaves the engine to do
 *
 * `std::monostate` when there is nothing to do: the start runs, or nothing ended.
 */
using HostInvokeReport =
    std::variant<std::monostate, HostInvokeNotStarted, HostInvokeCompletedAtOnce, HostInvokeTimedOut>;

/**
 * @brief The host-run invocations of one session: running, or re-armed by a restore
 *        and waiting to start
 *
 * Every operation is an owner-thread operation of the engine that holds the ledger
 * (ARCHITECTURE.md "Engine Threading"); the ledger has no lock of its own. Each takes
 * what it needs of the engine as an argument (the tree's invokers, the session's id,
 * the scheduler), so the ledger holds no reference into the engine and any engine
 * can hold one.
 */
class HostInvokeLedger {
public:
    /// What a restored host-run invocation has left of its deadline.
    enum class RestoredDeadline {
        None,    ///< it had none
        At,      ///< it comes due at `atMs` of the engine's clock
        Passed,  ///< it came due while the machine was away
    };

    /// A host-run invocation a saved state listed, waiting for the first macrostep the
    /// host drives to start it again. `request` is what the saved machine started it
    /// with, marked `restarted`; `atMs` is the moment of the engine's clock a deadline
    /// in `RestoredDeadline::At` comes due, worked out from the wall-clock moment the
    /// saved state holds.
    struct RestoredHostInvoke {
        ::SCE::HostInvokeRequest request;
        RestoredDeadline deadline = RestoredDeadline::None;
        uint64_t atMs = 0;
    };

    /// A host-run invocation as a save reads it: the request it was started with, and
    /// the moment of the engine's clock its deadline comes due, if it has one.
    struct RunningHostInvoke {
        const ::SCE::HostInvokeRequest *request = nullptr;
        std::optional<uint64_t> dueAtMs;
    };

    /**
     * @brief Start a host-run invocation, reporting what the engine is left to do
     *
     * `std::nullopt` when no invoker is registered for the request's type, which the
     * engine reports as its own (a generated site raises `error.execution`). Otherwise
     * the start is recorded and the invoker asked, and the report says what is left: an
     * error to raise, a completion to end the run with, or nothing.
     *
     * A `_sce_deadline_ms` param is the engine's, not the host's: it is taken out of
     * the request and armed with the start, to come due that many milliseconds after
     * `nowMs()`, the engine's clock, which is read only when the request names a
     * deadline. A value that is not a whole number of milliseconds starts nothing and
     * is reported as `HostInvokeNotStarted`: an argument of the element that cannot be
     * evaluated. A missing invoker is asked about first, because it is the prior fact.
     *
     * @param sessionId The session that starts it, which the request carries to the
     *                  host and the host names again to end it (docs/adr/0045)
     */
    template <typename Deadlines, typename Now>
    std::optional<HostInvokeReport> start(const ::SCE::TreeInvokers &invokers, const ::SCE::HostInvokeRequest &request,
                                          const std::string &sessionId, Deadlines &deadlines, Now &&nowMs) {
        const auto handler = invokers.handlers.find(request.processorType);
        if (handler == invokers.handlers.end()) {
            return std::nullopt;
        }
        ::SCE::HostInvokeRequest started = request;
        std::optional<uint64_t> deadlineMs;
        const auto deadlineParam = started.params.find(std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM));
        if (deadlineParam != started.params.end()) {
            const std::vector<std::string> written = deadlineParam->second;
            started.params.erase(deadlineParam);
            if (written.size() == 1) {
                deadlineMs = ::SCE::parseHostInvokeDeadlineMs(written.front());
            }
            if (!deadlineMs.has_value()) {
                std::string joined;
                for (const auto &value : written) {
                    joined += (joined.empty() ? "" : ",") + value;
                }
                return HostInvokeReport(
                    HostInvokeNotStarted{"<invoke type='" + request.processorType + "' id='" + request.invokeId +
                                         "'>: " + std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM) + "='" + joined +
                                         "' is not a whole number of milliseconds"});
            }
        }
        std::optional<uint64_t> deadlineAtMs;
        if (deadlineMs.has_value()) {
            // Held at the clock's maximum: the largest deadline the grammar
            // admits would otherwise wrap into the past and fire at once.
            const uint64_t now = nowMs();
            deadlineAtMs = *deadlineMs > UINT64_MAX - now ? UINT64_MAX : now + *deadlineMs;
        }
        return startRun(handler->second, std::move(started), sessionId, deadlineAtMs, deadlines);
    }

    /**
     * @brief End start `token` of an invocation, a completion or a failure as `failed`
     *        says, if that start is still running
     *
     * The one body behind every ending a host reports, later or at once: the token
     * check, the deadline drop, and the report of the event to raise. Empty, raising
     * nothing, for a start that was cancelled, already ended, or replaced by a restart
     * (a restarted `<invoke>` is a different process under the same id, and what a
     * cancelled process sends is ignored).
     */
    template <typename Deadlines>
    std::optional<HostInvokeEnded> endRun(const std::string &processorType, const std::string &invokeId, uint64_t token,
                                          bool failed, const std::string &data, const std::string &origin,
                                          const std::string &originType, Deadlines &deadlines) {
        static_assert(HostInvokeDeadlineSchedulerTrait<Deadlines>::value,
                      "a host-invoke ledger's scheduler arms, drops and answers a deadline by token");
        const auto running = started_.find(std::make_pair(processorType, invokeId));
        if (running == started_.end() || running->second.token != token) {
            return std::nullopt;
        }
        started_.erase(running);
        deadlines.dropHostInvokeDeadline(token);
        return HostInvokeEnded{invokeId, token, failed, data, origin, originType};
    }

    /**
     * @brief Stop the invocation of this type and id, whose state has exited
     *
     * One a restore re-armed and has not started again was never started by this
     * engine, so there is nothing to tell the host to stop: it is dropped, so it is
     * never started. A running one is taken out with its deadline dropped, and its
     * invoker is told to stop it.
     *
     * @return whether a cancel was delivered
     */
    template <typename Deadlines>
    bool cancel(const ::SCE::TreeInvokers &invokers, const std::string &processorType, const std::string &invokeId,
                Deadlines &deadlines) {
        static_assert(HostInvokeDeadlineSchedulerTrait<Deadlines>::value,
                      "a host-invoke ledger's scheduler arms, drops and answers a deadline by token");
        restored_.erase(std::remove_if(restored_.begin(), restored_.end(),
                                       [&](const RestoredHostInvoke &restored) {
                                           return restored.request.processorType == processorType &&
                                                  restored.request.invokeId == invokeId;
                                       }),
                        restored_.end());
        const auto running = started_.find(std::make_pair(processorType, invokeId));
        if (running == started_.end()) {
            return false;
        }
        const uint64_t token = running->second.token;
        started_.erase(running);
        deadlines.dropHostInvokeDeadline(token);
        return deliverCancel(invokers, processorType, invokeId, token);
    }

    /**
     * @brief The deadline of a start came due, handed back by the scheduler
     *
     * If that start is still running it ends: the host is told to stop, and the report
     * is `HostInvokeTimedOut`. Through the same table a completion is judged against,
     * so exactly one of the two happens. Nothing to raise when the start is over, or
     * when no invoker is registered any more to be told.
     */
    HostInvokeReport expire(const ::SCE::TreeInvokers &invokers, const ::SCE::HostInvokeDeadline &deadline) {
        const auto running = started_.find(std::make_pair(deadline.processorType, deadline.invokeId));
        if (running == started_.end() || running->second.token != deadline.token) {
            return std::monostate{};
        }
        started_.erase(running);
        if (!deliverCancel(invokers, deadline.processorType, deadline.invokeId, deadline.token)) {
            return std::monostate{};
        }
        return HostInvokeTimedOut{deadline.invokeId};
    }

    /**
     * @brief Start the invocations a restore re-armed, as the macrostep's start would
     *        start ones the document had just entered, and hand `report` what each one
     *        leaves to raise, in order
     *
     * A deadline that came due while the machine was away ends the invocation without a
     * start: the host is not asked to begin what it would be told to stop at once, and
     * the report is `HostInvokeTimedOut`. An invoker that is not registered by now is
     * the same error a document's own start gets. Each report is handed over before the
     * next invocation starts, so what the engine raises for one stands where it would
     * have had that one been the only start.
     */
    template <typename Deadlines, typename Report>
    void startRestored(const ::SCE::TreeInvokers &invokers, const std::string &sessionId, Deadlines &deadlines,
                       Report &&report) {
        if (restored_.empty()) {
            return;
        }
        std::vector<RestoredHostInvoke> restored;
        restored.swap(restored_);
        for (RestoredHostInvoke &host : restored) {
            if (host.deadline == RestoredDeadline::Passed) {
                report(HostInvokeReport(HostInvokeTimedOut{host.request.invokeId}));
                continue;
            }
            const auto handler = invokers.handlers.find(host.request.processorType);
            if (handler == invokers.handlers.end()) {
                report(HostInvokeReport(
                    HostInvokeNotStarted{"<invoke> names an invoker the host declared but never registered"}));
                continue;
            }
            report(startRun(handler->second, std::move(host.request), sessionId,
                            host.deadline == RestoredDeadline::At ? std::optional<uint64_t>(host.atMs) : std::nullopt,
                            deadlines));
        }
    }

    /// Whether a restore re-armed an invocation that has not started again yet: until
    /// it does, the scheduler holds nothing of it, and the engine owes the host a
    /// macrostep before any clock does.
    bool hasRestored() const {
        return !restored_.empty();
    }

    /// Re-arm @p restored, to start at the top of the first macrostep the host drives.
    void rearm(RestoredHostInvoke restored) {
        restored_.push_back(std::move(restored));
    }

    /// Whether @p deadline belongs to a start that is running.
    bool holds(const ::SCE::HostInvokeDeadline &deadline) const {
        const auto started = started_.find(std::make_pair(deadline.processorType, deadline.invokeId));
        return started != started_.end() && started->second.token == deadline.token;
    }

    /**
     * @brief Every invocation that is running, or re-armed by a restore and waiting to
     *        start, as a save reads it, ordered by `(type, id)` so every backend writes
     *        them in the same order
     *
     * A running one's deadline is the moment its pending deadline comes due on
     * `deadlines`; a re-armed one's is the moment it kept, and one that came due while
     * the machine was away comes due now, `nowMs()` of the engine's clock, read for
     * each such invocation. The requests are the ledger's own and stay valid until the
     * ledger next changes.
     */
    template <typename Deadlines, typename Now>
    std::vector<RunningHostInvoke> running(const Deadlines &deadlines, Now &&nowMs) const {
        static_assert(HostInvokeDeadlineSchedulerTrait<Deadlines>::value,
                      "a host-invoke ledger's scheduler arms, drops and answers a deadline by token");
        std::vector<RunningHostInvoke> listed;
        for (const auto &entry : started_) {
            const StartedHostInvoke &started = entry.second;
            listed.push_back(RunningHostInvoke{&started.request, deadlines.hostInvokeDeadlineDueAt(started.token)});
        }
        for (const auto &restored : restored_) {
            std::optional<uint64_t> dueAt;
            if (restored.deadline == RestoredDeadline::At) {
                dueAt = restored.atMs;
            } else if (restored.deadline == RestoredDeadline::Passed) {
                dueAt = nowMs();
            }
            listed.push_back(RunningHostInvoke{&restored.request, dueAt});
        }
        std::stable_sort(listed.begin(), listed.end(), [](const RunningHostInvoke &a, const RunningHostInvoke &b) {
            return std::tie(a.request->processorType, a.request->invokeId) <
                   std::tie(b.request->processorType, b.request->invokeId);
        });
        return listed;
    }

    /// The token the next start receives: a save writes it, so a restored machine's
    /// next start is told apart from every start before the save.
    uint64_t nextToken() const {
        return nextToken_;
    }

    /// Continue the tokens from @p next, the next token a saved state holds.
    void continueTokensFrom(uint64_t next) {
        nextToken_ = next;
    }

private:
    using Key = std::pair<std::string, std::string>;

    /// A host-run invocation that was started and has neither ended nor been
    /// cancelled: its start's token and the request the host was handed, which a
    /// save writes.
    struct StartedHostInvoke {
        uint64_t token = 0;
        ::SCE::HostInvokeRequest request;
    };

    /**
     * @brief The start itself, once the invoker is known and the deadline, if there is
     *        one, is the moment of the engine's clock it comes due
     *
     * The token is assigned and the run recorded before the handler is asked, so a
     * handler that completes at once, whether by returning `doneData` or by ending it
     * through the engine before it returns, completes a run the ledger already knows.
     * A completion the host reports now is handed back (`HostInvokeCompletedAtOnce`) for
     * the engine to end through the door a later one takes. A restore starts an
     * invocation again through here, with the deadline it had left.
     */
    template <typename Deadlines>
    HostInvokeReport startRun(const ::SCE::HostInvokeHandler &handler, ::SCE::HostInvokeRequest request,
                              const std::string &sessionId, std::optional<uint64_t> deadlineAtMs,
                              Deadlines &deadlines) {
        static_assert(HostInvokeDeadlineSchedulerTrait<Deadlines>::value,
                      "a host-invoke ledger's scheduler arms, drops and answers a deadline by token");
        // §scxml-6.4.1: the type is one the platform serves, through the invoker the
        // host registered for it.
        const uint64_t token = nextToken_++;
        request.token = token;
        // docs/adr/0045: the session that starts it, which the host may serve or refuse
        // it by, and names again to end it.
        request.sessionId = sessionId;
        const Key key = std::make_pair(request.processorType, request.invokeId);
        // What a save writes is the request as the host was handed it, so the run keeps
        // its own copy of what the host may go on to read.
        started_[key] = StartedHostInvoke{token, request};
        if (deadlineAtMs.has_value()) {
            deadlines.scheduleHostInvokeDeadlineAt(token,
                                                   std::make_shared<const ::SCE::HostInvokeDeadline>(
                                                       ::SCE::HostInvokeDeadline{key.first, key.second, token}),
                                                   *deadlineAtMs);
        }
        ::SCE::HostInvokeEvent event;
        event.start = std::move(request);
        const auto response = handler(event);
        if (response.has_value() && response->refusal.has_value()) {
            // The host could not start it, so it never started: taken back out of the
            // running set with its deadline dropped, and the element's error reported
            // with what the host said.
            started_.erase(key);
            deadlines.dropHostInvokeDeadline(token);
            return HostInvokeNotStarted{*response->refusal};
        }
        if (response.has_value() && response->doneData.has_value()) {
            return HostInvokeCompletedAtOnce{key.first, key.second, token, *response->doneData};
        }
        return std::monostate{};
    }

    /// Tell the invoker of @p processorType to stop start @p token of @p invokeId;
    /// whether one was registered to be told.
    static bool deliverCancel(const ::SCE::TreeInvokers &invokers, const std::string &processorType,
                              const std::string &invokeId, uint64_t token) {
        const auto it = invokers.handlers.find(processorType);
        if (it == invokers.handlers.end()) {
            return false;
        }
        ::SCE::HostInvokeEvent event;
        event.cancel = ::SCE::HostInvokeCancel{processorType, invokeId, token};
        (void)it->second(event);
        return true;
    }

    // §scxml-6.4: every host-run invocation started and neither ended nor cancelled,
    // keyed by `(type, id)`, with its start's token and the request, so an engine's exit chain
    // can be an unconditional cancel and an ending can be judged against the run it
    // names. One entry per key: an `<invoke>` belongs to one state and a state is in
    // the configuration at most once, so the same id is never running twice; a
    // restart replaces the entry, and its new token is what makes the first run's
    // reply stale.
    std::map<Key, StartedHostInvoke> started_;
    // The token the next start receives.
    uint64_t nextToken_ = 0;
    // docs/adr/0045: the invocations a restore re-armed, which start at the top of the
    // first macrostep the host drives (`startRestored`), and until then are neither
    // running nor gone.
    std::vector<RestoredHostInvoke> restored_;
};

}  // namespace SCE::Core
