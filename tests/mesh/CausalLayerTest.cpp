// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The tests of docs/adr/0085, section 9.2 for the C++ core's causal layer,
// the same tests the Rust reference holds (backends/rust/mesh/src/causal.rs).
// Each names the change to the rule, or to the layer, that turns it red; the
// rule's own mutants are run by
// sce-build/tests/mutations/a_cpp_causal_route_delivers_each_change_once_ctest.cases.

#include "mesh/CausalLayer.h"
#include "mesh/CausalRules.h"

#include <gtest/gtest.h>

#include <cstdint>
#include <optional>
#include <set>
#include <utility>
#include <variant>
#include <vector>

using SCE::Mesh::CausalDot;
using SCE::Mesh::CausalEntry;
using SCE::Mesh::CausalGroup;
using SCE::Mesh::CausalJournalRecord;
using SCE::Mesh::CausalLayer;
using SCE::Mesh::CausalOffered;
using SCE::Mesh::CausalOfferError;
using SCE::Mesh::CausalRefusal;
using SCE::Mesh::CausalRejected;
using SCE::Mesh::CausalReplica;
using SCE::Mesh::CausalStart;
using SCE::Mesh::CausalTag;
using SCE::Mesh::CausalVerdict;

namespace {

/// A message as the tests carry it: the tag it was sent with and a name.
struct Msg {
    CausalTag tag;
    std::uint32_t name = 0;
};

using Layer = CausalLayer<Msg>;
using Names = std::vector<std::uint32_t>;

CausalGroup view() {
    return CausalGroup{"people", 0, {"phone/list", "laptop/list", "tablet/list"}};
}

CausalGroup pairView() {
    return CausalGroup{"pair", 0, {"phone/list", "laptop/list"}};
}

CausalReplica replica(std::uint8_t n) {
    CausalReplica out;
    out.id.fill(n);
    return out;
}

Layer layer(const CausalGroup &group, std::uint8_t n, std::size_t room) {
    return Layer(group, CausalStart::newIncarnation(replica(n)), room);
}

/// The offer of the change `name`, whose message carries the tag it is given.
CausalOffered offerNamed(Layer &at, std::uint32_t name) {
    return at.offer([name](const CausalTag &tag) { return std::optional<Msg>(Msg{tag, name}); });
}

/// The dot an offer was accepted with, or a failure.
std::optional<CausalDot> acceptedDot(const CausalOffered &offered) {
    const auto *verdict = std::get_if<CausalVerdict>(&offered);
    if (verdict == nullptr || verdict->kind != CausalVerdict::Kind::Accepted) {
        return std::nullopt;
    }
    return verdict->dot;
}

/// Offer the change `name` and commit its journal record: the message that
/// leaves for the group.
Msg make(Layer &at, std::uint32_t name) {
    const auto dot = acceptedDot(offerNamed(at, name));
    if (!dot) {
        ADD_FAILURE() << "expected the change " << name << " accepted";
        return Msg{};
    }
    auto sent = at.committed(*dot);
    EXPECT_EQ(sent.size(), 1u) << "one change released per commit";
    return sent.empty() ? Msg{} : sent.front();
}

/// The sender's own entry of a message's tag: the dot of its change.
CausalEntry own(const Msg &message) {
    if (message.tag.vector.empty()) {
        ADD_FAILURE() << "a tag carries its sender's entry first";
        return CausalEntry{};
    }
    return message.tag.vector.front();
}

/// Give `message` to `at`: the names it delivers, in order.
Names give(Layer &at, const Msg &message) {
    auto taken = at.receive(message.tag, message);
    const auto *delivered = std::get_if<std::vector<Msg>>(&taken);
    if (delivered == nullptr) {
        ADD_FAILURE() << "a change of the view is taken in";
        return {};
    }
    Names names;
    for (const auto &m : *delivered) {
        names.push_back(m.name);
    }
    return names;
}

/// Give `to` the `causalVector` of `from`: the messages `to` answers with.
std::vector<Msg> exchange(const Layer &from, Layer &to) {
    auto taken = to.receiveSummary(from.summary());
    auto *answer = std::get_if<std::vector<Msg>>(&taken);
    if (answer == nullptr) {
        ADD_FAILURE() << "a vector of the view is taken in";
        return {};
    }
    return std::move(*answer);
}

std::uint64_t column(const Layer &at, const CausalReplica &of) {
    for (const auto &entry : at.delivered()) {
        if (CausalReplica::of(entry) == of) {
            return entry.seq;
        }
    }
    return 0;
}

std::vector<CausalEntry> cutOf(const Layer &at) {
    const auto cut = at.stableCut();
    const auto *entries = std::get_if<std::vector<CausalEntry>>(&cut);
    if (entries == nullptr) {
        ADD_FAILURE() << "the cut is computed";
        return {};
    }
    return *entries;
}

}  // namespace

