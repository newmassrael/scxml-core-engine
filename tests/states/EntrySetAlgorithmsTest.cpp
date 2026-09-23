// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D — what a microstep enters.
//
// `EntrySetAlgorithms` transcribes computeEntrySet and the procedures it calls.
// Each case below states the entry set the appendix computes for one
// transition, worked by hand from the pseudo-code, and asks the helper for it.
// Three answers are the reason the helper exists and are pinned by name:
//
//   * a target SET enters every target, and a `<parallel>` region no target
//     descends into still gets its default;
//   * a compound state entered only as an ANCESTOR of a deeper target is not
//     in statesForDefaultEntry, so its initial transition content does not run;
//   * a history that recorded several regions restores all of them.
//
// The helper is instantiated twice in production — over `std::string` for the
// Interpreter and over enums for the generated code — so both are compiled and
// exercised here.

#include "core/EntrySetHelper.h"
#include "gtest/gtest.h"

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace SCE {
namespace {

using Core::EntrySetAlgorithms;

// ── The Interpreter's instantiation: states and histories are strings ─────────
//
// <scxml initial="s">
//   <state id="s" initial="p">
//     <parallel id="p">
//       <state id="r1" initial="a1"> a1 a2 </state>
//       <state id="r2" initial="b1"> b1 b2
//         <history id="hb" type="shallow"> -> b2 </history> </state>
//       <state id="r3"> c1 c2 </state>                    (no initial: c1)
//     </parallel>
//     <state id="q" initial="q1"> q1 <final id="qf"/> </state>
//     <history id="hs" type="deep"> -> "a2 b2" </history>
//     <state id="m" initial="m1a m2b">
//       <parallel id="mp">
//         <state id="m1"> m1a m1b </state>
//         <state id="m2"> m2a m2b </state>
//       </parallel>
//     </state>
//   </state>
//   <state id="t"/>
// </scxml>

class TableDoc {
public:
    using State = std::string;
    using History = std::string;
    using Target = Core::EntryTarget<State, History>;

    TableDoc() {
        add("s", "", Kind::Compound, {"p"});
        add("p", "s", Kind::Parallel);
        add("r1", "p", Kind::Compound, {"a1"});
        add("a1", "r1", Kind::Atomic);
        add("a2", "r1", Kind::Atomic);
        add("r2", "p", Kind::Compound, {"b1"});
        add("b1", "r2", Kind::Atomic);
        add("b2", "r2", Kind::Atomic);
        add("r3", "p", Kind::Compound);
        add("c1", "r3", Kind::Atomic);
        add("c2", "r3", Kind::Atomic);
        add("q", "s", Kind::Compound, {"q1"});
        add("q1", "q", Kind::Atomic);
        add("qf", "q", Kind::Final);
        add("m", "s", Kind::Compound, {"m1a", "m2b"});
        add("mp", "m", Kind::Parallel);
        add("m1", "mp", Kind::Compound);
        add("m1a", "m1", Kind::Atomic);
        add("m1b", "m1", Kind::Atomic);
        add("m2", "mp", Kind::Compound);
        add("m2a", "m2", Kind::Atomic);
        add("m2b", "m2", Kind::Atomic);
        add("t", "", Kind::Atomic);
        histories_["hb"] = HistoryRow{"r2", {"b2"}};
        histories_["hs"] = HistoryRow{"s", {"a2", "b2"}};
    }

    void record(const History &history, std::vector<State> states) {
        recorded_[history] = std::move(states);
    }

    /// A target token as a document names it: a history id is a history.
    Target token(const std::string &id) const {
        return histories_.count(id) != 0 ? Target::onHistory(id) : Target::onState(id);
    }

    std::vector<Target> tokens(const std::vector<std::string> &ids) const {
        std::vector<Target> out;
        for (const auto &id : ids) {
            out.push_back(token(id));
        }
        return out;
    }

    std::optional<State> parentOf(const State &s) const {
        const auto &parent = rows_.at(s).parent;
        return parent.empty() ? std::nullopt : std::optional<State>(parent);
    }

    bool isCompound(const State &s) const {
        return rows_.at(s).kind == Kind::Compound;
    }

    bool isParallel(const State &s) const {
        return rows_.at(s).kind == Kind::Parallel;
    }

    std::vector<State> childStates(const State &s) const {
        std::vector<State> children;
        for (const auto &id : order_) {
            if (rows_.at(id).parent == s) {
                children.push_back(id);
            }
        }
        return children;
    }

    std::vector<Target> initialTargets(const State &s) const {
        const auto &written = rows_.at(s).initial;
        if (!written.empty()) {
            return tokens(written);
        }
        return {Target::onState(childStates(s).front())};
    }

    State historyParent(const History &h) const {
        return histories_.at(h).parent;
    }

    std::optional<std::vector<State>> historyValue(const History &h) const {
        const auto it = recorded_.find(h);
        return it == recorded_.end() ? std::nullopt : std::optional<std::vector<State>>(it->second);
    }

    std::vector<Target> historyDefaultTargets(const History &h) const {
        return tokens(histories_.at(h).defaults);
    }

    int documentOrder(const State &s) const {
        return rows_.at(s).order;
    }

private:
    enum class Kind { Atomic, Compound, Parallel, Final };

    struct Row {
        std::string parent;
        Kind kind;
        int order;
        std::vector<std::string> initial;
    };

    struct HistoryRow {
        std::string parent;
        std::vector<std::string> defaults;
    };

    void add(const std::string &id, const std::string &parent, Kind kind, std::vector<std::string> initial = {}) {
        rows_[id] = Row{parent, kind, static_cast<int>(order_.size()), std::move(initial)};
        order_.push_back(id);
    }

    std::map<std::string, Row> rows_;
    std::vector<std::string> order_;
    std::map<std::string, HistoryRow> histories_;
    std::map<std::string, std::vector<std::string>> recorded_;
};

using StringTransition = Core::EntryTransition<std::string, std::string>;

StringTransition transition(const TableDoc &doc, std::optional<std::string> source, std::vector<std::string> targets,
                            bool internal = false) {
    return StringTransition{std::move(source), doc.tokens(targets), internal};
}

std::vector<std::string> sorted(std::vector<std::string> states) {
    std::sort(states.begin(), states.end());
    return states;
}

TEST(EntrySetAlgorithms, TheInitialConfigurationEntersEveryRegionByDefault) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::nullopt, {"s"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"s", "p", "r1", "a1", "r2", "b1", "r3", "c1"}));
    // A `<parallel>` is not compound, so it has no initial transition to run.
    EXPECT_EQ(sorted(entry.statesForDefaultEntry), (std::vector<std::string>{"r1", "r2", "r3", "s"}));
    EXPECT_TRUE(entry.defaultHistoryContent.empty());
}

