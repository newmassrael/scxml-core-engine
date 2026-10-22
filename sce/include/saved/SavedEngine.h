// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Saving a `datamodel="sce-static"` machine and restoring it (SCE Accepted Subset §2.15,
// docs/adr/0045): the steps every generated `saveAt` and `restoreAt` take alike. The generated
// machine adds only its own variables, its histories and its running children between them. The
// C++ twin of `sce.SaveSession`, `sce.JudgeSession` and `sce.EnterSession` (Go,
// `engine_saved.go`), of `saved_engine.py` (Python), of the saved-state half of
// `sce_rust_runtime::saved_state` (Rust) and of Kotlin's `StateMachineEngine`.
//
// What a generated policy of a document that saves adds to what the engine asks of every policy
// is what only the document knows about the sessions and invocations a saved state names, and the
// tables of its `<history>` elements:
//
//   sessionIdText()                    the id of the session, which a saved state names it by
//   runningInvokes()                   the id the document gives each `<invoke>` whose child
//                                      session is running, in document order (§scxml-6.4)
//   invokeStarts()                     how many times each `<invoke>` has started in this
//                                      session, for each started at least once, in document order
//   invokeOwner(id)                    the state that holds the `<invoke>` `id`, nullopt for an id
//                                      the document does not invoke
//   hostInvokeOwner(type, id)          the state that holds the host-run `<invoke>` of this type
//                                      and id, nullopt for one the document does not have a host
//                                      invoker run (§scxml-6.4.1)
//   judgeChild(child)                  refuses `child`, a running child a format-2 state lists,
//                                      unless its session is a state of the document the child
//                                      names, to any depth (docs/adr/0045, section 3, step 1),
//                                      building nothing
//   resumeChildren(engine, children,   resumes each of `children` on `engine`, in the order given,
//                  wallNowMs)          each built as starting it builds it and restored where it
//                                      stood in place of initialized: no `<onentry>`, no `<data>`,
//                                      no `<param>` or namelist evaluated, no `idlocation`
//                                      written, nothing sent
//   restartInvokes(engine, ids)        starts each of `ids` again from its beginning, as entering
//                                      its state starts it, which is what a format-1 state asks
//                                      of a child it names by id alone
//   historyIds()                       the ids of the document's `<history>` elements, by id
//   historyFromId(id)                  the history of that id, nullopt for none
//   historyIsDeep(history)             whether it is a deep history
//   setHistoryValue(history, states)   what it recorded, as a restore sets it

#pragma once

#include "core/ConfigurationHelper.h"
#include "core/EventNameResolver.h"
#include "core/HostProcessor.h"
#include "saved/SavedState.h"
#include "static/StaticExecutionEngine.h"

#include <algorithm>
#include <cstdint>
#include <limits>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