// Drop. Red if `dot.seq <= seen` becomes `<` in causal_admit: the change
// given again is then held, not dropped.
TEST(CausalLayerTest, AChangeGivenAgainIsDroppedAndNotHeld) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    const Msg c1 = make(p, 1);
    EXPECT_EQ(give(q, c1), Names{1});
    EXPECT_EQ(give(q, c1), Names{});
    EXPECT_EQ(q.heldLen(), 0u);
    EXPECT_EQ(column(q, replica(1)), 1u);
}

// Deliver. Red if the delivered vector is not raised by the change
// delivered (CausalLayer::deliver): the author's next change is then held.
TEST(CausalLayerTest, TheNextChangeIsDeliveredOnceAndRaisesItsAuthorsColumn) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    const Msg c1 = make(p, 1);
    const Msg c2 = make(p, 2);
    EXPECT_EQ(give(q, c1), Names{1});
    EXPECT_EQ(column(q, replica(1)), 1u);
    EXPECT_EQ(give(q, c2), Names{2});
    EXPECT_EQ(column(q, replica(1)), 2u);
}

// Hold, a gap of the author. Red if `dot.seq === seen + 1` is weakened to
// `dot.seq > seen`: the change past the gap is then delivered first.
TEST(CausalLayerTest, AChangePastAGapOfItsAuthorIsHeldUntilTheGapFills) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    const Msg c1 = make(p, 1);
    const Msg c2 = make(p, 2);
    EXPECT_EQ(give(q, c2), Names{});
    EXPECT_EQ(q.heldLen(), 1u);
    EXPECT_EQ(give(q, c1), (Names{1, 2}));
    EXPECT_EQ(q.heldLen(), 0u);
}

// Hold, an uncovered context. Red if the check `C[b] <= D[b]` is removed from
// causal_admit: the change is then delivered before the change of another
// author it depends on.
TEST(CausalLayerTest, AChangeWhoseContextIsNotDeliveredIsHeldUntilItIs) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    auto r = layer(view(), 3, 16);
    const Msg c1 = make(p, 1);
    EXPECT_EQ(give(r, c1), Names{1});
    const Msg d1 = make(r, 10);
    EXPECT_EQ(give(q, d1), Names{});
    EXPECT_EQ(give(q, c1), (Names{1, 10}));
}

// No gap timeout. Red if anything other than the change before it releases
// a held change, as a gap timeout that fast-forwards the author's column
// would.
TEST(CausalLayerTest, AHeldChangeStaysHeldHoweverManyVectorsPass) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    auto r = layer(view(), 3, 16);
    const Msg c1 = make(p, 1);
    const Msg c2 = make(p, 2);
    EXPECT_EQ(give(q, c2), Names{});
    for (int i = 0; i < 1000; ++i) {
        exchange(r, q);
        exchange(q, r);
    }
    EXPECT_EQ(q.heldLen(), 1u);
    EXPECT_EQ(column(q, replica(1)), 0u);
    EXPECT_EQ(give(q, c1), (Names{1, 2}));
}

