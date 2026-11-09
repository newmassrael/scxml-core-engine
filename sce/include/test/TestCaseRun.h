// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The run of one test case of the test profile (docs/SCE_TEST_PROFILE.md §2 to
// §5, §12): its entities, its ports and timers, the round scheduler on the
// logical clock, and the test case verdict.
//
// A component's machine is generated from its `test-component` and reaches
// this run only through its ComponentContext (§12.3): the snapshot its
// branches' guards read, the statement each taken branch runs first, the
// statements of the profile, and the doors of a send and of a PTC. The run
// knows nothing of the engine or of the record types, so it is written once
// for every suite and tested on its own.

#pragma once

#include <algorithm>
#include <exception>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

#include "test/ModuleParameters.h"
#include "test/Port.h"
#include "test/TestLog.h"
#include "test/Timer.h"
#include "test/Verdict.h"

namespace SCE::Test {

/// The name of the MTC in a configuration; no PTC takes it (§11.6).
inline constexpr const char *MTC_NAME = "mtc";

/// A component port named in a configuration: the component's name (`mtc` or a
/// PTC's id) and the port's id.
struct Endpoint {
    std::string component;
    std::string port;

    friend bool operator<(const Endpoint &a, const Endpoint &b) {
        return std::tie(a.component, a.port) < std::tie(b.component, b.port);
    }

    friend bool operator==(const Endpoint &a, const Endpoint &b) {
        return a.component == b.component && a.port == b.port;
    }
};

/**
 * @brief A `test-configuration` as the run reads it (§11.6)
 *
 * Generated from the configuration document. Each port takes part in one
 * connection or one mapping (§2.2), which the build has checked.
 */
struct Configuration {
    /// The MTC's component type.
    std::string mtcType;
    /// Each PTC's name and its component type.
    std::map<std::string, std::string> ptcTypes;
    /// The port ids each component type declares.
    std::map<std::string, std::vector<std::string>> portsOfType;
    /// The timer ids each component type declares.
    std::map<std::string, std::vector<std::string>> timersOfType;
    /// The `<sce:connect>` pairs (-4 §9.10).
    std::vector<std::pair<Endpoint, Endpoint>> connections;
    /// The `<sce:map>` entries: a component port and its system port (-4 §9.32).
    std::vector<std::pair<Endpoint, std::string>> mappings;
};

/// What a line of the run's log records (§12.5): the run as a reader follows
/// it, with the removals of PTCs and the cause of an `error` in the runtime's
/// words. The test log, which two runtimes compare, is TestCaseResult::records.
enum class LogKind { Start, Create, Send, Receive, Timeout, Done, Kill, Verdict, Log, Error };

/// One line of the run's log.
struct LogRecord {
    Instant at;
    std::string component;
    LogKind kind;
    std::string detail;
};

/// The outcome of one test case: its verdict, the instant it ended at, the
/// lines of its log, and its test log.
struct TestCaseResult {
    Verdict verdict = Verdict::None;
    Instant endedAt = 0;
    /// The lines of the run's log, for a reader.
    std::vector<LogRecord> log;
    /// The test log (docs/adr/0091, C3): the records of TCI's test logging
    /// interface, in the order the run produced them, which canonicalLog()
    /// writes.
    std::vector<TestLogRecord> records;

    /// The test log of this run of the test case @p testCase of the suite
    /// @p suite, in its one canonical text (writeTestLog()).
    std::string canonicalLog(const std::string &suite, const std::string &testCase) const {
        return writeTestLog(suite, testCase, verdict, records);
    }
};

/**
 * @brief The SUT's side of the mapped ports (§12.2)
 *
 * Shaped after TRI (ES 201 873-5): `send` is `triSend`, and the inputs it
 * returns are what `triEnqueueMsg` would enqueue. The adapter owns the codec
 * and the transport; the run sees values only (ADR 0091 C2). The instants are
 * the logical clock's, so a recorded trace replays to the same run (§3.1).
 */
class SystemAdapter {
public:
    virtual ~SystemAdapter() = default;

    /// A component sent @p message on a port mapped to @p systemPort.
    virtual void send(const std::string &systemPort, const Message &message, Instant now) = 0;

    /// The instant of the earliest input not yet taken, if any.
    virtual std::optional<Instant> nextInput() const = 0;

    /// Every input due at or before @p now, in arrival order, each with the
    /// system port it arrives on.
    virtual std::vector<std::pair<std::string, Message>> takeInputsDue(Instant now) = 0;
};

class TestCaseRun;
class ComponentContext;

/**
 * @brief A component's machine, as the run drives it (§12.3)
 *
 * Generated from a `test-component`: the backend's statechart machine, with
 * each branch a transition on the round's snapshot event.
 */
class ComponentMachine {
public:
    virtual ~ComponentMachine() = default;