namespace SCE::Saved {

namespace detail {

inline constexpr std::int64_t kInt64Max = std::numeric_limits<std::int64_t>::max();

/// `base` and `add` added, held to what a signed 64-bit count can say.
inline std::int64_t saturatingAddMs(std::int64_t base, std::uint64_t add) {
    if (add > static_cast<std::uint64_t>(kInt64Max - base)) {
        return kInt64Max;
    }
    return base + static_cast<std::int64_t>(add);
}

/// The wall-clock moment, in milliseconds since the Unix epoch, a thing that comes due at
/// `readyAtMs` of the engine's clock comes due at, given that `wallNowMs` is what time it is on
/// the wall now and `nowMs` on the engine's clock: none earlier than now, for one already past.
inline std::int64_t dueOn(std::int64_t wallNowMs, std::uint64_t nowMs, std::uint64_t readyAtMs) {
    return saturatingAddMs(wallNowMs, readyAtMs > nowMs ? readyAtMs - nowMs : 0);
}

/// The moment of the engine's clock a thing saved to come due at the wall-clock moment `due`
/// comes due at, given the wall clock reads `wallNowMs` now and the engine's `nowMs`: now, for
/// one that came due while the machine was away.
inline std::uint64_t readyAt(std::uint64_t nowMs, std::int64_t due, std::int64_t wallNowMs) {
    const std::uint64_t wait =
        due > wallNowMs ? static_cast<std::uint64_t>(due) - static_cast<std::uint64_t>(wallNowMs) : 0;
    const std::uint64_t cap = static_cast<std::uint64_t>(kInt64Max);
    return wait > cap - std::min(nowMs, cap) ? cap : nowMs + wait;
}

inline SavedParams cloneParams(const std::map<std::string, std::vector<std::string>> &params) {
    return SavedParams(params.begin(), params.end());
}

}  // namespace detail

// ── History ──────────────────────────────────────────────────────────────────

/// What each history of `policy`'s document has recorded, as a saved state holds it: state ids in
/// document order (the order one machine writes is every backend's), ordered by the history's id.
/// A history that has recorded nothing is left out.
template <typename Policy> std::vector<SavedHistory> saveHistory(const Policy &policy) {
    std::vector<SavedHistory> saved;
    for (const std::string &id : Policy::historyIds()) {
        const auto history = Policy::historyFromId(id);
        if (!history.has_value()) {
            continue;
        }
        auto recorded = policy.historyValue(*history);
        if (!recorded.has_value()) {
            continue;
        }
        std::sort(recorded->begin(), recorded->end(), [](const auto &a, const auto &b) {
            return Policy::getDocumentOrder(a) < Policy::getDocumentOrder(b);
        });
        SavedHistory entry;
        entry.id = id;
        for (const auto &state : *recorded) {
            entry.states.emplace_back(Policy::getStateName(state));
        }
        saved.push_back(std::move(entry));
    }
    std::sort(saved.begin(), saved.end(), [](const SavedHistory &a, const SavedHistory &b) { return a.id < b.id; });
    return saved;
}

namespace detail {

/// A history refusal in the document's own vocabulary.
template <typename Policy> std::string describeHistory(const Core::HistoryRejection<typename Policy::State> &verdict) {
    const auto name = [](const std::optional<typename Policy::State> &state) {
        return std::string(Policy::getStateName(*state));
    };
    switch (verdict.kind) {
    case Core::HistoryRejectionKind::Empty:
        return "no state, and a history records at least one";
    case Core::HistoryRejectionKind::Duplicate:
        return "'" + name(verdict.state) + "' twice";
    case Core::HistoryRejectionKind::NotBelow:
        return "'" + name(verdict.state) + "', which is not below its parent '" + name(verdict.parent) + "'";
    case Core::HistoryRejectionKind::NotAChild:
        return "'" + name(verdict.state) + "', which is not a child of '" + name(verdict.parent) +
               "' (a shallow history records the children)";
    case Core::HistoryRejectionKind::NotAtomic:
        return "'" + name(verdict.state) + "', which has children (a deep history records atomic states)";
    case Core::HistoryRejectionKind::Arity:
        return std::string("states of no configuration: ") + Core::configurationRejectionText(verdict.arity);
    case Core::HistoryRejectionKind::Accepted:
        break;
    }
    return "a value no history records";
}

}  // namespace detail

/// What `saved` records for each history of `Policy`'s document, read as the states of this
/// document, or the refusal that says which value is not one.
///
/// Every value is judged before any is applied, so a state with one bad history restores none of
/// them. A history the document does not declare, one named twice, and a value its history could
/// not have recorded (a state outside its parent, a compound state with two active children) are
/// refused: a resumed machine entered through a history must land in a configuration its document
/// could have been in.
template <typename Policy>
std::vector<std::pair<typename Policy::History, std::vector<typename Policy::State>>>
restoreHistory(const SavedState &saved) {
    using State = typename Policy::State;
    std::vector<std::pair<typename Policy::History, std::vector<State>>> restored;
    for (std::size_t at = 0; at < saved.history.size(); ++at) {
        const SavedHistory &recorded = saved.history[at];
        for (std::size_t earlier = 0; earlier < at; ++earlier) {
            if (saved.history[earlier].id == recorded.id) {
                throw StateRefusal("the saved state records the history '" + recorded.id + "' twice");
            }
        }
        const auto history = Policy::historyFromId(recorded.id);
        if (!history.has_value()) {
            throw StateRefusal("the saved state records the history '" + recorded.id +
                               "', which the document does not declare");
        }
        std::vector<State> states;
        for (const std::string &name : recorded.states) {
            const auto state = Policy::getStateFromName(name.c_str());
            if (!state.has_value()) {
                throw StateRefusal("the history '" + recorded.id + "' records '" + name +
                                   "', which the document does not name");
            }
            states.push_back(*state);
        }
        const auto verdict =
            Core::validateHistory<Policy>(Policy::getHistoryParent(*history), Policy::historyIsDeep(*history), states);
        if (verdict.kind != Core::HistoryRejectionKind::Accepted) {
            throw StateRefusal("the history '" + recorded.id + "' records " + detail::describeHistory<Policy>(verdict));
        }
        restored.emplace_back(*history, std::move(states));
    }
    return restored;
}

// ── The saved-state door of an engine ────────────────────────────────────────

/// A waiting send read against the document: the event it names, resolved, so a restore that
/// cannot resolve one refuses before it arms any.
template <typename Event> struct ReadSend {
    std::int64_t due = 0;
    SavedAct act;
    std::optional<Event> event;
};

/// A saved session read against the document of a policy, before anything is built: its
/// configuration and current leaf, its external queue, its waiting sends and, for a format-1
/// state, the `<invoke>` ids to start again.
template <typename Engine> struct JudgedSession {
    std::vector<typename Engine::State> configuration;
    typename Engine::State current{};
    std::vector<typename Engine::EventWithMetadata> external;
    std::vector<ReadSend<typename Engine::Event>> pending;
    std::vector<std::string> restarts;
};

/// What a saved state asks of an engine. A friend of the engine, which keeps its queues, its
/// scheduler and its running host invocations to itself: a save reads them and a restore stands
/// them where they were, and nothing else should be able to.
struct EngineAccess {
    /// `engine`'s saved state, a machine of the document whose saved state is bound to `shape`,
    /// with the `variables`, the `history` and the running `children` its generated `saveAt` read.
    /// The configuration is written in document order, and every waiting delayed `<send>` is
    /// written as the moment it comes due on the wall clock whose reading now is `wallNowMs`.
    ///
    /// A machine any of whose record schemas or enums is past its first revision names each such
    /// revision in `revisions` and writes the revised shape; any other writes the first shape and
    /// no `revisions`. The state is format 2: the machine's own id, how many times each `<invoke>`
    /// has started, and children, each running child with its whole state, which the generated
    /// machine saved through the child's own document, in document order.
    ///
    /// Refused for a machine that is not running (never started, or ended at a top-level
    /// `<final>`, where there is nothing left to resume), for one whose last macrostep stopped at
    /// the microstep ceiling (its configuration is not a settled one, and a restore would resume it
    /// as if it were), for one with no session id, and when `children` are not the children the
    /// machine is running.
    template <typename Engine>
    static SavedState saveSession(Engine &engine, const SavedShape &shape, JsonObject variables,
                                  std::vector<SavedHistory> history, std::vector<SavedChild> children,
                                  std::int64_t wallNowMs) {
        using Policy = typename Engine::StatePolicyType;
        Policy &policy = engine.policy_;
        if (!engine.isRunning_) {
            throw StateRefusal("the machine is not running: it was never started, or it has ended");
        }
        if (engine.macrostepTruncated_) {
            throw StateRefusal("the machine's last macrostep stopped at the microstep ceiling, so it does not "
                               "stand at a settled configuration");
        }
        const std::string sessionId = policy.sessionIdText();
        if (sessionId.empty()) {
            throw StateRefusal("the machine has no session id, which a saved state names it by");
        }
        // What the generated machine saved is every child it runs, and nothing else: one left
        // out would restore a parent whose child is gone.
        const std::vector<std::string> running = policy.runningInvokes();
        std::vector<std::string> savedIds;
        for (const SavedChild &child : children) {
            savedIds.push_back(child.id);
        }
        if (running != savedIds) {
            throw StateRefusal("the machine runs the children " + quoted(running) + " and the save holds " +
                               quoted(savedIds) + ", a defect of its generated save");
        }

        SavedState state;
        // Read before anything is built, so a delayed send this state cannot carry refuses the
        // save instead of leaving it out.
        state.pending = savePending(engine, sessionId, wallNowMs);
        // Document order, so the same machine saves the same text on every backend.
        std::vector<typename Engine::State> active = engine.getActiveStates();
        std::sort(active.begin(), active.end(), [](const auto &a, const auto &b) {
            return Policy::getDocumentOrder(a) < Policy::getDocumentOrder(b);
        });
        for (const auto &member : active) {
            state.configuration.emplace_back(Policy::getStateName(member));
        }
        state.current = Policy::getStateName(engine.currentState_);
        // The revision of each record schema and each enum past its first, whose digest list is
        // empty exactly when it is in its first revision: the record schemas first, then the
        // enums, each in the order the generator gave them.
        for (const RecordLineage &record : shape.records) {
            if (!record.revisions.empty()) {
                state.revisions.push_back(SavedRevision{record.event, record.revision, record.revisions.back()});
            }
        }
        for (const EnumLineage &lineage : shape.enums) {
            if (!lineage.revisions.empty()) {
                state.revisions.push_back(SavedRevision{lineage.key, lineage.revision, lineage.revisions.back()});
            }
        }
        state.shape = state.revisions.empty() ? shape.format2First : shape.format2Revised;
        state.variables = std::move(variables);
        state.history = std::move(history);
        state.invokes.resumed = true;
        state.invokes.children = std::move(children);
        state.sessionId = sessionId;
        state.invokeStarts = policy.invokeStarts();
        state.hostInvokes = saveHostInvokes(engine, wallNowMs);
        state.hostInvokeToken = static_cast<std::int64_t>(engine.nextHostInvokeToken_);
        state.autoSendSeq = static_cast<std::int64_t>(engine.autoSendSeq_);
        for (const auto &queued : engine.externalQueue_.snapshot()) {
            // §scxml-5.10: a queued event is saved under the name it ARRIVED under, so a restore
            // gives the machine the same `_event.name`.
            SavedEvent event;
            event.name = queued.name.empty() ? std::string(Policy::getEventName(queued.event)) : queued.name;
            event.data = queued.data;
            // An event nothing stamped a type on was taken off this queue, which is external.
            event.type = queued.type.empty() ? "external" : queued.type;
            event.sendId = queued.sendId;
            event.origin = queued.origin;
            event.originType = queued.originType;
            event.invokeId = queued.invokeId;
            if (queued.hostInvokeToken.has_value()) {
                event.hostInvokeToken = static_cast<std::int64_t>(*queued.hostInvokeToken);
            }
            state.external.push_back(std::move(event));
        }
        return state;
    }

