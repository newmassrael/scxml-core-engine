// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trace port of a C++ machine: what a run tells a host of each step
// Appendix D takes, so the run can be judged a behaviour of the document's
// model (docs/adr/0115, D4 and D8).
//
// A step is the start of the interpreter, a microstep on eventless transitions,
// the taking of an internal or an external event (with the microstep it
// selects, or none), or the exit of the interpreter: the steps
// `docs/formal/AppendixD.tla` names `Start`, `Eventless`, `Internal`,
// `External` and `Stop`. Each is told once it is over, with what its record
// `told` holds there: the transitions it took by their rows, the states it
// exited in the order it exited them, the states it entered in the order it
// entered them, and the configuration it left, or, for a stop, the top-level
// `<final>` the run ended in. The macrosteps are the runs of steps from a start
// or an external event to the next one.
//
// A machine tells its steps only when its build asks for it: its policy's
// `TRACE_PORT` (`TracePortTrait`), which a generated machine takes from its
// `<NAME>_TRACE_PORT`, whose default is `SCE_TRACE_PORT` below, 0 unless set.
// Off, the engine holds no recorder (`TracePortSlot` is an empty base) and
// every hook is a discarded `if constexpr` branch, so the machine is what it
// was before the port existed. On, the engine holds a `TraceRecorder`, a host
// sets the port with `setTracePort`, and with no port set each hook asks one
// `std::function`. The port reads and is never asked anything, so a machine
// that tells its steps takes the steps one that does not takes.
//
// What a step names is the run trace format's (`tests/forge/conformance/
// run_trace.schema.json`, read by sce-build's `run_trace`), and `RunTrace`
// writes it in that format, laid out as the Rust runtime's `trace_port::RunTrace`
// writes it, which `sce-codegen formal traces` judges. An event from outside,
// and a delayed send that comes due, are steps of the environment and not of
// the interpreter (AppendixD's `Arrive` and `Due`): the engine cannot tell
// where an event on its external queue came from, and the external step that
// takes it names it.
//
// The cross-language siblings are the Rust runtime's `trace_port` module
// (feature `trace-port`) and the C11 runtime's `sce/trace_port.h` with
// `sce/trace_json.h`. Header-only; strings are written through
// `SCE::JsonText`, whose escapes are the one set every engine writes.

#pragma once

#include "common/JsonText.h"

#include <cstddef>
#include <cstdint>
#include <functional>
#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

// docs/adr/0115, D4 and D8: whether the machines of this build tell a trace port
// each step of Appendix D they take. Off by default. A generated machine's own
// `<NAME>_TRACE_PORT` takes this value unless the build sets that one, so a
// build can trace one machine of several.
#ifndef SCE_TRACE_PORT
#define SCE_TRACE_PORT 0
#endif

namespace SCE::Core {

/// The version of the run trace format `RunTrace::toJson` writes, the
/// `TRACE_FORMAT_VERSION` its reader in sce-build checks.
inline constexpr std::uint64_t TRACE_FORMAT_VERSION = 1;

/// The lane this runtime is, as a run trace names it.
inline constexpr const char *TRACE_LANE = "cpp";

/// Which step of Appendix D a `TraceStep` is.
enum class TraceStepKind {
    /// AppendixD's `Start`: the entry into the initial configuration.
    Start,
    /// AppendixD's `Eventless`: a microstep on eventless transitions.
    Eventless,
    /// AppendixD's `Internal`: an event taken off the internal queue, and the
    /// microstep it selects, if it selects one.
    Internal,
    /// AppendixD's `External`: an event taken off the external queue, and the
    /// microstep it selects, if it selects one.
    External,
    /// AppendixD's `Stop`: every state still active exited.
    Stop
};

/// The step's name in the run trace format.
constexpr const char *traceStepKindName(TraceStepKind kind) noexcept {
    switch (kind) {
    case TraceStepKind::Start:
        return "start";
    case TraceStepKind::Eventless:
        return "eventless";
    case TraceStepKind::Internal:
        return "internal";
    case TraceStepKind::External:
        return "external";
    case TraceStepKind::Stop:
        return "stop";
    }
    return "";
}

/// A transition a step took, by the pair its row of the emitted transition
/// table is named by (docs/adr/0089): the state it is written in, by its id,
/// and its position among that state's own transitions in document order.
struct TraceFired {
    /// The state the transition is written in, by its id.
    const char *source = "";
    /// Its position among `source`'s own transitions, from 0.
    std::uint32_t index = 0;

