// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D — isInFinalState, the predicate every done.state
// decision about a `<parallel>` asks.
//
// The appendix defines it recursively: a compound state is in a final state
// when one of its `<final>` children is active, a `<parallel>` when EVERY one
// of its child states is, and nothing else ever is. The helper both C++
// engines reached asked each region of a `<parallel>` for an active `<final>`
// child instead. That is the compound case applied to every region, and a
// region that is itself a `<parallel>` has no `<final>` child to find — so a
// `<parallel>` holding one could never complete, however far its regions got.
//
// Both instantiations are exercised: the lambda form the Interpreter uses
// (string ids) and the policy form the generated code calls
// (`ParallelCompletionHelper::areAllRegionsInFinal` over an enum). The
// document-level pair belongs with the seven-channel integration fixtures,
// once every backend runs the appendix's microstep.

#include "core/ParallelCompletionHelper.h"
#include "gtest/gtest.h"

#include <map>
#include <optional>
#include <string>
#include <vector>

namespace SCE {
namespace {

// run (parallel)
//   form > filling, formDone (final)
//   checks (parallel)
//     left  > leftPending,  leftDone (final)
//     right > rightPending, rightDone (final)
//
// `checks` is the region the old helper could never count: a `<parallel>`
// among the regions of a `<parallel>`.

// ── The Interpreter's instantiation: states are strings ──────────────────────

const std::map<std::string, std::string> kParents = {
    {"form", "run"},           {"filling", "form"},    {"formDone", "form"},    {"checks", "run"},
    {"left", "checks"},        {"right", "checks"},    {"leftPending", "left"}, {"leftDone", "left"},
    {"rightPending", "right"}, {"rightDone", "right"},
};

std::optional<std::string> parentOf(const std::string &id) {
    const auto it = kParents.find(id);
    if (it == kParents.end()) {
        return std::nullopt;
    }
    return it->second;
}

bool isParallel(const std::string &id) {
    return id == "run" || id == "checks";
}

std::vector<std::string> childStates(const std::string &id) {
    if (id == "run") {
        return {"form", "checks"};
    }
    if (id == "checks") {
        return {"left", "right"};
    }
    return {};
}

bool isFinal(const std::string &id) {
    return id == "formDone" || id == "leftDone" || id == "rightDone";
}

bool inFinal(const std::string &state, const std::vector<std::string> &configuration) {
    return Core::CompletionAlgorithms::isInFinalState(state, configuration, parentOf, isParallel, childStates, isFinal);
}

// ── The AOT instantiation: states are an enum ────────────────────────────────

enum class S : uint8_t {
    Run,
    Form,
    Filling,
    FormDone,
    Checks,
    Left,
    LeftPending,
    LeftDone,
    Right,
    RightPending,
    RightDone,
};

struct EnumPolicy {
    using State = S;

    static std::optional<State> getParent(State s) {
        switch (s) {
        case S::Form:
        case S::Checks:
            return S::Run;
        case S::Filling:
        case S::FormDone:
            return S::Form;
        case S::Left:
        case S::Right:
            return S::Checks;
        case S::LeftPending:
        case S::LeftDone:
            return S::Left;
        case S::RightPending:
        case S::RightDone:
            return S::Right;
        case S::Run:
            break;
        }
        return std::nullopt;
    }

    static bool isCompoundState(State s) {
        return s == S::Form || s == S::Left || s == S::Right;
    }

    static bool isParallelState(State s) {
        return s == S::Run || s == S::Checks;
    }

    static std::vector<State> getChildStates(State s) {
        switch (s) {
        case S::Run:
            return {S::Form, S::Checks};
        case S::Form:
            return {S::Filling, S::FormDone};
        case S::Checks:
            return {S::Left, S::Right};
        case S::Left:
            return {S::LeftPending, S::LeftDone};
        case S::Right:
            return {S::RightPending, S::RightDone};
        default:
            return {};
        }
    }

    static std::vector<State> getParallelRegions(State s) {
        return isParallelState(s) ? getChildStates(s) : std::vector<State>{};
    }

    static bool isDescendantOf(State candidate, State ancestor) {
        for (auto current = getParent(candidate); current.has_value(); current = getParent(current.value())) {
            if (current.value() == ancestor) {
                return true;
            }
        }
        return false;
    }

    static int getDocumentOrder(State s) {
        return static_cast<int>(s);
    }

    static bool isFinalState(State s) {
        return s == S::FormDone || s == S::LeftDone || s == S::RightDone;
    }
};

bool allRegionsFinal(S parallel, const std::vector<S> &configuration) {
    return Core::ParallelCompletionHelper::areAllRegionsInFinal<S, EnumPolicy>(parallel, configuration);
}

}  // namespace

TEST(CompletionAlgorithms, CompoundStateIsFinalWhenAFinalChildIsActive) {
    EXPECT_TRUE(inFinal("form", {"run", "form", "formDone", "checks", "left", "leftPending", "right", "rightPending"}));
    EXPECT_FALSE(inFinal("form", {"run", "form", "filling", "checks", "left", "leftPending", "right", "rightPending"}));
}

TEST(CompletionAlgorithms, NestedParallelIsFinalOnlyWhenEveryRegionIs) {
    EXPECT_TRUE(inFinal("checks", {"checks", "left", "leftDone", "right", "rightDone"}));
    EXPECT_FALSE(inFinal("checks", {"checks", "left", "leftDone", "right", "rightPending"}))
        << "one region still pending leaves the <parallel> short of final";
}

TEST(CompletionAlgorithms, AParallelRegionOfAParallelCounts) {
    const std::vector<std::string> complete = {"run",  "form",     "formDone", "checks",
                                               "left", "leftDone", "right",    "rightDone"};
    EXPECT_TRUE(inFinal("run", complete))
        << "`checks` has no <final> child of its own; it is final because both of its regions are";

    const std::vector<std::string> checksPending = {"run",  "form",     "formDone", "checks",
                                                    "left", "leftDone", "right",    "rightPending"};
    EXPECT_FALSE(inFinal("run", checksPending));
}

TEST(CompletionAlgorithms, NeitherAnAtomicStateNorAFinalElementIsInAFinalState) {
    const std::vector<std::string> configuration = {"run",  "form",        "formDone", "checks",
                                                    "left", "leftPending", "right",    "rightPending"};
    EXPECT_FALSE(inFinal("leftPending", configuration)) << "an atomic state has no <final> child to be active";
    EXPECT_FALSE(inFinal("formDone", configuration))
        << "being a <final> element is not being IN a final state — the predicate asks about children";
}

TEST(ParallelCompletionHelper, AParallelRegionOfAParallelCountsForTheGeneratedCode) {
    EXPECT_TRUE(allRegionsFinal(
        S::Run, {S::Run, S::Form, S::FormDone, S::Checks, S::Left, S::LeftDone, S::Right, S::RightDone}))
        << "the form the generated code calls must agree with the appendix on a nested <parallel>";
    EXPECT_FALSE(allRegionsFinal(
        S::Run, {S::Run, S::Form, S::FormDone, S::Checks, S::Left, S::LeftDone, S::Right, S::RightPending}));
    EXPECT_FALSE(allRegionsFinal(
        S::Run, {S::Run, S::Form, S::Filling, S::Checks, S::Left, S::LeftDone, S::Right, S::RightDone}));
}

TEST(ParallelCompletionHelper, OnlyAParallelIsAskedAboutItsRegions) {
    EXPECT_FALSE(allRegionsFinal(S::Form, {S::Run, S::Form, S::FormDone}))
        << "a compound state has no regions; the question does not apply to it";
}

}  // namespace SCE
