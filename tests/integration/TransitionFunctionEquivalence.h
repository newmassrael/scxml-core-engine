// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A machine held to the transition function generated beside it
// (docs/SCE_TRANSITION_FUNCTION.md) — the drive the C++ AOT and Interpreter
// drivers of the stems that declare a function share.
//
// The function is computed at build time by the selection rule an engine
// follows at run time, so the two are one fact only if every pair agrees, and
// this is what asks: a fresh machine per pair, driven to the state by the
// shortest event path the function itself gives, then sent the event. What the
// engine reaches, and whether it took the event or discarded it, must be what
// the function says.
//
// The function is a namespace, which a template cannot take, so a driver hands
// the drive a view of it (`SCE_TRANSITION_FUNCTION_VIEW`) and the questions the
// drive asks of its engine (`EngineProbe`): one for the generated machine
// (`aotProbe`) and one for the Interpreter (`interpreterProbe`).

#pragma once

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Tests {

/// A generated transition function, read through its accessors.
template <typename State, typename Event> struct TransitionFunctionView {
    std::vector<State> states;
    std::vector<Event> events;
    State initial;
    std::function<std::optional<State>(State, Event)> transition;
    std::function<bool(State)> isFinal;
    std::function<std::set<std::string>(State)> configuration;
    std::function<std::string(State)> stateName;
    std::function<std::string(Event)> eventName;
};

/// The view of the transition function generated in the namespace `ns`, whose
/// accessors every generated C++ function names alike.
#define SCE_TRANSITION_FUNCTION_VIEW(ns)                                                                               \
    ::SCE::Tests::TransitionFunctionView<ns::State, ns::Event> {                                                       \
        {ns::ALL_STATES.begin(), ns::ALL_STATES.end()}, {ns::ALL_EVENTS.begin(), ns::ALL_EVENTS.end()}, ns::INITIAL,   \
            [](ns::State state, ns::Event event) { return ns::transition(state, event); },                             \
            [](ns::State state) { return ns::isFinal(state); },                                                        \
            [](ns::State state) {                                                                                      \
                std::set<std::string> ids;                                                                             \
                for (std::string_view id : ns::configuration(state)) {                                                 \
                    ids.emplace(id);                                                                                   \
                }                                                                                                      \
                return ids;                                                                                            \
            },                                                                                                         \
            [](ns::State state) { return std::string(ns::getStateName(state)); },                                      \
            [](ns::Event event) { return std::string(ns::getEventName(event)); },                                      \
    }

/// What the drive asks of the engine it holds to a function: a fresh machine at
/// its initial configuration, an event delivered by name and run to
/// completion, the ids of the active states, the final the run ended in, and
/// how many events the engine has discarded.
struct EngineProbe {
    std::function<void()> start;
    std::function<void(const std::string &)> send;
    std::function<std::set<std::string>()> active;
    std::function<std::optional<std::string>()> terminal;
    std::function<std::uint64_t()> discarded;
};

/// The generated machine `SM`, held in `machine`, which the probe replaces on
/// every start.
template <typename SM> EngineProbe aotProbe(std::unique_ptr<SM> &machine) {
    EngineProbe probe;
    probe.start = [&machine] {
        machine = std::make_unique<SM>();
        if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
            machine->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(
                &::SCE::ScriptEngineProvider::getScriptEngine(), [](::SCE::IScriptEngine *) {}));
        }
        machine->initialize();
    };
    probe.send = [&machine](const std::string &name) {
        if (const auto event = SM::PolicyType::getEventFromName(name)) {
            machine->processEvent(*event);
        }
    };
    probe.active = [&machine] {
        std::set<std::string> ids;
        for (const auto state : machine->getActiveStates()) {
            ids.emplace(SM::PolicyType::getStateName(state));
        }
        return ids;
    };
    probe.terminal = [&machine]() -> std::optional<std::string> {
        if (const auto state = machine->terminalState()) {
            return std::string(SM::PolicyType::getStateName(*state));
        }
        return std::nullopt;
    };
    probe.discarded = [&machine]() -> std::uint64_t { return machine->discardedExternalEvents(); };
    return probe;
}

