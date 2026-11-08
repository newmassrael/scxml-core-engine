// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The causal route of the C++ router (docs/adr/0085, sections 3.2 and 9.2):
// the routes the Rust router's tests hold (backends/rust/mesh/src/router.rs)
// and its three-copy world, the envelopes carried as the CBOR the codec
// writes, so the tag crosses the wire as keys 19 and 20. Each names the change
// that turns it red.

#include "mesh/CausalRoute.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <gtest/gtest.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <sstream>
#include <string>
#include <utility>
#include <variant>
#include <vector>

using SCE::Mesh::CausalDot;
using SCE::Mesh::CausalGroup;
using SCE::Mesh::CausalOffered;
using SCE::Mesh::CausalReceipt;
using SCE::Mesh::CausalReplica;
using SCE::Mesh::CausalRoute;
using SCE::Mesh::CausalRouteFault;
using SCE::Mesh::CausalStart;
using SCE::Mesh::CausalTag;
using SCE::Mesh::CausalVerdict;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::PatternKind;
using SCE::Mesh::PayloadCodec;
using SCE::Mesh::ReasonCode;

namespace {

/// A causal group of three, every copy one machine `list` on its own device;
/// the binding is the group's route.
CausalGroup people() {
    return CausalGroup{"people", 0, {"phone/list", "laptop/list", "tablet/list"}};
}

CausalReplica replica(std::uint8_t n) {
    CausalReplica out;
    out.id.fill(n);
    return out;
}

/// The copy of the list whose replica is `[n; 16]`, started.
CausalRoute copy(std::uint8_t n) {
    CausalRoute route("list", "custom_tcp", people());
    route.start(CausalStart::newIncarnation(replica(n)), 64, 100);
    return route;
}

std::array<std::uint8_t, 16> id(std::uint8_t n) {
    std::array<std::uint8_t, 16> out{};
    out.fill(n);
    return out;
}

/// The envelope a document's `<send event="edit">` to the binding becomes.
MeshEnvelope edit(const std::string &data, std::uint8_t n) {
    MeshEnvelope env;
    env.id = id(n);
    env.source = "list";
    env.type = "edit";
    env.pattern = PatternKind::FireForget;
    env.datacontenttype = data.empty() ? PayloadCodec::None : PayloadCodec::Json;
    env.data.assign(data.begin(), data.end());
    return env;
}

/// The dot an offer was accepted with.
std::optional<CausalDot> accepted(const std::optional<CausalOffered> &offered) {
    if (!offered) {
        return std::nullopt;
    }
    const auto *verdict = std::get_if<CausalVerdict>(&*offered);
    if (verdict == nullptr || verdict->kind != CausalVerdict::Kind::Accepted) {
        return std::nullopt;
    }
    return verdict->dot;
}

/// An envelope as the wire carries it: written by the codec, read back.
MeshEnvelope overTheWire(const MeshEnvelope &env) {
    const auto bytes = SCE::Mesh::encodeEnvelope(env);
    MeshEnvelope read;
    EXPECT_FALSE(bytes.empty());
    EXPECT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), read));
    return read;
}

std::vector<std::string> types(const std::vector<MeshEnvelope> &envelopes) {
    std::vector<std::string> out;
    for (const auto &env : envelopes) {
        out.push_back(env.type);
    }
    return out;
}

}  // namespace

// Write-ahead. Red if the route hands a causal change out when the layer
// tags it: the send then leaves before the journal commit.
TEST(CausalRouteTest, ACausalSendLeavesOnlyOnceItsJournalRecordIsCommitted) {
    auto phone = copy(1);
    auto laptop = copy(2);
    const auto dot = accepted(phone.offer(edit("{\"n\":1}", 9)));
    ASSERT_TRUE(dot.has_value());
    EXPECT_EQ(dot->seq, 1u);
    // Nothing of it is owed to a peer that asks before the commit.
    const auto early = laptop.announce(id(3), 0);
    ASSERT_TRUE(early.has_value());
    EXPECT_TRUE(phone.receive(overTheWire(*early)).send.empty());
    const auto released = phone.committed(*dot);
    ASSERT_EQ(released.size(), 1u);
    const MeshEnvelope sent = overTheWire(released[0]);
    EXPECT_EQ(types(laptop.receive(sent).deliver), std::vector<std::string>{"edit"});
    // The same change again is dropped by its dot, not delivered twice.
    EXPECT_TRUE(laptop.receive(sent).deliver.empty());
}

// A route the host has not started takes no change and judges nothing.
TEST(CausalRouteTest, ACausalRouteIsUsedOnlyOnceStarted) {
    CausalRoute route("list", "custom_tcp", people());
    EXPECT_FALSE(route.started());
    EXPECT_EQ(route.layer(), nullptr);
    EXPECT_FALSE(route.offer(edit("", 1)).has_value());
    EXPECT_FALSE(route.announce(id(1), 0).has_value());
    EXPECT_FALSE(route.announceDue(0));
    const auto tagged = copy(2).announce(id(2), 0);
    ASSERT_TRUE(tagged.has_value());
    const CausalReceipt receipt = route.receive(*tagged);
    ASSERT_TRUE(receipt.fault.has_value());
    EXPECT_EQ(receipt.fault->kind, CausalRouteFault::Kind::NotStarted);
    EXPECT_TRUE(receipt.deliver.empty());
}