    /// Enter the initial configuration and complete the first macrostep. Every
    /// call the machine makes goes to @p context, which outlives it.
    virtual void start(ComponentContext &context) = 0;

    /// Deliver the round's snapshot event: the machine takes the first branch
    /// whose guard holds on the snapshot (P3) and completes its macrostep (P4),
    /// or takes none.
    virtual void deliverSnapshot() = 0;

    /// The machine has entered a top-level `<final>` (P10).
    virtual bool terminated() const = 0;
};

/// Builds the machine of a component type, or none for a type it does not make,
/// which ends the run with `error` where a component of that type is created.
using MachineFactory = std::function<std::unique_ptr<ComponentMachine>(const std::string &componentType)>;

/**
 * @brief What a component's machine reads and does through the run (§12.3)
 *
 * One per component; its address does not change while the component lives.
 */
class ComponentContext {
public:
    ComponentContext(TestCaseRun &run, std::string name) : run_(run), name_(std::move(name)) {}

    ComponentContext(const ComponentContext &) = delete;
    ComponentContext &operator=(const ComponentContext &) = delete;

    const std::string &name() const {
        return name_;
    }

    // ── The snapshot, read by the branches' guards (P2) ──────────────

    /// The message the snapshot saw at the head of @p port, or none.
    const Message *snapshotOf(const std::string &port) const;
    /// The snapshot saw @p timer timed out.
    bool timedOut(const std::string &timer) const;
    /// The snapshot saw the PTC @p ptc in `DONE`.
    bool done(const std::string &ptc) const;

    // ── Taking a branch (P4 step 1), the first statement of each ─────

    /// A receive branch: remove the head of @p port, which its content reads
    /// as `_event.data`.
    Message receive(const std::string &port);
    /// A timeout branch: @p timer becomes idle.
    void timeout(const std::string &timer);
    /// A done branch on the PTC @p ptc: nothing is removed (-4 §9.16).
    void doneTaken(const std::string &ptc);

    // ── Statements of the profile ────────────────────────────────────

    void setverdict(Verdict verdict, const std::string &reason);
    void startTimer(const std::string &timer, Duration duration);
    void stopTimer(const std::string &timer);
    void log(const std::string &text);

    // ── Doors (§12.3) ────────────────────────────────────────────────

    /// A send on @p port (P6).
    void send(const std::string &port, Message message);
    /// The start of the invocation that creates the PTC @p ptc (P9).
    void create(const std::string &ptc);
    /// The cancel of that invocation: the PTC is removed (P9).
    void kill(const std::string &ptc);

    // ── Module parameters (§11.5, §11.8) ─────────────────────────────

    /// Hands @p read the module parameters of the run, which it reads into the
    /// variables the component declares as module parameters, before the
    /// component's machine starts. A value the run was not given, or one @p read
    /// refuses as not of the type the component declares, is a dynamic error
    /// (P13): the run ends with `error` and false is returned, so the machine is
    /// not started on a value made up in its place.
    template <typename Read> bool readModuleParameters(Read &&read);

    Instant now() const;

private:
    friend class TestCaseRun;

    TestCaseRun &run_;
    std::string name_;
    std::map<std::string, Port> ports_;
    std::map<std::string, Timer> timers_;
    std::set<std::string> doneSnapshot_;
    Verdict verdict_ = Verdict::None;
    bool branchTaken_ = false;
};

/**
 * @brief One run of one test case (§2.3, §3, §4.9 to §4.11, §5)
 *
 * The components run on the calling thread, in the order they were created;
 * §3.2 shows that the order within a round does not change the result.
 */
class TestCaseRun {
public:
    /// @param guard The test case's guard duration (P11), if it has one.
    /// @param moduleParameters The values of the module parameters the
    ///        components read (§11.8): the suite's, with any the caller overrides.
    TestCaseRun(Configuration configuration, MachineFactory factory, SystemAdapter &adapter,
                std::optional<Duration> guard, ModuleParameters moduleParameters = ModuleParameters())
        : configuration_(std::move(configuration)), factory_(std::move(factory)), adapter_(adapter), guard_(guard),
          moduleParameters_(std::move(moduleParameters)) {}