    /// Refuse `saved` unless it can be a state of the document of `Engine`'s policy, building
    /// nothing (docs/adr/0045, section 3, step 1): its configuration, its queue, its waiting sends,
    /// its running and host-run invocations, its count of starts, and each running child's session
    /// against the document that child runs (`Policy::judgeChild`), to any depth.
    ///
    /// The shape, the revisions, the history and the variables are the generated machine's to judge
    /// before this (`upgradeSavedState`, `restoreHistory`); a generated `judge` does both, and is
    /// what a parent's judge of a tree calls for each child it holds.
    template <typename Engine> static void judgeSession(const SavedState &saved) {
        static_cast<void>(judge<Engine>(saved));
    }

    /// Stand `engine`, a machine of the policy it holds whose variables already hold what `saved`
    /// holds, where `saved` names: the machine a host drives in place of one it would
    /// `initialize`.
    ///
    /// No `<onentry>` runs and no `<data>` is evaluated (`enterAt`): the run being resumed already
    /// did both, and the host calls it caused cannot be made twice. A configuration that is not one
    /// of this document is refused, and nothing is entered.
    ///
    /// The engine's session id is the one `saved` names, a format-2 state's own, which every child's
    /// id and a peer's address name (docs/adr/0045). `wallNowMs` is what time it is on the wall
    /// clock the saved `due`s were written against: a waiting send is armed to come due `due -
    /// wallNowMs` after now, and one already due is armed to come due now, behind the ones due
    /// before it.
    ///
    /// A format-1 state's running children are started again from their beginning, last, so what a
    /// child sends as it starts stands behind what was already waiting. A format-2 state's are
    /// judged with the rest before anything is built, and resumed, each where it stood, once this
    /// session stands where it did (`Policy::resumeChildren`).
    template <typename Engine>
    static void enterSession(Engine &engine, const SavedState &saved, std::int64_t wallNowMs) {
        auto judged = judge<Engine>(saved);
        const auto verdict = engine.enterAt(judged.configuration, judged.current);
        if (verdict != Core::ConfigurationRejection::None) {
            throw StateRefusal(std::string("the saved configuration is refused: ") +
                               Core::configurationRejectionText(verdict));
        }
        for (auto &queued : judged.external) {
            engine.externalQueue_.raise(queued);
        }
        armPending(engine, judged.pending, wallNowMs);
        rearmHostInvokes(engine, saved.hostInvokes, wallNowMs);
        engine.nextHostInvokeToken_ = static_cast<std::uint64_t>(saved.hostInvokeToken);
        engine.autoSendSeq_ = static_cast<std::uint64_t>(saved.autoSendSeq);
        if (saved.invokes.resumed) {
            engine.policy_.resumeChildren(engine, saved.invokes.children, wallNowMs);
        } else {
            // Last, so what a child sends as it starts stands behind what was already waiting:
            // the saved machine's queue was ahead of it.
            engine.policy_.restartInvokes(engine, judged.restarts);
        }
    }

private:
    static std::string quoted(const std::vector<std::string> &ids) {
        std::string out = "[";
        for (std::size_t at = 0; at < ids.size(); ++at) {
            out += (at == 0 ? "'" : ", '") + ids[at] + "'";
        }
        return out + "]";
    }