// Repair, when a link comes back. Red if the vector a peer sends then is not
// answered with what it lacks (causal_lacks answers false): the lost change
// is then never sent again.
TEST(CausalRouteTest, AVectorSentWhenALinkComesBackRepairsWhatWasLost) {
    auto phone = copy(1);
    auto laptop = copy(2);
    const auto dot = accepted(phone.offer(edit("", 9)));
    ASSERT_TRUE(dot.has_value());
    (void)phone.committed(*dot);  // lost on the way
    const auto vector = laptop.announce(id(3), 10);
    ASSERT_TRUE(vector.has_value());
    EXPECT_EQ(vector->pattern, PatternKind::CausalVector);
    const CausalReceipt answer = phone.receive(overTheWire(*vector));
    EXPECT_TRUE(answer.deliver.empty()) << "a vector never reaches an engine";
    ASSERT_EQ(answer.send.size(), 1u);
    EXPECT_EQ(types(laptop.receive(overTheWire(answer.send[0])).deliver), std::vector<std::string>{"edit"});
}

// The period. Red if a route is never owed a vector again once it announced:
// a member that lost a change and whose links never drop is then never
// repaired.
TEST(CausalRouteTest, ACausalRouteIsOwedAVectorOnItsPeriod) {
    auto phone = copy(1);
    EXPECT_TRUE(phone.announceDue(0));
    ASSERT_TRUE(phone.announce(id(3), 0).has_value());
    EXPECT_FALSE(phone.announceDue(99));
    EXPECT_TRUE(phone.announceDue(100));
}

// The view. Red if the epoch check is removed: a change tagged with another
// epoch is then delivered rather than raised as row 18.
TEST(CausalRouteTest, AChangeOfAnotherViewIsRaisedAndNotDelivered) {
    auto laptop = copy(2);
    MeshEnvelope env = edit("", 5);
    CausalTag tag;
    tag.epoch = 1;
    tag.vector = {replica(1).entry(1)};
    ASSERT_TRUE(SCE::Mesh::stampCausalTag(env, tag));
    const CausalReceipt receipt = laptop.receive(overTheWire(env));
    EXPECT_TRUE(receipt.deliver.empty());
    ASSERT_TRUE(receipt.raise.has_value());
    EXPECT_EQ(receipt.raise->reason, ReasonCode::CausalViewMismatch);
    EXPECT_EQ(receipt.raise->source, std::optional<std::string>("list"));
    EXPECT_EQ(receipt.raise->epoch, std::optional<std::uint64_t>(1));
    EXPECT_EQ(receipt.raise->replica, std::optional<std::string>("01010101-0101-0101-0101-010101010101"));
}

// An envelope on a causal route without its tag cannot be placed. Red if it
// is delivered as a plain envelope rather than raised as row 4.
TEST(CausalRouteTest, AnEnvelopeWithoutItsTagIsRaisedAndNotDelivered) {
    auto laptop = copy(2);
    const CausalReceipt receipt = laptop.receive(overTheWire(edit("", 5)));
    EXPECT_TRUE(receipt.deliver.empty());
    ASSERT_TRUE(receipt.raise.has_value());
    EXPECT_EQ(receipt.raise->reason, ReasonCode::EnvelopeCorrupt);
    EXPECT_EQ(receipt.raise->transport, std::optional<std::string>("custom_tcp"));
    EXPECT_EQ(receipt.raise->codec, std::optional<std::string>("cbor"));
}

namespace {

/// A small deterministic generator for the world below.
struct Rng {
    std::uint64_t state;

    std::uint64_t step() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        return state;
    }

    std::size_t below(std::size_t n) {
        return static_cast<std::size_t>(step() % n);
    }

    bool chance(std::uint64_t percent) {
        return step() % 100 < percent;
    }
};

/// Three copies of the list, the bytes in flight to each, and what each has
/// taken in: per author, how many of its changes.
struct World {
    std::vector<CausalRoute> copies;
    std::vector<std::pair<std::size_t, std::vector<std::uint8_t>>> wire;
    std::array<std::array<std::uint32_t, 3>, 3> seen{};
    Rng rng;
    std::int64_t now = 0;
    bool lossy = true;

    std::array<std::uint8_t, 16> drawId() {
        return id(static_cast<std::uint8_t>(rng.step()));
    }

    /// Put what copy `at` sends on the wire to the two others, as the CBOR
    /// the codec writes, losing and repeating some of it.
    void transmit(std::size_t at, const MeshEnvelope &env) {
        const auto bytes = SCE::Mesh::encodeEnvelope(env);
        ASSERT_FALSE(bytes.empty());
        for (std::size_t to = 0; to < 3; ++to) {
            if (to == at) {
                continue;
            }
            if (lossy && rng.chance(30)) {
                continue;
            }
            if (lossy && rng.chance(10)) {
                wire.emplace_back(to, bytes);
            }
            wire.emplace_back(to, bytes);
        }
    }

