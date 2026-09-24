// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A parsed document as W3C SCXML Appendix D reads it, for the Interpreter.
//
// The procedures the two C++ engines share — `core/EntrySetHelper.h`,
// `core/MicrostepAlgorithms.h` — ask a document a fixed set of questions: a
// state's parent, its child states, its initial targets, a history's parent
// and default, the document order. The generated code answers them from the
// tables it was generated with. This answers them from an `SCXMLModel`, once,
// when the model is loaded, because the model's own accessors were built for
// other questions and answer these wrongly:
//
//  - `IStateNode::getChildren()` lists `<history>` pseudo-states among the
//    children; the appendix's child states are `<state>`, `<parallel>` and
//    `<final>` only.
//  - The model has no document order for states under a second top-level
//    state: the walk the Interpreter used started at the FIRST one.
//  - A `<history>`'s default was read as its first target, and a state's
//    default initial as its first child even when that child is a
//    `<history>`.
//
// Run-time state — the configuration, what a history recorded — is not here.
// The Interpreter's `MicrostepHost` joins this with those.

#pragma once

#include "core/EntrySetHelper.h"
#include "types.h"

#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

namespace SCE {

class IStateNode;
class SCXMLModel;

class InterpreterDocument {
public:
    using State = std::string;
    using History = std::string;
    using Target = Core::EntryTarget<State, History>;

    explicit InterpreterDocument(const SCXMLModel &model);

    // ── The questions `EntrySetAlgorithms` asks a document ───────────────
    // (all but `historyValue`, which is what the run recorded)

    std::optional<State> parentOf(const State &state) const;
    /// A `<state>` with child states. A `<parallel>` answers false: it is
    /// never a transition domain.
    bool isCompound(const State &state) const;
    bool isParallel(const State &state) const;
    /// `<state>`, `<parallel>` and `<final>` children, in document order.
    std::vector<State> childStates(const State &state) const;
    /// A compound state's initial targets as written — `<initial>`'s
    /// transition, else the `initial` attribute, else its first child state.
    std::vector<Target> initialTargets(const State &state) const;
    State historyParent(const History &history) const;
    std::vector<Target> historyDefaultTargets(const History &history) const;
    int documentOrder(const State &state) const;

    // ── What the Interpreter's host asks besides ─────────────────────────

    bool isFinal(const State &state) const;
    bool isHistory(const std::string &id) const;
    /// A target list as the document wrote it: each token a state or a
    /// `<history>`, kept apart because a history is never in a configuration.
    std::vector<Target> targetsOf(const std::vector<std::string> &ids) const;
    /// The targets of the document's own initial transition: the `<scxml>`
    /// element's `initial` attribute, else its first child state.
    const std::vector<Target> &documentInitialTargets() const;
    /// The parsed node for a state or `<history>` id, null for an unknown id.
    IStateNode *nodeOf(const std::string &id) const;

private:
    struct StateEntry {
        IStateNode *node = nullptr;
        std::optional<State> parent;
        Type type = Type::ATOMIC;
        int order = 0;
        std::vector<State> children;
        std::vector<Target> initial;
    };

    struct HistoryEntry {
        IStateNode *node = nullptr;
        State parent;
        std::vector<Target> defaults;
    };

    void indexSubtree(IStateNode *node, const std::optional<State> &parent, int &order);
    std::vector<Target> defaultInitialOf(const StateEntry &entry) const;

    std::unordered_map<State, StateEntry> states_;
    std::unordered_map<History, HistoryEntry> histories_;
    std::vector<Target> documentInitial_;
};

}  // namespace SCE