TEST(EntrySetAlgorithms, ATargetSetEntersEveryTargetAndDefaultsTheRegionNoTargetReaches) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("t"), {"a2", "b2"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"s", "p", "r1", "a2", "r2", "b2", "r3", "c1"}));
    // `s`, `r1` and `r2` are entered as ANCESTORS of a named target, so their
    // initial transitions do not run; only `r3`, reached by nobody, defaults.
    EXPECT_EQ(entry.statesForDefaultEntry, (std::vector<std::string>{"r3"}));
    EXPECT_FALSE(entry.isDefaultEntry("s"));
    EXPECT_TRUE(entry.isDefaultEntry("r3"));
}

TEST(EntrySetAlgorithms, AnUnrecordedShallowHistoryTakesItsDefaultAndOwesItsContentToItsParent) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("a1"), {"hb"})}, doc);

    // The domain is `s`: the history stands for `b2`, and the least compound
    // ancestor of `a1` and `b2` walks past the `<parallel>`.
    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"p", "r1", "a1", "r2", "b2", "r3", "c1"}));
    EXPECT_EQ(sorted(entry.statesForDefaultEntry), (std::vector<std::string>{"r1", "r3"}));
    EXPECT_EQ(entry.defaultHistoryContentOf("r2"), std::optional<std::string>("hb"));
    EXPECT_EQ(entry.defaultHistoryContentOf("p"), std::nullopt);
}