    /// The host-run invocations that are running, or re-armed and waiting to start, for a save:
    /// each with the request it was started with and the moment of the engine's clock its deadline
    /// comes due, if it has one. Ordered by `(type, id)`, so every backend writes them in the same
    /// order.
    template <typename Engine>
    static std::vector<std::pair<const ::SCE::HostInvokeRequest *, std::optional<std::uint64_t>>>
    runningHostInvokes(const Engine &engine) {
        const auto pending = engine.scheduler_.pending();
        std::vector<std::pair<const ::SCE::HostInvokeRequest *, std::optional<std::uint64_t>>> running;
        for (const auto &[key, started] : engine.startedHostInvokes_) {
            std::optional<std::uint64_t> dueAt;
            for (const auto &entry : pending) {
                if (entry.hostInvokeDeadline && entry.hostInvokeDeadlineToken == started.token) {
                    dueAt = entry.fireTime;
                    break;
                }
            }
            running.emplace_back(&started.request, dueAt);
        }
        for (const auto &restored : engine.restoredHostInvokes_) {
            std::optional<std::uint64_t> dueAt;
            if (restored.deadline == Engine::RestoredDeadline::At) {
                dueAt = restored.atMs;
            } else if (restored.deadline == Engine::RestoredDeadline::Passed) {
                dueAt = engine.nowMs();
            }
            running.emplace_back(&restored.request, dueAt);
        }
        std::stable_sort(running.begin(), running.end(), [](const auto &a, const auto &b) {
            return std::tie(a.first->processorType, a.first->invokeId) <
                   std::tie(b.first->processorType, b.first->invokeId);
        });
        return running;
    }