// Repair by vector. Red if what a vector lacks is computed as empty
// (causal_lacks answers false): the lost change is then never sent again.
TEST(CausalLayerTest, ALostChangeIsReceivedAfterOneVectorExchange) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    (void)make(p, 1);
    const auto answer = exchange(q, p);
    ASSERT_EQ(answer.size(), 1u);
    EXPECT_EQ(give(q, answer[0]), Names{1});
}

// The stable cut. Red if the minimum becomes a maximum in causal_cut (the
// cut is then {p:2}), or if a member whose vector is unknown is left out (the
// cut is then not empty before the third member is heard from).
TEST(CausalLayerTest, TheStableCutIsTheMinimumOverTheViewWithAnUnknownMemberAtZero) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    auto r = layer(view(), 3, 16);
    const Msg c1 = make(p, 1);
    const Msg c2 = make(p, 2);
    give(q, c1);
    give(q, c2);
    exchange(q, p);
    EXPECT_EQ(cutOf(p), std::vector<CausalEntry>{});
    give(r, c1);
    exchange(r, p);
    EXPECT_EQ(cutOf(p), std::vector<CausalEntry>{replica(1).entry(1)});
}

// The mark. Red if a received vector is not recorded as its member's latest
// (CausalLayer::noteKnown in receiveSummary): members that never wrote then
// never move the cut, and the log keeps the change.
TEST(CausalLayerTest, MembersThatNeverWroteMoveTheCutByTheirVectorsAlone) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    auto r = layer(view(), 3, 16);
    const Msg c1 = make(p, 1);
    give(q, c1);
    give(r, c1);
    EXPECT_EQ(p.logLen(), 1u);
    exchange(q, p);
    exchange(r, p);
    EXPECT_EQ(p.logLen(), 0u);
}

// Retention. Red if the log is pruned at the copy's own delivered vector
// instead of the cut (CausalLayer::prune): the change a member still lacks is
// then gone, and its repair is not answered.
TEST(CausalLayerTest, TheLogDropsAChangeAtTheCutAndAnswersARepairBeforeIt) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    auto r = layer(view(), 3, 16);
    const Msg c1 = make(p, 1);
    const Msg c2 = make(p, 2);
    give(q, c1);
    give(q, c2);
    give(r, c1);
    exchange(q, p);
    const auto answer = exchange(r, p);
    EXPECT_EQ(p.logLen(), 1u) << "c1 is stable, c2 is not";
    ASSERT_EQ(answer.size(), 1u);
    EXPECT_EQ(give(r, answer[0]), Names{2});
}

// The verdict. Red if the number is taken before the refusal
// (CausalLayer::offer): the next accepted change then leaves a gap, and the
// peer holds it.
TEST(CausalLayerTest, AnOfferPastTheRoomIsRefusedAndTakesNoNumber) {
    auto p = layer(pairView(), 1, 1);
    auto q = layer(pairView(), 2, 16);
    const Msg c1 = make(p, 1);
    const CausalOffered refused = offerNamed(p, 99);
    const auto *verdict = std::get_if<CausalVerdict>(&refused);
    ASSERT_NE(verdict, nullptr);
    EXPECT_EQ(*verdict, CausalVerdict::refused(CausalRefusal::NoRoomForHistory));
    EXPECT_EQ(p.ownSeq(), 1u);
    EXPECT_EQ(give(q, c1), Names{1});
    exchange(q, p);
    EXPECT_EQ(p.logLen(), 0u) << "stability frees room";
    const Msg c2 = make(p, 2);
    EXPECT_EQ(own(c2), replica(1).entry(2));
    EXPECT_EQ(give(q, c2), Names{2});
}