TEST(EntrySetAlgorithms, ARecordedShallowHistoryRestoresWhatItRecordedAndOwesNoContent) {
    TableDoc doc;
    doc.record("hb", {"b1"});
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("a1"), {"hb"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"p", "r1", "a1", "r2", "b1", "r3", "c1"}));
    EXPECT_TRUE(entry.defaultHistoryContent.empty());
}

TEST(EntrySetAlgorithms, ADeepHistoryThatRecordedEveryRegionRestoresAllOfThem) {
    TableDoc doc;
    doc.record("hs", {"a2", "b1", "c2"});
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("t"), {"hs"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"s", "p", "r1", "a2", "r2", "b1", "r3", "c2"}));
    // Nothing is entered by default: every compound state on the way is an
    // ancestor of a recorded state.
    EXPECT_TRUE(entry.statesForDefaultEntry.empty());
    EXPECT_TRUE(entry.defaultHistoryContent.empty());
}

TEST(EntrySetAlgorithms, AnUnrecordedDeepHistoryWithAMultiStateDefaultEntersTheWholeSet) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("t"), {"hs"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"s", "p", "r1", "a2", "r2", "b2", "r3", "c1"}));
    EXPECT_EQ(entry.statesForDefaultEntry, (std::vector<std::string>{"r3"}));
    EXPECT_EQ(entry.defaultHistoryContentOf("s"), std::optional<std::string>("hs"));
}

TEST(EntrySetAlgorithms, AnInternalTransitionToADescendantLeavesItsSourceOutOfTheSet) {
    TableDoc doc;
    const auto entry =
        EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("s"), {"q1"}, /*internal=*/true)}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"q", "q1"}));
    EXPECT_TRUE(entry.statesForDefaultEntry.empty());
}

TEST(EntrySetAlgorithms, AnExternalTransitionToADescendantReentersItsSource) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("s"), {"q1"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"s", "q", "q1"}));
    EXPECT_TRUE(entry.statesForDefaultEntry.empty());
}

TEST(EntrySetAlgorithms, AnExternalTransitionOnARegionRootReentersEverySiblingRegion) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("r1"), {"a2"})}, doc);

    // A `<parallel>` is never a domain, so the domain is `s`, and the sibling
    // regions are exited and entered again at their defaults.
    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"p", "r1", "a2", "r2", "b1", "r3", "c1"}));
    EXPECT_EQ(sorted(entry.statesForDefaultEntry), (std::vector<std::string>{"r2", "r3"}));
}

TEST(EntrySetAlgorithms, AnInternalTransitionOnARegionRootStaysInsideItsRegion) {
    TableDoc doc;
    const auto entry =
        EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("r1"), {"a2"}, /*internal=*/true)}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"a2"}));
}

TEST(EntrySetAlgorithms, TransitionsOfOneMicrostepEnterInDocumentOrder) {
    TableDoc doc;
    // Selected second-region first, as a set is free to be; entry order is
    // document order all the same.
    const auto entry = EntrySetAlgorithms::computeEntrySet(
        {transition(doc, std::string("b1"), {"b2"}), transition(doc, std::string("a1"), {"a2"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"a2", "b2"}));
}

TEST(EntrySetAlgorithms, ADeepMultiTargetInitialDefaultsOnlyTheStateThatNamesIt) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("t"), {"m"})}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<std::string>{"s", "m", "mp", "m1", "m1a", "m2", "m2b"}));
    // `m` is the target and defaults; `m1` and `m2` are ancestors of its
    // initial targets, and `s` an ancestor of the target itself.
    EXPECT_EQ(entry.statesForDefaultEntry, (std::vector<std::string>{"m"}));
}

TEST(EntrySetAlgorithms, ATargetlessTransitionEntersNothing) {
    TableDoc doc;
    const auto entry = EntrySetAlgorithms::computeEntrySet({transition(doc, std::string("a1"), {})}, doc);

    EXPECT_TRUE(entry.statesToEnter.empty());
    EXPECT_TRUE(entry.statesForDefaultEntry.empty());
}