    /// Every host-run invocation of `engine` that is running, or was re-armed by a restore and
    /// has yet to start, as a saved state holds it: the request it was started with and the
    /// wall-clock moment its deadline comes due.
    template <typename Engine>
    static std::vector<SavedHostInvoke> saveHostInvokes(const Engine &engine, std::int64_t wallNowMs) {
        const std::uint64_t now = engine.nowMs();
        std::vector<SavedHostInvoke> hosts;
        for (const auto &[request, dueAt] : runningHostInvokes(engine)) {
            SavedHostInvoke host;
            host.processorType = request->processorType;
            host.invokeId = request->invokeId;
            host.src = request->src;
            host.params = detail::cloneParams(request->params);
            host.data = request->eventData;
            host.content = request->content;
            if (dueAt.has_value()) {
                host.due = detail::dueOn(wallNowMs, now, *dueAt);
            }
            hosts.push_back(std::move(host));
        }
        return hosts;
    }

    /// Every delayed send of `engine` still waiting, as a saved state holds it: written as the
    /// wall-clock moment it comes due, in the order the machine would deliver them.
    ///
    /// A send to the parent or to a child of this session is held as the act that delivers it to
    /// the session the restored tree holds there (docs/adr/0045). A send the format cannot carry,
    /// one routed to a session by its id or to a Mesh peer, refuses the save. A document that makes
    /// one is generated without the save API, so no generated machine reaches this; the refusal is
    /// for a machine built by hand, where leaving the entry out would restore a machine that never
    /// delivers it and say nothing.
    ///
    /// The deadline of a running host-run invocation is not a send: it is saved with the
    /// invocation, as its `due`, and left out here. One that belongs to no running invocation is
    /// refused like a send the format cannot carry.
    ///
    /// The act of a waiting send names the session that sent it as its origin, as every
    /// backend's does, so what is saved is that session's id.
    template <typename Engine>
    static std::vector<SavedSend> savePending(const Engine &engine, const std::string &sessionId,
                                              std::int64_t wallNowMs) {
        using Policy = typename Engine::StatePolicyType;
        using Kind = ::SCE::ScheduledRoute::Kind;
        const std::uint64_t now = engine.nowMs();
        std::vector<SavedSend> sends;
        for (const auto &entry : engine.scheduler_.pending()) {
            if (entry.hostInvokeDeadline) {
                const auto &deadline = *entry.hostInvokeDeadline;
                const auto started =
                    engine.startedHostInvokes_.find(std::make_pair(deadline.processorType, deadline.invokeId));
                if (started != engine.startedHostInvokes_.end() && started->second.token == deadline.token) {
                    continue;
                }
                throw StateRefusal("the deadline of the invocation '" + deadline.invokeId +
                                   "' belongs to no invocation this machine is running");
            }
            SavedSend send;
            send.due = detail::dueOn(wallNowMs, now, entry.fireTime);
            SavedAct &act = send.act;
            act.sendId = entry.sendId;
            if (entry.hostSend) {
                const ::SCE::HostSendRequest &request = *entry.hostSend;
                act.kind = SavedActKind::Host;
                act.processorType = request.processorType;
                act.event = request.eventName;
                act.target = request.target;
                act.content = request.content;
                act.params = detail::cloneParams(request.params);
                act.sendId = request.sendId;
                act.data = request.eventData;
                act.invokeId = request.invokeId;
            } else if (!entry.route) {
                // §scxml-5.10: a send is saved under the name it was sent under, which is its
                // member's own unless a computed one cut back to a longer name.
                act.kind = SavedActKind::Raise;
                act.event = Policy::getEventName(entry.event);
                act.data = entry.eventData;
                act.origin = sessionId;
            } else if (entry.route->kind == Kind::ExternalQueue) {
                act.kind = SavedActKind::Raise;
                act.event = entry.route->eventName;
                act.data = entry.eventData;
                act.origin = sessionId;
            } else if (entry.route->kind == Kind::InternalQueue) {
                act.kind = SavedActKind::Internal;
                act.event = entry.route->eventName.empty() ? std::string(Policy::getEventName(entry.event))
                                                           : entry.route->eventName;
                act.data = entry.eventData;
                act.origin = sessionId;
            } else if (entry.route->kind == Kind::Parent) {
                // docs/adr/0045: the parent and a child of this session are sessions the saved
                // tree holds.
                act.kind = SavedActKind::Parent;
                act.event = entry.route->eventName;
                act.data = entry.eventData;
                act.origin = sessionId;
            } else if (entry.route->kind == Kind::Invocation) {
                act.kind = SavedActKind::Child;
                act.invokeId = entry.route->address;
                act.event = entry.route->eventName;
                act.data = entry.eventData;
                act.origin = sessionId;
            } else {
                throw StateRefusal("the delayed send '" + entry.sendId + "' is routed to " +
                                   (entry.route->kind == Kind::MeshPeer ? "the Mesh peer '" : "the session '") +
                                   entry.route->address + "', which a saved state does not carry");
            }
            sends.push_back(std::move(send));
        }
        return sends;
    }