    friend bool operator==(const TraceFired &a, const TraceFired &b) {
        return std::string(a.source) == b.source && a.index == b.index;
    }

    friend bool operator!=(const TraceFired &a, const TraceFired &b) {
        return !(a == b);
    }
};

namespace detail {

inline void appendTraceText(std::string &out, const char *text) {
    out += '"';
    ::SCE::JsonText::appendEscaped(out, text);
    out += '"';
}

inline void appendTraceNames(std::string &out, const char *field, const std::vector<const char *> &names) {
    out += ", ";
    appendTraceText(out, field);
    out += ": [";
    for (std::size_t at = 0; at < names.size(); ++at) {
        if (at > 0) {
            out += ", ";
        }
        appendTraceText(out, names[at]);
    }
    out += ']';
}

}  // namespace detail

/// One step a run took, as the port is told it.
///
/// Each kind says what the run trace format asks of it and no more: a start
/// what it entered and the configuration; a microstep, eventless or on an
/// event, what it took, exited and entered and the configuration; an event step
/// the event too; a stop what it exited and the top-level final state the run
/// ended in. What a kind does not say is empty. The states are named by the
/// ids the policy's `getStateName` answers, which live as long as the program;
/// the event's name is the step's own.
struct TraceStep {
    /// Which step.
    TraceStepKind kind = TraceStepKind::Start;
    /// The event an internal or external step took, by the name `_event.name`
    /// holds while it is taken: the whole name it was raised or sent under,
    /// which is not the name of the event of the document it matched when a
    /// door resolved a longer name to it.
    std::optional<std::string> event;
    /// The transitions the step's microstep took, in the order they were
    /// selected.
    std::vector<TraceFired> fired;
    /// The states exited, in the order they were exited.
    std::vector<const char *> exited;
    /// The states entered, in the order they were entered.
    std::vector<const char *> entered;
    /// The configuration the step left, in document order; empty for a stop.
    std::vector<const char *> configuration;
    /// The top-level `<final>` the run ended in, of a stop that ends one;
    /// null otherwise.
    const char *outcome = nullptr;

    /// The step as one object of the run trace format: the fields its kind
    /// says, in the order the format lists them.
    std::string toJson() const {
        std::string out = "{\"kind\": ";
        detail::appendTraceText(out, traceStepKindName(kind));
        switch (kind) {
        case TraceStepKind::Start:
            detail::appendTraceNames(out, "entered", entered);
            detail::appendTraceNames(out, "configuration", configuration);
            break;
        case TraceStepKind::Eventless:
        case TraceStepKind::Internal:
        case TraceStepKind::External:
            if (kind != TraceStepKind::Eventless) {
                out += ", \"event\": ";
                detail::appendTraceText(out, event ? event->c_str() : "");
            }
            out += ", \"fired\": [";
            for (std::size_t at = 0; at < fired.size(); ++at) {
                if (at > 0) {
                    out += ", ";
                }
                out += "{\"source\": ";
                detail::appendTraceText(out, fired[at].source);
                out += ", \"index\": ";
                out += std::to_string(fired[at].index);
                out += '}';
            }
            out += ']';
            detail::appendTraceNames(out, "exited", exited);
            detail::appendTraceNames(out, "entered", entered);
            detail::appendTraceNames(out, "configuration", configuration);
            break;
        case TraceStepKind::Stop:
            detail::appendTraceNames(out, "exited", exited);
            out += ", \"outcome\": ";
            if (outcome != nullptr) {
                detail::appendTraceText(out, outcome);
            } else {
                out += "null";
            }
            break;
        }
        out += '}';
        return out;
    }
};

/// The steps of one run, kept as the port is told them, and written in the
/// run trace format.
class RunTrace {
public:
    /// Keep `step`, after the steps kept before it.
    void push(const TraceStep &step) {
        steps_.push_back(step);
    }