TEST(EntrySetAlgorithms, TheDomainIsAskedOfTheStatesAHistoryStandsFor) {
    TableDoc doc;
    // Unrecorded, `hs` stands for "a2 b2", whose least compound ancestor with
    // `t` is the document itself.
    EXPECT_EQ(EntrySetAlgorithms::getTransitionDomain(transition(doc, std::string("t"), {"hs"}), doc), std::nullopt);
    // Recorded inside `q`, an internal transition on `q` keeps `q` as its domain.
    doc.record("hs", {"q1"});
    EXPECT_EQ(EntrySetAlgorithms::getEffectiveTargetStates(doc.tokens({"hs"}), doc), (std::vector<std::string>{"q1"}));
    EXPECT_EQ(EntrySetAlgorithms::getTransitionDomain(transition(doc, std::string("q"), {"hs"}, true), doc),
              std::optional<std::string>("q"));
}

// ── The generated code's instantiation: states and histories are enums ────────
//
// <scxml initial="top">
//   <state id="top">
//     <parallel id="par">
//       <state id="left"> la lb </state>
//       <state id="right"> ra rb <history id="hr"> -> rb </history> </state>
//     </parallel>
//   </state>
//   <state id="out"/>
// </scxml>

enum class S : uint8_t { Top, Par, Left, La, Lb, Right, Ra, Rb, Out };
enum class H : uint8_t { Hr };

struct EnumDoc {
    using State = S;
    using History = H;
    using Target = Core::EntryTarget<S, H>;

    std::optional<std::vector<S>> recordedHr;

    std::optional<S> parentOf(S s) const {
        switch (s) {
        case S::Par:
            return S::Top;
        case S::Left:
        case S::Right:
            return S::Par;
        case S::La:
        case S::Lb:
            return S::Left;
        case S::Ra:
        case S::Rb:
            return S::Right;
        default:
            return std::nullopt;
        }
    }

    bool isCompound(S s) const {
        return s == S::Top || s == S::Left || s == S::Right;
    }

    bool isParallel(S s) const {
        return s == S::Par;
    }

    std::vector<S> childStates(S s) const {
        switch (s) {
        case S::Top:
            return {S::Par};
        case S::Par:
            return {S::Left, S::Right};
        case S::Left:
            return {S::La, S::Lb};
        case S::Right:
            return {S::Ra, S::Rb};
        default:
            return {};
        }
    }

    std::vector<Target> initialTargets(S s) const {
        return {Target::onState(childStates(s).front())};
    }

    S historyParent(H) const {
        return S::Right;
    }

    std::optional<std::vector<S>> historyValue(H) const {
        return recordedHr;
    }

    std::vector<Target> historyDefaultTargets(H) const {
        return {Target::onState(S::Rb)};
    }

    int documentOrder(S s) const {
        return static_cast<int>(s);
    }
};

using EnumTransition = Core::EntryTransition<S, H>;

TEST(EntrySetAlgorithmsEnum, ATargetSetAcrossRegionsEntersBothTargets) {
    const EnumDoc doc;
    const EnumTransition t{S::Out, {EnumDoc::Target::onState(S::Lb), EnumDoc::Target::onState(S::Rb)}, false};
    const auto entry = EntrySetAlgorithms::computeEntrySet({t}, doc);

    EXPECT_EQ(entry.statesToEnter, (std::vector<S>{S::Top, S::Par, S::Left, S::Lb, S::Right, S::Rb}));
    EXPECT_TRUE(entry.statesForDefaultEntry.empty());
}

TEST(EntrySetAlgorithmsEnum, AHistoryTargetIsDereferencedThroughTheDoc) {
    EnumDoc doc;
    const EnumTransition t{S::La, {EnumDoc::Target::onHistory(H::Hr)}, false};

    const auto defaulted = EntrySetAlgorithms::computeEntrySet({t}, doc);
    EXPECT_EQ(defaulted.statesToEnter, (std::vector<S>{S::Par, S::Left, S::La, S::Right, S::Rb}));
    EXPECT_EQ(defaulted.defaultHistoryContentOf(S::Right), std::optional<H>(H::Hr));

    doc.recordedHr = std::vector<S>{S::Ra};
    const auto restored = EntrySetAlgorithms::computeEntrySet({t}, doc);
    EXPECT_EQ(restored.statesToEnter, (std::vector<S>{S::Par, S::Left, S::La, S::Right, S::Ra}));
    EXPECT_TRUE(restored.defaultHistoryContent.empty());
}

}  // namespace
}  // namespace SCE