    /// The event `name` is, in the document of `Policy`, or its refusal-less absence.
    template <typename Policy> static std::optional<typename Policy::Event> resolveEvent(const std::string &name) {
        return ::SCE::Core::resolveArrivingEventName(
            name, [](const std::string &exact) { return Policy::getEventFromName(exact); });
    }

    template <typename Engine> static JudgedSession<Engine> judge(const SavedState &saved) {
        using Policy = typename Engine::StatePolicyType;
        JudgedSession<Engine> judged;
        for (const std::string &name : saved.configuration) {
            const auto state = Policy::getStateFromName(name.c_str());
            if (!state.has_value()) {
                throw StateRefusal("the document has no state '" + name + "'");
            }
            judged.configuration.push_back(*state);
        }
        const auto current = Policy::getStateFromName(saved.current.c_str());
        if (!current.has_value()) {
            throw StateRefusal("the document has no state '" + saved.current + "'");
        }
        judged.current = *current;
        const auto verdict = Core::validateConfiguration<Policy>(judged.configuration, *current);
        if (verdict != Core::ConfigurationRejection::None) {
            throw StateRefusal(std::string("the saved configuration is refused: ") +
                               Core::configurationRejectionText(verdict));
        }
        const auto holds = [&judged](typename Engine::State state) {
            return std::find(judged.configuration.begin(), judged.configuration.end(), state) !=
                   judged.configuration.end();
        };
        // Every queued event is read before anything is entered, so one this document cannot name
        // refuses the restore rather than half of the queue.
        for (std::size_t at = 0; at < saved.external.size(); ++at) {
            const SavedEvent &event = saved.external[at];
            // The name a queued event arrived under (§scxml-5.10): the document's own event for
            // it, or the one it extends (§scxml-3.12.1).
            const auto resolved = resolveEvent<Policy>(event.name);
            if (!resolved.has_value()) {
                throw StateRefusal("external[" + std::to_string(at) + "] is '" + event.name +
                                   "', which the document does not name");
            }
            if (event.type != "internal" && event.type != "external" && event.type != "platform") {
                throw StateRefusal("external[" + std::to_string(at) + "] has the type '" + event.type +
                                   "', which is not an event type");
            }
            typename Engine::EventWithMetadata queued(*resolved, event.data, event.origin, event.sendId, event.type,
                                                      event.originType, event.invokeId);
            queued.name = event.name == Policy::getEventName(*resolved) ? std::string() : event.name;
            if (event.hostInvokeToken.has_value()) {
                queued.hostInvokeToken = static_cast<std::uint64_t>(*event.hostInvokeToken);
            }
            judged.external.push_back(std::move(queued));
        }
        // The same for a waiting send: an event this document does not name refuses the restore,
        // not the sends before it.
        for (std::size_t at = 0; at < saved.pending.size(); ++at) {
            judged.pending.push_back(readPendingSend<Policy>(saved.pending[at], at));
        }
        // The same for a running invocation: one the document does not have, one whose state the
        // saved configuration does not stand in, or one named twice would be a child the saved
        // machine could not have had.
        const std::vector<std::string> ids = saved.invokes.invokeIds();
        for (std::size_t at = 0; at < ids.size(); ++at) {
            const auto owner = Policy::invokeOwner(ids[at]);
            if (!owner.has_value()) {
                throw StateRefusal("invokes[" + std::to_string(at) + "] is '" + ids[at] +
                                   "', which the document does not invoke");
            }
            if (!holds(*owner)) {
                throw StateRefusal("invokes[" + std::to_string(at) + "] is '" + ids[at] +
                                   "', whose state the saved configuration does not stand in");
            }
            if (std::find(ids.begin(), ids.begin() + static_cast<std::ptrdiff_t>(at), ids[at]) !=
                ids.begin() + static_cast<std::ptrdiff_t>(at)) {
                throw StateRefusal("invokes[" + std::to_string(at) + "] is '" + ids[at] +
                                   "', which an earlier entry already names");
            }
        }
        // A count of starts names an `<invoke>` of the document.
        for (const SavedInvokeStart &started : saved.invokeStarts) {
            if (!Policy::invokeOwner(started.id).has_value()) {
                throw StateRefusal("invokestarts counts '" + started.id + "', which the document does not invoke");
            }
        }
        if (saved.invokes.resumed) {
            for (std::size_t at = 0; at < saved.invokes.children.size(); ++at) {
                judgeChildEntry<Policy>(saved, saved.invokes.children[at], at);
            }
        } else {
            judged.restarts = saved.invokes.ids;
        }
        // And for a host-run one, by the `(type, id)` the document declares: one it does not have,
        // one whose state the saved configuration does not stand in, or one named twice is a run
        // the saved machine could not have had.
        for (std::size_t at = 0; at < saved.hostInvokes.size(); ++at) {
            const SavedHostInvoke &host = saved.hostInvokes[at];
            const auto owner = Policy::hostInvokeOwner(host.processorType, host.invokeId);
            if (!owner.has_value()) {
                throw StateRefusal("hostinvokes[" + std::to_string(at) + "] is '" + host.invokeId + "' of type '" +
                                   host.processorType + "', which the document does not have a host invoker run");
            }
            if (!holds(*owner)) {
                throw StateRefusal("hostinvokes[" + std::to_string(at) + "] is '" + host.invokeId +
                                   "', whose state the saved configuration does not stand in");
            }
            for (std::size_t earlier = 0; earlier < at; ++earlier) {
                if (saved.hostInvokes[earlier].processorType == host.processorType &&
                    saved.hostInvokes[earlier].invokeId == host.invokeId) {
                    throw StateRefusal("hostinvokes[" + std::to_string(at) + "] is '" + host.invokeId +
                                       "', which an earlier entry already names");
                }
            }
        }
        return judged;
    }