    /// The steps kept, in the order the run took them.
    const std::vector<TraceStep> &steps() const {
        return steps_;
    }

    /// The run in the run trace format: its version, its lane and its steps,
    /// one step to a line.
    std::string toJson() const {
        std::string out = "{\n  \"version\": ";
        out += std::to_string(TRACE_FORMAT_VERSION);
        out += ",\n  \"lane\": ";
        detail::appendTraceText(out, TRACE_LANE);
        out += ",\n  \"steps\": [";
        for (std::size_t at = 0; at < steps_.size(); ++at) {
            out += at == 0 ? "\n    " : ",\n    ";
            out += steps_[at].toJson();
        }
        out += steps_.empty() ? "]\n}\n" : "\n  ]\n}\n";
        return out;
    }

private:
    std::vector<TraceStep> steps_;
};

/// What `setTracePort` takes: told each step once it is over, on the thread
/// that drives the engine, inside the call that drove it. It must not call
/// back into the engine.
using TracePort = std::function<void(const TraceStep &)>;

/// The port a host set, if any, and the step being told.
///
/// A step is opened where Appendix D begins it and closed where it ends; the
/// transitions, exits and entries between are added to it as the engine takes
/// each. With no port set nothing is opened, so nothing is kept, and an exit or
/// an entry that is no part of a step (a restore entering a configuration) is
/// told to no one.
class TraceRecorder {
public:
    /// Tell `port` each step from now on (an empty function tells none).
    void set(TracePort port) {
        port_ = std::move(port);
        step_.reset();
    }

    /// Tell no port anything.
    void clear() {
        port_ = nullptr;
        step_.reset();
    }

    /// Whether a port is set, so a step would be told.
    bool hasPort() const {
        return static_cast<bool>(port_);
    }

    /// Whether a step is being told.
    bool isOpen() const {
        return step_.has_value();
    }

    /// Begin a step of `kind`, which took the event named `event` when it is
    /// an event step, in place of any step left open; nothing when no port is
    /// set.
    void begin(TraceStepKind kind, std::optional<std::string> event = std::nullopt) {
        if (!port_) {
            return;
        }
        TraceStep step;
        step.kind = kind;
        step.event = std::move(event);
        step_ = std::move(step);
    }

    /// The step's microstep takes the transition written `index`-th in the
    /// state named `source`.
    void fired(const char *source, std::uint32_t index) {
        if (step_) {
            step_->fired.push_back(TraceFired{source, index});
        }
    }

    /// The step exits the state named `state`.
    void exited(const char *state) {
        if (step_) {
            step_->exited.push_back(state);
        }
    }

    /// The step enters the state named `state`.
    void entered(const char *state) {
        if (step_) {
            step_->entered.push_back(state);
        }
    }

    /// The step is over: tell the port it, with the configuration it left
    /// (the states active, in document order), or, for a stop, the top-level
    /// `<final>` the run ended in, null when it ended in none.
    void end(std::vector<const char *> configuration, const char *outcome) {
        if (!port_ || !step_) {
            return;
        }
        TraceStep step = std::move(*step_);
        step_.reset();
        if (step.kind == TraceStepKind::Stop) {
            step.outcome = outcome;
        } else {
            step.configuration = std::move(configuration);
        }
        port_(step);
    }

private:
    TracePort port_;
    std::optional<TraceStep> step_;
};

/// Whether the machines of policy `P` tell a trace port: `P::TRACE_PORT`, and
/// false for a policy that declares none, as every policy written by hand.
template <typename P, typename = void> struct TracePortTrait : std::false_type {};

template <typename P>
struct TracePortTrait<P, std::void_t<decltype(P::TRACE_PORT)>> : std::bool_constant<static_cast<bool>(P::TRACE_PORT)> {
};

/// Where an engine holds its recorder: an empty base when its machine tells no
/// trace port, which costs the engine no byte, and the recorder when it does.
template <bool Traced> struct TracePortSlot {};

template <> struct TracePortSlot<true> {
    TraceRecorder traceRecorder_;
};

}  // namespace SCE::Core