    /// Copy `at` takes in the change whose data is `<author> <n> <seen of
    /// author 0> <of 1> <of 2>`, what its author had taken in when it made
    /// it. It must be the author's next change, and everything its author
    /// had taken in must be taken in here already.
    void take(std::size_t at, const MeshEnvelope &env) {
        std::istringstream text(std::string(env.data.begin(), env.data.end()));
        std::array<std::uint32_t, 5> fields{};
        for (auto &field : fields) {
            text >> field;
        }
        const std::size_t author = fields[0];
        ASSERT_LT(author, 3u);
        EXPECT_EQ(fields[1], seen[at][author] + 1) << "copy " << at << " takes in a change once and skips no gap";
        for (std::size_t other = 0; other < 3; ++other) {
            if (other != author) {
                EXPECT_GE(seen[at][other], fields[2 + other])
                    << "copy " << at << " takes in a change before a change it depends on";
            }
        }
        seen[at][author] = fields[1];
    }

    /// Copy `at` makes its next change, and its host commits the journal
    /// record of every accepted one at once.
    void make(std::size_t at) {
        const std::uint32_t n = seen[at][at] + 1;
        std::ostringstream data;
        data << at << ' ' << n << ' ' << seen[at][0] << ' ' << seen[at][1] << ' ' << seen[at][2];
        seen[at][at] = n;
        MeshEnvelope env = edit(data.str(), 0);
        env.id = drawId();
        const auto dot = accepted(copies[at].offer(env));
        ASSERT_TRUE(dot.has_value());
        for (const auto &released : copies[at].committed(*dot)) {
            transmit(at, released);
        }
    }

    void announce(std::size_t at) {
        const auto vector = copies[at].announce(drawId(), now);
        ASSERT_TRUE(vector.has_value());
        transmit(at, *vector);
    }

    void tick(std::size_t at) {
        if (copies[at].announceDue(now)) {
            announce(at);
        }
    }

    void deliverOne() {
        const std::size_t index = rng.below(wire.size());
        auto [to, bytes] = std::move(wire[index]);
        wire.erase(wire.begin() + static_cast<std::ptrdiff_t>(index));
        MeshEnvelope env;
        ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env));
        const CausalReceipt receipt = copies[to].receive(env);
        EXPECT_FALSE(receipt.raise.has_value());
        EXPECT_FALSE(receipt.fault.has_value());
        for (const auto &answer : receipt.send) {
            transmit(to, answer);
        }
        for (const auto &change : receipt.deliver) {
            take(to, change);
        }
    }
};

}  // namespace

// The world. Three copies on links that lose, duplicate and reorder take in
// every change exactly once, in an order that is a linear extension of what
// each change's author had taken in, with no gap skipped, and their logs drop
// to the stable cut once the links are quiet. Red if the hold branch
// delivers (the order check fails), or if repair is removed (a lost change
// then never arrives).
TEST(CausalRouteTest, ThreeCopiesThroughTheirRoutesTakeInEveryChangeOnceInCausalOrder) {
    constexpr std::uint32_t kChangesEach = 15;
    for (const std::uint64_t seed : {0x0BAD5EEDULL, 0x0CA5CADEULL, 0x00C0FFEEULL}) {
        World world;
        for (std::uint8_t n = 1; n <= 3; ++n) {
            world.copies.push_back(copy(n));
        }
        world.rng = Rng{seed};
        const auto busy = [&world]() {
            for (std::size_t at = 0; at < 3; ++at) {
                if (world.seen[at][at] < kChangesEach) {
                    return true;
                }
            }
            return !world.wire.empty();
        };
        while (busy()) {
            ++world.now;
            const std::size_t roll = world.rng.below(10);
            if (roll <= 2) {
                const std::size_t at = world.rng.below(3);
                if (world.seen[at][at] < kChangesEach) {
                    world.make(at);
                }
            } else if (roll == 3) {
                world.tick(world.rng.below(3));
            } else if (!world.wire.empty()) {
                world.deliverOne();
            }
            if (::testing::Test::HasFatalFailure()) {
                return;
            }
        }
        world.lossy = false;
        for (int round = 0; round < 20; ++round) {
            ++world.now;
            for (std::size_t at = 0; at < 3; ++at) {
                world.announce(at);
            }
            while (!world.wire.empty()) {
                world.deliverOne();
            }
        }
        for (std::size_t at = 0; at < 3; ++at) {
            const std::array<std::uint32_t, 3> all{kChangesEach, kChangesEach, kChangesEach};
            EXPECT_EQ(world.seen[at], all) << "seed " << seed << ": copy " << at << " takes in every change";
            const auto *layer = world.copies[at].layer();
            ASSERT_NE(layer, nullptr);
            EXPECT_EQ(layer->heldLen(), 0u) << "seed " << seed << ": nothing held";
            EXPECT_EQ(layer->logLen(), 0u) << "seed " << seed << ": the log is at the cut";
        }
    }
}