/// The Interpreter running `document`, held in `machine`, which the probe
/// replaces on every start.
inline EngineProbe interpreterProbe(std::shared_ptr<StateMachine> &machine, const std::string &document) {
    EngineProbe probe;
    probe.start = [&machine, document] {
        machine = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
        machine->setEventRaiser(std::make_shared<EventRaiserImpl>());
        ASSERT_TRUE(machine->loadSCXMLFromString(document));
        ASSERT_TRUE(machine->start());
    };
    probe.send = [&machine](const std::string &name) { machine->processEvent(name); };
    probe.active = [&machine] {
        const auto states = machine->getActiveStates();
        return std::set<std::string>(states.begin(), states.end());
    };
    probe.terminal = [&machine] { return machine->terminalState(); };
    probe.discarded = [&machine]() -> std::uint64_t {
        return static_cast<std::uint64_t>(machine->getStatistics().failedTransitions);
    };
    return probe;
}

/// The shortest sequence of events that takes `function` from its initial
/// state to `target`, read from the function alone; nothing when no sequence
/// does.
template <typename State, typename Event>
std::optional<std::vector<Event>> shortestPath(const TransitionFunctionView<State, Event> &function, State target) {
    std::vector<std::pair<State, std::vector<Event>>> frontier{{function.initial, {}}};
    std::vector<State> seen{function.initial};
    for (std::size_t next = 0; next < frontier.size(); ++next) {
        const auto [state, path] = frontier[next];
        if (state == target) {
            return path;
        }
        for (const Event event : function.events) {
            const auto to = function.transition(state, event);
            if (to && std::find(seen.begin(), seen.end(), *to) == seen.end()) {
                seen.push_back(*to);
                auto longer = path;
                longer.push_back(event);
                frontier.emplace_back(*to, std::move(longer));
            }
        }
    }
    return std::nullopt;
}

/// Drive `engine` to every state of `function` by the shortest path the
/// function gives, send it every event there, and expect the configuration it
/// reaches, and whether it took the event or discarded it, to be what the
/// function says. The configuration is compared as a set of ids: a state's
/// ancestors are fixed by the document, so the set says what the ordered list
/// says. Returns how many pairs it checked, which a caller holds to the number
/// of states times the number of events.
template <typename State, typename Event>
std::size_t expectEveryPairAgrees(const TransitionFunctionView<State, Event> &function, const EngineProbe &engine) {
    engine.start();
    EXPECT_EQ(engine.active(), function.configuration(function.initial))
        << "the machine does not start where the function does";
    std::size_t pairs = 0;
    for (const State state : function.states) {
        const std::string name = function.stateName(state);
        const auto path = shortestPath(function, state);
        if (!path) {
            ADD_FAILURE() << name << " is not reached from the initial state";
            continue;
        }
        for (const Event event : function.events) {
            const std::string pair = "(" + name + ", " + function.eventName(event) + ")";
            engine.start();
            for (const Event taken : *path) {
                engine.send(function.eventName(taken));
            }
            ++pairs;
            if (function.isFinal(state)) {
                EXPECT_EQ(engine.terminal(), std::optional<std::string>(name))
                    << pair << ": the path does not end the machine there";
                continue;
            }
            EXPECT_EQ(engine.active(), function.configuration(state))
                << pair << ": the path leaves the machine elsewhere";
            const std::uint64_t discarded = engine.discarded();
            engine.send(function.eventName(event));
            const auto to = function.transition(state, event);
            if (!to) {
                EXPECT_EQ(engine.discarded(), discarded + 1)
                    << pair << ": the function refuses it and the machine took it";
                EXPECT_EQ(engine.active(), function.configuration(state))
                    << pair << ": the machine moved on an event it discarded";
            } else {
                EXPECT_EQ(engine.discarded(), discarded)
                    << pair << ": the function takes it and the machine discarded it";
                if (function.isFinal(*to)) {
                    EXPECT_EQ(engine.terminal(), std::optional<std::string>(function.stateName(*to)))
                        << pair << ": the machine did not end where the function says";
                } else {
                    EXPECT_EQ(engine.active(), function.configuration(*to))
                        << pair << ": the machine is not where the function says";
                }
            }
        }
    }
    return pairs;
}

}  // namespace SCE::Tests
