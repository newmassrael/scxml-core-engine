// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The trace port: what a run tells a test of each step Appendix D takes, so the run can be judged a
// behaviour of the document's model (docs/adr/0115, D4 and D8).
//
// A step is the start of the interpreter, a microstep on eventless transitions, the taking of an
// internal or an external event (with the microstep it selects, or none), or the exit of the
// interpreter: the steps `docs/formal/AppendixD.tla` names `Start`, `Eventless`, `Internal`,
// `External` and `Stop`. Each is told once it is over, with what its record `told` holds there: the
// transitions it took by their rows, the states it exited in the order it exited them, the states it
// entered in the order it entered them, and the configuration it left. The macrosteps are the runs of
// steps from a start or an external event to the next one.
//
// What the port is told is written in the one run trace format every lane writes (`RunTrace.toJSON`,
// `tests/forge/conformance/run_trace.schema.json`, read by sce-build's `run_trace`), and
// `sce-codegen formal traces` judges the file against the model it names.
//
// The steps and their writer are types of this module, which cost a machine nothing it does not call.
// What tells them anything is the engine's (`Engine.setTracePort`), and that is the `TracePort` trait
// of this package, which no default asks for: a build without it compiles no field, no call and no
// code of the port into `Engine` (the compilation condition `SCE_TRACE_PORT`), and with it and no
// port set each step asks one optional. The port reads and is never asked anything, so a machine with
// a port set takes the steps one without takes.
//
// An event from outside, and a delayed send that comes due, are steps of the environment and not of
// the interpreter (AppendixD's `Arrive` and `Due`): the engine cannot tell where an event on its
// external queue came from, and the external step that takes it names it.
//
// The cross-language siblings are the Rust runtime's `trace_port` module (feature `trace-port`) and
// the C11 runtime's `sce/trace_port.h` with `sce/trace_json.h` (`SCE_TRACE_PORT`).

/// The version of the run trace format `RunTrace.toJSON` writes, the `TRACE_FORMAT_VERSION` its
/// reader in sce-build checks.
public let traceFormatVersion = 1

/// The lane this runtime is, as a run trace names it.
public let traceLane = "swift"

/// Which step of Appendix D a `TraceStep` is.
public enum TraceStepKind: Hashable, Sendable {
    /// AppendixD's `Start`: the entry into the initial configuration.
    case start
    /// AppendixD's `Eventless`: a microstep on eventless transitions.
    case eventless
    /// AppendixD's `Internal`: an event taken off the internal queue, and the microstep it selects,
    /// if it selects one.
    case `internal`
    /// AppendixD's `External`: an event taken off the external queue, and the microstep it selects,
    /// if it selects one.
    case external
    /// AppendixD's `Stop`: every state still active exited.
    case stop

    /// The step's name in the run trace format.
    public var name: String {
        switch self {
        case .start:
            return "start"
        case .eventless:
            return "eventless"
        case .internal:
            return "internal"
        case .external:
            return "external"
        case .stop:
            return "stop"
        }
    }
}

/// A transition a step took, by the pair its row of the emitted transition table is named by
/// (docs/adr/0089): the state it is written in and its position among that state's own transitions
/// in document order.
public struct TraceFired: Hashable, Sendable {
    /// The state the transition is written in, by its id.
    public var source: String

    /// Its position among `source`'s own transitions, from 0.
    public var index: Int

    public init(source: String, index: Int) {
        self.source = source
        self.index = index
    }
}

/// One step a run took, as the port is told it.
///
/// Each kind says what the run trace format asks of it and no more: a start what it entered and the
/// configuration; a microstep, eventless or on an event, what it took, exited and entered and the
/// configuration; an event step the event too; a stop what it exited and the top-level final state
/// the run ended in. What a kind does not say is empty.
public struct TraceStep: Hashable, Sendable {
    /// Which step.
    public var kind: TraceStepKind

    /// The event an internal or external step took, by the name `_event.name` holds while it is
    /// taken: the whole name it was raised or sent under, which is not the name of the event of the
    /// document it matched when a door resolved a longer name, or one the document never writes, to
    /// it.
    public var event: String?

    /// The transitions the step's microstep took, in the order they were selected.
    public var fired: [TraceFired]

    /// The states exited, in the order they were exited.
    public var exited: [String]

    /// The states entered, in the order they were entered.
    public var entered: [String]

    /// The configuration the step left, in document order; empty for a stop.
    public var configuration: [String]

    /// The top-level `<final>` the run ended in, of a stop that ends one.
    public var outcome: String?

    public init(
        kind: TraceStepKind,
        event: String? = nil,
        fired: [TraceFired] = [],
        exited: [String] = [],
        entered: [String] = [],
        configuration: [String] = [],
        outcome: String? = nil
    ) {
        self.kind = kind
        self.event = event
        self.fired = fired
        self.exited = exited
        self.entered = entered
        self.configuration = configuration
        self.outcome = outcome
    }