    /// Refuse `child`, the `index`-th running child of the format-2 session `parent`, unless it is
    /// the child that session could be running (docs/adr/0045, section 3, step 1): its start is the
    /// count the session holds for its `<invoke>`, its session id is the session's and the id it
    /// was started under, and its own session is a state of the document it names
    /// (`Policy::judgeChild`).
    template <typename Policy>
    static void judgeChildEntry(const SavedState &parent, const SavedChild &child, std::size_t index) {
        const std::string what = "invokes[" + std::to_string(index) + "]";
        const std::int64_t count = parent.invokeStart(child.id);
        if (child.start != count) {
            throw StateRefusal(what + " is start " + std::to_string(child.start) + " of '" + child.id +
                               "', and the session counts " + std::to_string(count) +
                               " starts of it: a state re-entered cancels the earlier child before it starts the next "
                               "one");
        }
        const std::string derived = parent.sessionId + "." + child.invokeId;
        if (child.session->sessionId != derived) {
            throw StateRefusal(what + ".session's id is '" + child.session->sessionId + "', and a child of '" +
                               parent.sessionId + "' started under '" + child.invokeId + "' is '" + derived + "'");
        }
        try {
            Policy::judgeChild(child);
        } catch (const StateRefusal &err) {
            throw refusalWithin(err, what + ".session");
        }
    }

    /// `send`, the `index`-th of a saved state's `pending`, with its event looked up in the
    /// document of `Policy`.
    template <typename Policy>
    static ReadSend<typename Policy::Event> readPendingSend(const SavedSend &send, std::size_t index) {
        ReadSend<typename Policy::Event> read;
        read.due = send.due;
        read.act = send.act;
        const SavedActKind kind = send.act.kind;
        if (kind == SavedActKind::Raise || kind == SavedActKind::Internal) {
            read.event = resolveEvent<Policy>(send.act.event);
            if (!read.event.has_value()) {
                throw StateRefusal("pending[" + std::to_string(index) + "] is '" + send.act.event +
                                   "', which the document does not name");
            }
        } else if (kind == SavedActKind::Child) {
            // The child is named by the `<invoke>` the target names, which must be one of the
            // document's: a send to any other is one the save would have been refused for.
            if (!Policy::invokeOwner(send.act.invokeId).has_value()) {
                throw StateRefusal("pending[" + std::to_string(index) + "] is a send to the child '" +
                                   send.act.invokeId + "', which the document does not invoke");
            }
        }
        return read;
    }