    /// Runs the test case to its end and returns its verdict and logs.
    TestCaseResult run() {
        result_ = TestCaseResult{};
        now_ = 0;
        trace(TestLog::TestCaseStarted{});
        createEntity(MTC_NAME, configuration_.mtcType);
        while (!ended()) {
            round();
        }
        finish();
        return result_;
    }

private:
    friend class ComponentContext;

    struct Entity {
        /// Which creation this is: a PTC removed and created again in one
        /// round is a new component, which took no snapshot in it.
        std::uint64_t serial = 0;
        std::string type;
        std::unique_ptr<ComponentContext> context;
        std::unique_ptr<ComponentMachine> machine;
        bool terminated = false;
    };

    // ── The round (§3.2, §12.4) ──────────────────────────────────────

    void round() {
        // Step 2, the update: inputs and timer expiries due at NOW.
        for (auto &[systemPort, message] : adapter_.takeInputsDue(now_)) {
            deliverInput(systemPort, std::move(message));
        }
        for (auto &[name, entity] : entities_) {
            for (auto &[id, timer] : entity.context->timers_) {
                timer.update(now_);
            }
        }
        // Step 3, the snapshot: every waiting component at once.
        std::vector<std::pair<std::string, std::uint64_t>> waiting;
        for (const auto &name : order_) {
            auto found = entities_.find(name);
            if (found == entities_.end() || found->second.terminated) {
                continue;
            }
            ComponentContext &context = *found->second.context;
            for (auto &[id, port] : context.ports_) {
                port.takeSnapshot();
            }
            for (auto &[id, timer] : context.timers_) {
                timer.takeSnapshot();
            }
            context.doneSnapshot_ = done_;
            waiting.emplace_back(name, found->second.serial);
        }
        // Steps 4 and 5: one component at a time, in creation order, each
        // selects on its own snapshot and runs the macrostep of the branch it
        // took. A component removed or terminated earlier in the round does
        // not select, and nothing does once the MTC has terminated (§3.2).
        bool anyTaken = false;
        for (const auto &[name, serial] : waiting) {
            if (ended()) {
                break;
            }
            auto found = entities_.find(name);
            if (found == entities_.end() || found->second.serial != serial || found->second.terminated) {
                continue;
            }
            Entity &entity = found->second;
            entity.context->branchTaken_ = false;
            entity.machine->deliverSnapshot();
            anyTaken = anyTaken || entity.context->branchTaken_;
            settle(name);
        }
        if (ended() || anyTaken) {
            return;
        }
        advance();
    }

    /// No branch fired: time advances to the next instant something can
    /// happen at (§3.3), or the test case can never end (P13).
    void advance() {
        std::optional<Instant> next = adapter_.nextInput();
        for (const auto &[name, entity] : entities_) {
            if (entity.terminated) {
                continue;
            }
            for (const auto &[id, timer] : entity.context->timers_) {
                if (auto expiry = timer.expiry()) {
                    next = next ? std::min(*next, *expiry) : *expiry;
                }
            }
        }
        if (guard_) {
            next = next ? std::min(*next, *guard_) : *guard_;
        }
        if (!next) {
            fail(MTC_NAME, "no branch can fire and no instant is pending: the test case can never end");
            return;
        }
        now_ = std::max(now_, *next);
        if (guard_ && now_ >= *guard_) {
            fail(MTC_NAME, "the guard of the test case was reached");
        }
    }

    /// A component that has just run: if it terminated, P10 applies.
    void settle(const std::string &name) {
        auto found = entities_.find(name);
        if (found == entities_.end() || found->second.terminated || !found->second.machine->terminated()) {
            return;
        }
        found->second.terminated = true;
        trace(TestLog::ComponentDone{name, found->second.context->verdict_});
        if (name == MTC_NAME) {
            mtcTerminated_ = true;
            return;
        }
        done_.insert(name);
        record(name, LogKind::Done, "");
    }

    bool ended() const {
        return mtcTerminated_ || error_;
    }

    /// The MTC terminated or the run ended in error: every PTC still present is
    /// removed, and the test case verdict is the worst local verdict (P10, P14).
    void finish() {
        std::vector<std::string> ptcs;
        for (const auto &[name, entity] : entities_) {
            if (name != MTC_NAME) {
                ptcs.push_back(name);
            }
        }
        for (const auto &name : ptcs) {
            removeEntity(name);
        }
        Verdict verdict = removedVerdict_;
        if (auto mtc = entities_.find(MTC_NAME); mtc != entities_.end()) {
            verdict = worse(verdict, mtc->second.context->verdict_);
        }
        if (error_) {
            verdict = Verdict::Error;
        }
        result_.verdict = verdict;
        result_.endedAt = now_;
        trace(TestLog::TestCaseEnded{verdict});
    }