// A message that cannot be made takes no number either. Red if the
// delivered vector is raised before `build` answers.
TEST(CausalLayerTest, AnOfferWhoseMessageCannotBeMadeTakesNoNumber) {
    auto p = layer(view(), 1, 16);
    const CausalOffered failed = p.offer([](const CausalTag &) { return std::optional<Msg>{}; });
    const auto *error = std::get_if<CausalOfferError>(&failed);
    ASSERT_NE(error, nullptr);
    EXPECT_EQ(error->kind, CausalOfferError::Kind::Build);
    EXPECT_EQ(p.ownSeq(), 0u);
    EXPECT_EQ(p.logLen(), 0u);
    EXPECT_EQ(own(make(p, 1)), replica(1).entry(1));
}

// Write-ahead. Red if the layer releases a change when it tags it: a repair
// asked before the journal commit is then answered with it.
TEST(CausalLayerTest, AnAcceptedChangeLeavesOnlyOnceItsJournalRecordIsCommitted) {
    auto p = layer(view(), 1, 16);
    const auto q = layer(view(), 2, 16);
    const auto dot = acceptedDot(offerNamed(p, 1));
    ASSERT_TRUE(dot.has_value());
    EXPECT_EQ(dot->seq, 1u);
    EXPECT_TRUE(exchange(q, p).empty());
    EXPECT_EQ(p.committed(*dot).size(), 1u);
    EXPECT_TRUE(p.committed(*dot).empty()) << "a change is released once";
    EXPECT_EQ(exchange(q, p).size(), 1u);
}

// Incarnation. Red if a copy without its journal reuses its previous
// identity (CausalStart::fromJournal), or continues from a number any peer
// reports: its next change then carries the dot of the change an offline
// member holds.
TEST(CausalLayerTest, ACopyWithoutItsJournalBeginsANewIncarnation) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    const Msg c1 = make(p, 1);
    const Msg c2 = make(p, 2);
    give(q, c1);
    // c2 reached only the tablet, which is offline now. The phone is
    // restored from a save taken after c1: it cannot know its highest. A copy
    // whose journal is its own continues from it.
    const CausalJournalRecord record{replica(1), {replica(1).entry(2)}};
    EXPECT_EQ(CausalStart::fromJournal(record, replica(9)), CausalStart::journal(record));
    Layer resumed(view(), CausalStart::journal(record), 16);
    EXPECT_EQ(own(make(resumed, 4)), replica(1).entry(3));
    Layer restored(view(), CausalStart::fromJournal(std::nullopt, replica(9)), 16);
    EXPECT_EQ(restored.replica(), replica(9));
    const Msg next = make(restored, 3);
    std::set<std::vector<std::uint64_t>> dots;
    for (const Msg *m : {&c1, &c2, &next}) {
        const CausalEntry dot = own(*m);
        dots.insert({dot.hi, dot.lo, dot.seq});
    }
    EXPECT_EQ(dots.size(), 3u) << "no dot names two contents";
    EXPECT_EQ(own(next), replica(9).entry(1));
    // Until view changes land, the laptop, which knows the phone's first
    // incarnation and the tablet, does not deliver the second.
    const auto r = layer(view(), 3, 16);
    exchange(r, q);
    auto taken = q.receive(next.tag, next);
    const auto *rejected = std::get_if<CausalRejected>(&taken);
    ASSERT_NE(rejected, nullptr);
    EXPECT_EQ(*rejected, CausalRejected::notInView(replica(9)));
}

// The view. Red if the epoch check is removed (CausalLayer::senderOf): a
// change of another epoch is then delivered.
TEST(CausalLayerTest, AChangeOfAnotherEpochIsRejectedAndNotDelivered) {
    auto p = layer(view(), 1, 16);
    auto q = layer(view(), 2, 16);
    Msg change = make(p, 1);
    change.tag.epoch = 1;
    auto taken = q.receive(change.tag, change);
    const auto *rejected = std::get_if<CausalRejected>(&taken);
    ASSERT_NE(rejected, nullptr);
    EXPECT_EQ(*rejected, CausalRejected::otherEpoch(1));
    CausalTag vector;
    vector.epoch = 1;
    vector.vector = {replica(1).entry(0)};
    auto summary = q.receiveSummary(vector);
    const auto *refused = std::get_if<CausalRejected>(&summary);
    ASSERT_NE(refused, nullptr);
    EXPECT_EQ(*refused, CausalRejected::otherEpoch(1));
    EXPECT_EQ(column(q, replica(1)), 0u);
}

