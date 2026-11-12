// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Written by hand: a generated machine held to the transition function generated beside it
// (docs/SCE_TRANSITION_FUNCTION.md).
//
// The function is computed at build time by the selection rule the engine follows at run time, so
// the two are one fact only if every pair agrees, and this is what asks: a fresh machine per pair,
// driven to the state by the shortest event path the function itself gives, then sent the event.
// What the engine reaches, and whether it took the event or discarded it, must be what the function
// says. The drivers of the integration stems that declare a function share it, so the stems ask the
// same question the same way.

import Testing

/// A generated transition function, read through its accessors: its names are the generator's, so
/// the drive is handed what it needs of it by value.
struct TransitionFunctionView<S: Hashable, E: Hashable> {
    var states: [S]
    var events: [E]
    var initial: S
    var transition: (S, E) -> S?
    var isFinal: (S) -> Bool
    var configuration: (S) -> [String]
    var stateName: (S) -> String
    var eventName: (E) -> String
}

/// What the drive asks of the engine it holds to a function: a fresh machine at its initial
/// configuration, an event delivered by name and run to completion, the ids of the active states,
/// the final the run ended in, and how many events the engine has discarded.
struct EngineProbe {
    var start: () throws -> Void
    var send: (String) -> Void
    var active: () -> Set<String>
    var terminal: () -> String?
    var discarded: () -> Int
}

/// The shortest sequence of events that takes `function` from its initial state to `target`, read
/// from the function alone; `nil` when no sequence does.
func shortestPath<S: Hashable, E: Hashable>(
    _ function: TransitionFunctionView<S, E>, to target: S
) -> [E]? {
    var frontier: [(state: S, path: [E])] = [(function.initial, [])]
    var seen: Set<S> = [function.initial]
    var next = 0
    while next < frontier.count {
        let (state, path) = frontier[next]
        next += 1
        if state == target {
            return path
        }
        for event in function.events {
            if let to = function.transition(state, event), seen.insert(to).inserted {
                frontier.append((to, path + [event]))
            }
        }
    }
    return nil
}

/// Drives `engine` to every state of `function` by the shortest path the function gives, sends it
/// every event there, and holds the configuration it reaches, and whether it took the event or
/// discarded it, to what the function says. The configuration is compared as a set of ids: a
/// state's ancestors are fixed by the document, so the set says what the ordered list says. Returns
/// how many pairs it checked, which a caller holds to the number of states times the number of
/// events.
func expectEveryPairAgrees<S: Hashable, E: Hashable>(
    _ function: TransitionFunctionView<S, E>, _ engine: EngineProbe
) throws -> Int {
    try engine.start()
    #expect(
        Set(function.configuration(function.initial)) == engine.active(),
        "the machine does not start where the function does")
    var pairs = 0
    for state in function.states {
        let name = function.stateName(state)
        guard let path = shortestPath(function, to: state) else {
            Issue.record("\(name) is not reached from the initial state")
            continue
        }
        for event in function.events {
            let pair = "(\(name), \(function.eventName(event)))"
            try engine.start()
            for taken in path {
                engine.send(function.eventName(taken))
            }
            pairs += 1
            if function.isFinal(state) {
                #expect(name == engine.terminal(), "\(pair): the path does not end the machine there")
                continue
            }
            #expect(
                Set(function.configuration(state)) == engine.active(),
                "\(pair): the path leaves the machine elsewhere")
            let discarded = engine.discarded()
            engine.send(function.eventName(event))
            if let to = function.transition(state, event) {
                #expect(
                    discarded == engine.discarded(),
                    "\(pair): the function takes it and the machine discarded it")
                if function.isFinal(to) {
                    #expect(
                        function.stateName(to) == engine.terminal(),
                        "\(pair): the machine did not end where the function says")
                } else {
                    #expect(
                        Set(function.configuration(to)) == engine.active(),
                        "\(pair): the machine is not where the function says")
                }
            } else {
                #expect(
                    discarded + 1 == engine.discarded(),
                    "\(pair): the function refuses it and the machine took it")
                #expect(
                    Set(function.configuration(state)) == engine.active(),
                    "\(pair): the machine moved on an event it discarded")
            }
        }
    }
    return pairs
}
