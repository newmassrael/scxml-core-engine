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

// §scxml-D-GlobalVariables `configuration`: the machine at rest, both regions of
// `run` active down to a leaf. §scxml-D-computeExitSet is defined over this and
// not over the hierarchy, so every exit-set assertion below has to name it.
const std::vector<S> kConfiguration = {S::Run, S::Drive, S::Running, S::Working, S::Budget, S::Within};

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

    const auto exitSet = Core::ParallelTransitionHelper::computeExitSet<S, EnumPolicy>(self, kConfiguration);

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
    deep.exitSet = CR::computeExitSet(S::Working, S::Judging, false, false, kConfiguration);

    CR::TransitionDescriptor self(S::Within, S::Within);
    self.exitSet = CR::computeExitSet(S::Within, S::Within, false, false, kConfiguration);

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

TEST(TransitionDomainSelfTransition, IntersectionAloneReachesTheAppendixVerdict) {
    // §scxml-D-removeConflictingTransitions tests ONE thing: whether the two
    // exit sets intersect. Three rules used to sit beside it in this engine — a
    // target/source equality check and a `<parallel>`-ancestor check each way —
    // and this is the case they existed for, so it is the case that says whether
    // removing them was safe.
    //
    // `working -> within` crosses from one region of `run` to the other. `run`
    // is a `<parallel>` and so not a domain candidate, so §scxml-D-findLCCA
    // walks past it: the domain is the `<scxml>` element, the transition tears
    // the whole `<parallel>` down, and the other region's transition must be
    // preempted by it.
    using CR = Core::ConflictResolutionHelper<EnumPolicy>;

    CR::TransitionDescriptor crossRegion(S::Working, S::Within);
    crossRegion.exitSet = CR::computeExitSet(S::Working, S::Within, false, false, kConfiguration);

    CR::TransitionDescriptor sibling(S::Within, S::Within);
    sibling.exitSet = CR::computeExitSet(S::Within, S::Within, false, false, kConfiguration);

    const auto selected = CR::removeConflictingTransitions({crossRegion, sibling});

    ASSERT_EQ(selected.size(), 1u)
        << "the sibling region's transition survived a transition that exits the whole `<parallel>`. "
           "§scxml-D-computeExitSet puts every active state below the `<scxml>` domain in the first "
           "transition's exit set, `within` among them, so the sets intersect and the appendix "
           "preempts the one whose source is not a descendant of the other's.";
    EXPECT_EQ(selected.front().source, S::Working);

    // Why the order had to be "exit set first, rules second". Assembled from one
    // region's own leaf-to-domain chain — what this engine did before — the same
    // transition's exit set is `{working, running, drive, run}` and the sibling's
    // is `{within}`. Those are DISJOINT, so the appendix rule alone acquits, and
    // the `<parallel>`-ancestor rule was the only thing reaching the right
    // verdict. Removing it while the set was still chain-shaped would have
    // silently un-preempted the sibling region.
    const std::vector<S> chainShaped = {S::Working, S::Running, S::Drive, S::Run};
    const std::vector<S> siblingChain = {S::Within};
    EXPECT_FALSE(Core::ConflictResolutionAlgorithms::hasIntersection(chainShaped, siblingChain))
        << "a chain-shaped exit set was supposed to be unable to reach the sibling region — if these "
           "now intersect, the premise this whole ordering rested on has changed";
    EXPECT_TRUE(Core::ConflictResolutionAlgorithms::hasIntersection(crossRegion.exitSet, sibling.exitSet))
        << "the appendix's configuration-shaped set is what makes the intersection true";
}

// ── §scxml-D-computeExitSet is read off the configuration ────────────────────
//
// The three below pin the procedure itself rather than an outcome. They were
// written while `removeConflictingTransitions` still applied a rule about
// `<parallel>` ancestors on top of the intersection, which reached the same
// outcome on its own and hid the set underneath. That rule is gone now — see
// `IntersectionAloneReachesTheAppendixVerdict` for why it could only go after
// this set was right. It stood in for the states this set was failing to
// name. Nothing can be said about removing it until the set underneath is the
// appendix's, and these are what say so.