// A replica is its UUID as two big-endian halves, as causal_entry.scxml's own
// example spells it. Red if a half is read little-endian, which would order
// two identities otherwise than their bytes.
TEST(CausalLayerTest, AReplicaIsItsUuidAsTwoBigEndianHalves) {
    CausalReplica r;
    for (std::uint8_t i = 0; i < 16; ++i) {
        r.id[i] = i;
    }
    const CausalEntry third = r.entry(3);
    EXPECT_EQ(third.hi, 0x0001020304050607ULL);
    EXPECT_EQ(third.lo, 0x08090a0b0c0d0e0fULL);
    EXPECT_EQ(third.seq, 3u);
    EXPECT_EQ(CausalReplica::of(third), r);
}

// The bound of a vector, which a generated C++ list does not hold itself.
// Red if CausalRules.cpp does not refuse a merge past 64 entries.
TEST(CausalLayerTest, AMergePastTheBoundOfAViewIsRefused) {
    std::vector<CausalEntry> left;
    std::vector<CausalEntry> right;
    for (std::uint64_t n = 1; n <= 33; ++n) {
        left.push_back(CausalEntry{n, 0, 1});
    }
    for (std::uint64_t n = 34; n <= 65; ++n) {
        right.push_back(CausalEntry{n, 0, 1});
    }
    const auto past = SCE::Mesh::causalRaise(left, right);
    ASSERT_FALSE(past.ok());
    EXPECT_EQ(past.error(), SCE::Forge::AlgorithmError::CapacityExceeded);
    right.pop_back();
    const auto within = SCE::Mesh::causalRaise(left, right);
    ASSERT_TRUE(within.ok());
    EXPECT_EQ(within.value().size(), SCE::Mesh::kCausalVectorBound);
}

// The tag an envelope carries. Red if one of keys 19 and 20 without the
// other is taken as a tag rather than refused as malformed.
TEST(CausalLayerTest, AnEnvelopeCarriesItsTagWholeOrNotAtAll) {
    SCE::Mesh::MeshEnvelope env;
    const auto none = SCE::Mesh::envelopeCausalTag(env);
    ASSERT_TRUE(std::holds_alternative<std::optional<CausalTag>>(none));
    EXPECT_FALSE(std::get<std::optional<CausalTag>>(none).has_value());

    CausalTag tag;
    tag.epoch = 0;
    tag.vector = {replica(1).entry(1)};
    ASSERT_TRUE(SCE::Mesh::stampCausalTag(env, tag));
    const auto whole = SCE::Mesh::envelopeCausalTag(env);
    ASSERT_TRUE(std::holds_alternative<std::optional<CausalTag>>(whole));
    EXPECT_EQ(std::get<std::optional<CausalTag>>(whole), std::optional<CausalTag>(tag));

    env.causal_vector.reset();
    const auto half = SCE::Mesh::envelopeCausalTag(env);
    ASSERT_TRUE(std::holds_alternative<CausalRejected>(half));
    EXPECT_EQ(std::get<CausalRejected>(half), CausalRejected::malformed());

    CausalTag past;
    past.vector = std::vector<CausalEntry>(SCE::Mesh::kCausalVectorBound + 1);
    SCE::Mesh::MeshEnvelope untouched;
    EXPECT_FALSE(SCE::Mesh::stampCausalTag(untouched, past));
    EXPECT_FALSE(untouched.causal_epoch.has_value());
}