    /// The step as one object of the run trace format: the fields its kind says, in the order the
    /// format lists them, each text in the one JSON form every engine writes (`quoteJSON`).
    public func toJSON() -> String {
        var out = "{\"kind\": " + quoteJSON(kind.name)
        switch kind {
        case .start:
            appendTraceNames(&out, "entered", entered)
            appendTraceNames(&out, "configuration", configuration)
        case .eventless, .internal, .external:
            if kind != .eventless {
                out += ", \"event\": " + quoteJSON(event ?? "")
            }
            out += ", \"fired\": ["
            for (at, row) in fired.enumerated() {
                if at > 0 {
                    out += ", "
                }
                out += "{\"source\": " + quoteJSON(row.source) + ", \"index\": " + String(row.index) + "}"
            }
            out += "]"
            appendTraceNames(&out, "exited", exited)
            appendTraceNames(&out, "entered", entered)
            appendTraceNames(&out, "configuration", configuration)
        case .stop:
            appendTraceNames(&out, "exited", exited)
            out += ", \"outcome\": " + (outcome.map(quoteJSON) ?? "null")
        }
        out += "}"
        return out
    }
}

/// `, "<field>": [<name>, ...]` after what `out` holds.
private func appendTraceNames(_ out: inout String, _ field: String, _ names: [String]) {
    out += ", " + quoteJSON(field) + ": ["
    for (at, name) in names.enumerated() {
        if at > 0 {
            out += ", "
        }
        out += quoteJSON(name)
    }
    out += "]"
}

/// The steps of one run, kept as the port is told them, and written in the run trace format.
public struct RunTrace: Hashable, Sendable {
    /// The steps kept, in the order the run took them.
    public private(set) var steps: [TraceStep] = []

    /// A trace of no step yet.
    public init() {}

    /// Keeps `step`, after the steps kept before it.
    public mutating func push(_ step: TraceStep) {
        steps.append(step)
    }

    /// The run in the run trace format: its version, its lane and its steps, one step to a line, laid
    /// out as the Rust runtime's `trace_port::RunTrace` writes it, so two lanes' runs of one document
    /// differ, line for line, only where the runs do.
    public func toJSON() -> String {
        var out = "{\n  \"version\": " + String(traceFormatVersion) + ",\n  \"lane\": " + quoteJSON(traceLane)
            + ",\n  \"steps\": ["
        for (at, step) in steps.enumerated() {
            out += at == 0 ? "\n    " : ",\n    "
            out += step.toJSON()
        }
        out += steps.isEmpty ? "]\n}\n" : "\n  ]\n}\n"
        return out
    }
}

/// What the engine holds for a port a test set: told each step once it is over.
typealias TracePort = (TraceStep) -> Void

/// The port a test set, if any, and the step being told.
///
/// A step is opened where Appendix D begins it and closed where it ends; the transitions taken and
/// the states exited and entered between are added to it as the engine takes, exits and enters each.
/// With no port set nothing is opened, so nothing is kept, and what a name would cost to compute is
/// not computed: each name is handed over as an autoclosure, asked only while a step is open.
struct TraceRecorder {
    private var port: TracePort?
    private var step: TraceStep?

    /// No port, no step: what every engine starts with.
    init() {}

    /// Tells `port` each step from now on.
    mutating func set(_ port: @escaping TracePort) {
        self.port = port
        step = nil
    }

    /// Tells no port anything.
    mutating func clear() {
        port = nil
        step = nil
    }

    /// Whether a step is being told.
    var isOpen: Bool {
        step != nil
    }

    /// Begins a step of `kind`, which took the event named `event` when it is an event step (nil
    /// otherwise), in place of any step left open; nothing when no port is set.
    mutating func begin(_ kind: TraceStepKind, event: @autoclosure () -> String?) {
        guard port != nil else {
            return
        }
        step = TraceStep(kind: kind, event: event())
    }

    /// The open step's microstep takes the transition written `index`-th in the state named
    /// `source`.
    mutating func fired(source: @autoclosure () -> String, index: Int) {
        guard step != nil else {
            return
        }
        let row = TraceFired(source: source(), index: index)
        step?.fired.append(row)
    }

    /// The open step exits the state named `state`.
    mutating func exited(_ state: @autoclosure () -> String) {
        guard step != nil else {
            return
        }
        let name = state()
        step?.exited.append(name)
    }

    /// The open step enters the state named `state`.
    mutating func entered(_ state: @autoclosure () -> String) {
        guard step != nil else {
            return
        }
        let name = state()
        step?.entered.append(name)
    }

    /// Closes the open step with what it left, the configuration or, for a stop, the top-level
    /// `<final>` the run ended in, and hands it back with the port to tell it; nil when no step was
    /// open. The caller tells the port once this has returned, so a port is never called while the
    /// recorder is being changed.
    mutating func close(
        configuration: @autoclosure () -> [String],
        outcome: @autoclosure () -> String?
    ) -> (port: TracePort, step: TraceStep)? {
        guard let port, var closed = step else {
            return nil
        }
        step = nil
        if closed.kind == .stop {
            closed.outcome = outcome()
        } else {
            closed.configuration = configuration()
        }
        return (port, closed)
    }
}