    /// Re-arm each of `hosts` on `engine`, in the order a saved state lists them. None starts here:
    /// the host registers its invokers on the engine this restore returns, so each starts at the top
    /// of the first macrostep it drives (`startRestoredHostInvokes`). A deadline comes due `due -
    /// wallNowMs` after now on the engine's own clock; one already due ends the invocation without a
    /// start.
    template <typename Engine>
    static void rearmHostInvokes(Engine &engine, const std::vector<SavedHostInvoke> &hosts, std::int64_t wallNowMs) {
        const std::uint64_t now = engine.nowMs();
        for (const SavedHostInvoke &host : hosts) {
            typename Engine::RestoredHostInvoke restored;
            restored.request.processorType = host.processorType;
            restored.request.invokeId = host.invokeId;
            restored.request.src = host.src;
            restored.request.params =
                std::map<std::string, std::vector<std::string>>(host.params.begin(), host.params.end());
            restored.request.eventData = host.data;
            restored.request.content = host.content;
            restored.request.restarted = true;
            if (host.due.has_value()) {
                if (*host.due <= wallNowMs) {
                    restored.deadline = Engine::RestoredDeadline::Passed;
                } else {
                    restored.deadline = Engine::RestoredDeadline::At;
                    restored.atMs = detail::readyAt(now, *host.due, wallNowMs);
                }
            }
            engine.restoredHostInvokes_.push_back(std::move(restored));
        }
    }

    /// Arm each of `pending`, already read, in the order a saved state lists them: each comes due
    /// `due - wallNowMs` after the engine's now, and one that came due while the machine was away
    /// comes due now. Entries due at the same moment are armed in order and the scheduler delivers
    /// those in the order they were armed, which is how the saved machine would have.
    template <typename Engine>
    static void armPending(Engine &engine, const std::vector<ReadSend<typename Engine::Event>> &pending,
                           std::int64_t wallNowMs) {
        using Event = typename Engine::Event;
        using Kind = ::SCE::ScheduledRoute::Kind;
        const std::uint64_t now = engine.nowMs();
        for (const auto &read : pending) {
            const SavedAct &act = read.act;
            const std::uint64_t fireAt = detail::readyAt(now, read.due, wallNowMs);
            switch (act.kind) {
            case SavedActKind::Raise: {
                const std::string arrival = engine.arrivalNameOf(*read.event, act.event);
                if (arrival.empty()) {
                    engine.scheduler_.scheduleEventAt(*read.event, fireAt, act.sendId, act.data, act.origin);
                } else {
                    engine.scheduler_.scheduleRoutedAt(*read.event, fireAt, act.sendId, act.data, act.origin,
                                                       ::SCE::ScheduledRoute{Kind::ExternalQueue, act.event});
                }
                break;
            }
            case SavedActKind::Internal:
                engine.scheduler_.scheduleRoutedAt(*read.event, fireAt, act.sendId, act.data, act.origin,
                                                   ::SCE::ScheduledRoute{Kind::InternalQueue, act.event});
                break;
            case SavedActKind::Host: {
                ::SCE::HostSendRequest request;
                request.processorType = act.processorType;
                request.eventName = act.event;
                request.target = act.target;
                request.content = act.content;
                request.params = std::map<std::string, std::vector<std::string>>(act.params.begin(), act.params.end());
                request.sendId = act.sendId;
                request.eventData = act.data;
                request.invokeId = act.invokeId;
                engine.scheduler_.scheduleHostSendAt(std::make_shared<const ::SCE::HostSendRequest>(std::move(request)),
                                                     fireAt, act.sendId);
                break;
            }
            case SavedActKind::Parent:
                engine.scheduler_.scheduleRoutedAt(Event{}, fireAt, act.sendId, act.data, "",
                                                   ::SCE::ScheduledRoute{Kind::Parent, act.event});
                break;
            case SavedActKind::Child:
                engine.scheduler_.scheduleRoutedAt(Event{}, fireAt, act.sendId, act.data, act.origin,
                                                   ::SCE::ScheduledRoute{Kind::Invocation, act.event, act.invokeId});
                break;
            }
        }
    }
};

}  // namespace SCE::Saved