    // ── Entities (§2.1, P9) ──────────────────────────────────────────

    void createEntity(const std::string &name, const std::string &type) {
        std::unique_ptr<ComponentMachine> machine = factory_(type);
        if (!machine) {
            fail(name, "is of the component type '" + type + "', which the factory of machines does not make");
            return;
        }
        Entity entity;
        entity.serial = ++created_;
        entity.type = type;
        entity.context = std::make_unique<ComponentContext>(*this, name);
        if (auto ports = configuration_.portsOfType.find(type); ports != configuration_.portsOfType.end()) {
            for (const auto &port : ports->second) {
                entity.context->ports_[port];
            }
        }
        if (auto timers = configuration_.timersOfType.find(type); timers != configuration_.timersOfType.end()) {
            for (const auto &timer : timers->second) {
                entity.context->timers_[timer];
            }
        }
        entity.machine = std::move(machine);
        ComponentContext &context = *entity.context;
        ComponentMachine &started = *entity.machine;
        entities_.emplace(name, std::move(entity));
        order_.push_back(name);
        record(name, name == MTC_NAME ? LogKind::Start : LogKind::Create, type);
        trace(TestLog::ComponentCreated{name, type});
        started.start(context);
        settle(name);
    }

    void create(const std::string &creator, const std::string &ptc) {
        auto type = configuration_.ptcTypes.find(ptc);
        if (type == configuration_.ptcTypes.end()) {
            fail(creator, "creates '" + ptc + "', which the configuration does not declare");
            return;
        }
        if (entities_.count(ptc) != 0) {
            fail(creator, "creates '" + ptc + "', which is already present");
            return;
        }
        done_.erase(ptc);
        createEntity(ptc, type->second);
    }

    /// Removes a PTC: its local verdict still counts (-4 §9.29a.2), and its
    /// ports are gone, so a send to one of them is an error.
    void removeEntity(const std::string &name) {
        auto found = entities_.find(name);
        if (found == entities_.end()) {
            return;
        }
        removedVerdict_ = worse(removedVerdict_, found->second.context->verdict_);
        record(name, LogKind::Kill, "");
        entities_.erase(found);
        order_.erase(std::remove(order_.begin(), order_.end(), name), order_.end());
        done_.erase(name);
    }

    // ── Ports (§2.2, P6) ─────────────────────────────────────────────

    void send(const std::string &sender, const std::string &port, Message message) {
        const Endpoint from{sender, port};
        record(sender, LogKind::Send, port + " " + message.event);
        trace(TestLog::MessageSent{sender, port, message.value.typeName(), message.value.canonical()});
        for (const auto &[a, b] : configuration_.connections) {
            if (a == from || b == from) {
                const Endpoint &to = a == from ? b : a;
                auto peer = entities_.find(to.component);
                if (peer == entities_.end()) {
                    fail(sender, "sends on '" + port + "', whose peer '" + to.component + "' is not present");
                    return;
                }
                peer->second.context->ports_[to.port].enqueue(std::move(message));
                return;
            }
        }
        for (const auto &[endpoint, systemPort] : configuration_.mappings) {
            if (endpoint == from) {
                adapter_.send(systemPort, message, now_);
                return;
            }
        }
        fail(sender, "sends on '" + port + "', which is neither connected nor mapped");
    }

    void deliverInput(const std::string &systemPort, Message message) {
        for (const auto &[endpoint, mapped] : configuration_.mappings) {
            if (mapped != systemPort) {
                continue;
            }
            auto component = entities_.find(endpoint.component);
            if (component == entities_.end()) {
                fail(endpoint.component, "an input on system port '" + systemPort + "' found no component");
                return;
            }
            component->second.context->ports_[endpoint.port].enqueue(std::move(message));
            return;
        }
        fail(MTC_NAME, "an input on system port '" + systemPort + "', which no port maps");
    }

    // ── The verdict and the log (§5, §12.5) ──────────────────────────

    /// A dynamic error (P13): the run ends with `error`. The test log records
    /// the verdict without @p why, which is this runtime's own prose.
    void fail(const std::string &component, const std::string &why) {
        error_ = true;
        record(component, LogKind::Error, why);
        trace(TestLog::VerdictSet{component, Verdict::Error, std::nullopt});
    }

    void record(const std::string &component, LogKind kind, std::string detail) {
        result_.log.push_back(LogRecord{now_, component, kind, std::move(detail)});
    }