TEST(TransitionDomainSelfTransition, ExitSetNamesTheSiblingRegionUnderTheDomain) {
    // A transition crossing from one region of `run` to the other. `run` is a
    // `<parallel>` and therefore not a domain candidate, so §scxml-D-findLCCA
    // walks past it and the domain is the `<scxml>` element — under which every
    // active state lies, `budget` and `within` included.
    Core::ParallelTransitionHelper::Transition<S> crossRegion;
    crossRegion.source = S::Working;
    crossRegion.targets = {S::Within};

    const auto exitSet = Core::ParallelTransitionHelper::computeExitSet<S, EnumPolicy>(crossRegion, kConfiguration);

    EXPECT_EQ(exitSet.count(S::Within), 1u)
        << "the sibling region's active leaf is missing from the exit set. §scxml-D-computeExitSet "
           "collects the ACTIVE states below the domain; walking the source's own ancestor chain "
           "instead never reaches a sibling region, so `removeConflictingTransitions` sees this "
           "transition and that region's transition on the same event as disjoint.";
    EXPECT_EQ(exitSet.count(S::Budget), 1u) << "the sibling region root is below the domain too";
    EXPECT_EQ(exitSet.count(S::Run), 1u) << "the `<parallel>` itself is below the `<scxml>` domain and exits with it";

    // The whole configuration is below the document root, so all six exit.
    EXPECT_EQ(exitSet.size(), kConfiguration.size())
        << "the domain is the `<scxml>` element, so §scxml-D-computeExitSet answers the whole "
           "configuration — no active state survives a transition whose domain is the document";
}

TEST(TransitionDomainSelfTransition, InternalTransitionToADescendantExitsThatDescendant) {
    // §scxml-D-getTransitionDomain hands an internal transition its own SOURCE
    // as the domain. That is not the same as exiting nothing: the source stays,
    // its active descendants do not.
    Core::ParallelTransitionHelper::Transition<S> internalDown;
    internalDown.source = S::Running;
    internalDown.targets = {S::Working};
    internalDown.isInternal = true;

    const auto exitSet = Core::ParallelTransitionHelper::computeExitSet<S, EnumPolicy>(internalDown, kConfiguration);

    EXPECT_EQ(exitSet.count(S::Working), 1u)
        << "an internal transition to a descendant answered an EMPTY exit set, so it conflicted with "
           "nothing — including a transition rooted at `working`, which it demonstrably exits";
    EXPECT_EQ(exitSet.count(S::Running), 0u) << "the domain itself is not exited; that is what makes it internal";
    EXPECT_EQ(exitSet.size(), 1u);
}

TEST(TransitionDomainSelfTransition, TheConflictSetAndTheMicrostepSetAreOneProcedure) {
    // The debt this test closes: the engine held TWO exit sets. The one
    // `removeConflictingTransitions` intersects walked the source's ancestor
    // chain; the one the microstep exits read the configuration. They answer
    // the same question and must not be able to disagree about it.
    const std::vector<Core::ParallelTransitionHelper::Transition<S>> transitions = {
        Core::ParallelTransitionHelper::Transition<S>(S::Working, {S::Within}),
    };

    const auto microstepSet =
        Core::ParallelTransitionHelper::computeStatesToExit<S, EnumPolicy>(transitions, kConfiguration);
    const auto conflictSet =
        Core::ParallelTransitionHelper::computeExitSet<S, EnumPolicy>(transitions.front(), kConfiguration);

    ASSERT_EQ(microstepSet.size(), conflictSet.size())
        << "the states this transition is judged to exit and the states it actually exits differ. "
           "Whichever is right, a resolver reading one while the microstep runs the other cannot be "
           "reasoned about.";
    for (const auto state : microstepSet) {
        EXPECT_EQ(conflictSet.count(state), 1u)
            << "state " << static_cast<int>(state) << " is exited by the microstep but absent from the set "
            << "conflict resolution intersects";
    }
}

}  // namespace SCE
