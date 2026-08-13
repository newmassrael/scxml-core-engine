// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D — the domain of an external self-transition.
//
// `findLCCA` picks its answer out of `getProperAncestors(stateList.head())`,
// and §D defines proper ancestors as the ancestors *excluding the state
// itself*. So no transition, self or otherwise, can have its own source as its
// domain. `HierarchicalAlgorithms::findLCA` used to short-circuit on
// `state1 == state2` and return the state, which reads as the textbook "least
// common ancestor" but is not the procedure Appendix D specifies.
//
// The consequence lands in `computeExitSet`, which walks up from the source and
// stops at the state just below the domain. With the source named as its own
// domain that walk has no stopping point: it runs to the document root, and the
// exit set names every ancestor including an enclosing `<parallel>`.
// `removeConflictingTransitions` then treats a sibling region's transition on
// the same event as conflicting and preempts it, so that region is left with no
// active leaf and its transition content never runs.
//
// The helpers are instantiated twice in production — over `std::string` for the
// Interpreter and over an enum for AOT — so both instantiations are exercised
// here rather than one being assumed to follow from the other. The end-to-end
// pair is `ParallelRegionsTakeOwnTransitions{,Aot}Test.cpp`.

#include "core/ConflictResolutionHelper.h"
#include "core/HierarchicalStateHelper.h"
#include "core/ParallelTransitionHelper.h"
#include "gtest/gtest.h"

#include <algorithm>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace SCE {
namespace {

// run (parallel)
//   drive > running > { working, judging }
//   budget > within
//
// The asymmetry is the point: `within`'s transition is an external
// self-transition one level shallower than `working`'s.

// ── The Interpreter's instantiation: states are strings ──────────────────────

const std::map<std::string, std::string> kParents = {
    {"drive", "run"},       {"running", "drive"}, {"working", "running"},
    {"judging", "running"}, {"budget", "run"},    {"within", "budget"},
};

std::optional<std::string> parentOf(const std::string &id) {
    const auto it = kParents.find(id);
    if (it == kParents.end()) {
        return std::nullopt;
    }
    return it->second;
}

// ── The AOT instantiation: states are an enum ────────────────────────────────

enum class S : uint8_t { Run, Drive, Running, Working, Judging, Budget, Within };

struct EnumPolicy {
    using State = S;

    static std::optional<State> getParent(State s) {
        switch (s) {
        case S::Drive:
        case S::Budget:
            return S::Run;
        case S::Running:
            return S::Drive;
        case S::Working:
        case S::Judging:
            return S::Running;
        case S::Within:
            return S::Budget;
        case S::Run:
            break;
        }
        return std::nullopt;
    }

    static bool isCompoundState(State s) {
        return s == S::Run || s == S::Drive || s == S::Running || s == S::Budget;
    }

    static State getInitialChild(State s) {
        switch (s) {
        case S::Run:
            return S::Drive;
        case S::Drive:
            return S::Running;
        case S::Running:
            return S::Working;
        case S::Budget:
            return S::Within;
        default:
            return s;
        }
    }

    static bool isParallelState(State s) {
        return s == S::Run;
    }

    static std::vector<State> getParallelRegions(State s) {
        if (s == S::Run) {
            return {S::Drive, S::Budget};
        }
        return {};
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

    static bool isFinalState(State) {
        return false;
    }
};

}  // namespace

TEST(TransitionDomainSelfTransition, DomainOfASelfTransitionIsTheParent) {
    const auto stringDomain = Core::HierarchicalAlgorithms::findLCA<std::string>("within", "within", parentOf);
    ASSERT_TRUE(stringDomain.has_value());
    EXPECT_EQ(stringDomain.value(), "budget")
        << "W3C SCXML Appendix D `findLCCA` draws its candidates from `getProperAncestors`, which "
           "excludes the state itself, so a self-transition's domain is the parent. Answering "
           "`within` leaves `computeExitSet` with no stopping point on its climb.";

    const auto enumDomain = Core::HierarchicalStateHelper<EnumPolicy>::findLCA(S::Within, S::Within);
    ASSERT_TRUE(enumDomain.has_value());
    EXPECT_EQ(enumDomain.value(), S::Budget) << "the enum instantiation must answer the same as the string one — both "
                                                "engines reach the domain through this one procedure";
}

TEST(TransitionDomainSelfTransition, ExitSetOfASelfTransitionIsTheStateAlone) {
    Core::ParallelTransitionHelper::Transition<S> self;
    self.source = S::Within;
    self.targets = {S::Within};
    self.isInternal = false;
    self.isTargetless = false;

    const auto exitSet = Core::ParallelTransitionHelper::computeExitSet<S, EnumPolicy>(self);

    EXPECT_EQ(exitSet.size(), 1u) << "an external self-transition exits its source and re-enters it — nothing else";
    EXPECT_EQ(exitSet.count(S::Within), 1u);
    EXPECT_EQ(exitSet.count(S::Run), 0u)
        << "the exit set reached the enclosing `<parallel>`. Conflict resolution reads that as this "
           "transition tearing down every region, and preempts the other regions' transitions on the "
           "same event — the defect this test exists to hold shut.";
}

TEST(TransitionDomainSelfTransition, ASelfTransitionDoesNotPreemptASiblingRegion) {
    // W3C SCXML 3.4: both regions have an enabled transition on the same event
    // and both must survive conflict resolution. This is the path the generated
    // engines actually take, with exit sets from the helper pinned above.
    using CR = Core::ConflictResolutionHelper<EnumPolicy>;

    CR::TransitionDescriptor deep(S::Working, S::Judging);
    deep.exitSet = CR::computeExitSet(S::Working, S::Judging, false, false);

    CR::TransitionDescriptor self(S::Within, S::Within);
    self.exitSet = CR::computeExitSet(S::Within, S::Within, false, false);

    const auto selected = CR::removeConflictingTransitions({deep, self});

    ASSERT_EQ(selected.size(), 2u)
        << "one region's transition was preempted by the other's. The two sources are in different "
           "regions of `run`, so their exit sets are disjoint and W3C SCXML 3.4 keeps both.";

    const auto hasSource = [&selected](S id) {
        return std::any_of(selected.begin(), selected.end(),
                           [id](const CR::TransitionDescriptor &t) { return t.source == id; });
    };
    EXPECT_TRUE(hasSource(S::Working));
    EXPECT_TRUE(hasSource(S::Within));
}

}  // namespace SCE