    /// A record of the test log, at `NOW`.
    void trace(TestLogEvent event) {
        result_.records.push_back(TestLogRecord{now_, std::move(event)});
    }

    Configuration configuration_;
    MachineFactory factory_;
    SystemAdapter &adapter_;
    std::optional<Duration> guard_;
    ModuleParameters moduleParameters_;

    Instant now_ = 0;
    std::uint64_t created_ = 0;
    std::map<std::string, Entity> entities_;
    std::vector<std::string> order_;
    std::set<std::string> done_;
    Verdict removedVerdict_ = Verdict::None;
    bool mtcTerminated_ = false;
    bool error_ = false;
    TestCaseResult result_;
};

// ── ComponentContext, which forwards to its run ──────────────────────

inline const Message *ComponentContext::snapshotOf(const std::string &port) const {
    auto found = ports_.find(port);
    return found == ports_.end() ? nullptr : found->second.snapshot();
}

inline bool ComponentContext::timedOut(const std::string &timer) const {
    auto found = timers_.find(timer);
    return found != timers_.end() && found->second.snapshotTimedOut();
}

inline bool ComponentContext::done(const std::string &ptc) const {
    return doneSnapshot_.count(ptc) != 0;
}

inline Message ComponentContext::receive(const std::string &port) {
    branchTaken_ = true;
    Message message = ports_.at(port).removeHead();
    run_.record(name_, LogKind::Receive, port + " " + message.event);
    run_.trace(TestLog::MessageReceived{name_, port, message.value.typeName(), message.value.canonical()});
    return message;
}

inline void ComponentContext::timeout(const std::string &timer) {
    branchTaken_ = true;
    timers_.at(timer).timeoutTaken();
    run_.record(name_, LogKind::Timeout, timer);
    run_.trace(TestLog::TimerTimedOut{name_, timer});
}

inline void ComponentContext::doneTaken(const std::string &ptc) {
    branchTaken_ = true;
    (void)ptc;
}

inline void ComponentContext::setverdict(Verdict verdict, const std::string &reason) {
    verdict_ = worse(verdict_, verdict);
    run_.record(name_, LogKind::Verdict, std::string(verdictName(verdict)) + (reason.empty() ? "" : " " + reason));
    run_.trace(TestLog::VerdictSet{name_, verdict, reason.empty() ? std::nullopt : std::optional<std::string>(reason)});
}

inline void ComponentContext::startTimer(const std::string &timer, Duration duration) {
    timers_[timer].start(run_.now_, duration);
    run_.trace(TestLog::TimerStarted{name_, timer, duration});
}

/// The test log records the stop of a running timer; the stop of one that is
/// not running changes nothing.
inline void ComponentContext::stopTimer(const std::string &timer) {
    Timer &binding = timers_[timer];
    const bool running = binding.status() == TimerStatus::Running;
    binding.stop();
    if (running) {
        run_.trace(TestLog::TimerStopped{name_, timer});
    }
}

inline void ComponentContext::log(const std::string &text) {
    run_.record(name_, LogKind::Log, text);
    run_.trace(TestLog::Log{name_, text});
}

inline void ComponentContext::send(const std::string &port, Message message) {
    run_.send(name_, port, std::move(message));
}

inline void ComponentContext::create(const std::string &ptc) {
    run_.create(name_, ptc);
}

inline void ComponentContext::kill(const std::string &ptc) {
    run_.removeEntity(ptc);
}

template <typename Read> bool ComponentContext::readModuleParameters(Read &&read) {
    try {
        const ModuleParameters &given = run_.moduleParameters_;
        std::forward<Read>(read)(given);
        return true;
    } catch (const std::exception &refused) {
        run_.fail(name_, refused.what());
        return false;
    }
}

inline Instant ComponentContext::now() const {
    return run_.now_;
}

/**
 * @brief Whether a receive branch with a template matches the snapshot of
 *        @p context (P2, docs/SCE_TEST_PROFILE.md §12.3)
 *
 * The snapshot saw a message at the head of @p port, under @p event, the event
 * name of the template's record, whose value is a `Value`, and @p matches, the
 * template's `match` bound to its arguments, holds of that value. A message of
 * another record, or one whose value is of another type, does not match.
 */
template <typename Value, typename Matches>
bool receives(const ComponentContext &context, const std::string &port, const std::string &event, Matches &&matches) {
    const Message *head = context.snapshotOf(port);
    if (head == nullptr || head->event != event) {
        return false;
    }
    const Value *value = head->value.as<Value>();
    return value != nullptr && std::forward<Matches>(matches)(*value);
}

}  // namespace SCE::Test
