// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh CausalLayer — one copy's side of a causal broadcast group.
//
// The causal route's delivery layer (docs/adr/0085, sections 3, 3.2, 3.3 and
// 4.2; SCE_MESH.md §mesh-10.12), the C++ core's counterpart of the Rust
// reference (backends/rust/mesh/src/causal.rs), with the same behaviour.
//
// A change this copy makes is offered here and answered with a verdict:
// Accepted with the dot the layer tagged it with, or Refused when the log has
// no room for one more change not yet stable. An accepted change is released
// to the peers only once the host reports its journal record committed
// (`committed`), so a number no peer has seen is the only one a crash can
// lose. A change that arrives is delivered once, and only after every change
// in its context; one that is not yet deliverable is held, and a gap is
// repaired by exchanging vectors, never skipped: there is no gap timeout.
// Every change sent or delivered is kept until the stable cut passes it.
//
// Every decision is a call into the generated standard rules
// (CausalRules.h): whether a change is dropped, delivered or held
// (causal_admit), the merge of two vectors (causal_raise), whether a vector
// lacks a change (causal_lacks) and the stable cut (causal_cut). What this
// class adds is the state those rules are applied to and the messages
// themselves while they are held or logged. It is generic over the message
// (`M`), which CausalRoute instantiates with a stamped MeshEnvelope, and
// transport-free, like the rest of the core. It is not synchronised: its
// owner serialises the calls.
//
// The author of a dot is a replica: one incarnation of one member, a UUID v7
// the host draws when an incarnation begins and keeps in the copy's journal
// beside the highest number it used (CausalStart). A copy that cannot know
// its highest number begins a new incarnation, never one asked of its peers.

#pragma once

#include "mesh/CausalEntry.h"
#include "mesh/CausalRules.h"
#include "mesh/MeshEnvelope.h"

#include <sce/forge/algorithm.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace SCE::Mesh {

/// A replica identity: one incarnation of one member of a group, a 16-byte
/// UUID v7 unique within the group and never used again.
struct CausalReplica {
    std::array<std::uint8_t, 16> id{};

    /// This replica's entry at `seq`: its identity as the rules hold it, its
    /// first eight bytes and its last eight, big-endian.
    [[nodiscard]] CausalEntry entry(std::uint64_t seq) const noexcept {
        CausalEntry out;
        out.hi = half(0);
        out.lo = half(8);
        out.seq = seq;
        return out;
    }

    /// The replica an entry names.
    [[nodiscard]] static CausalReplica of(const CausalEntry &entry) noexcept {
        CausalReplica replica;
        put(replica.id, 0, entry.hi);
        put(replica.id, 8, entry.lo);
        return replica;
    }

    bool operator==(const CausalReplica &other) const noexcept {
        return id == other.id;
    }

    bool operator!=(const CausalReplica &other) const noexcept {
        return id != other.id;
    }

    bool operator<(const CausalReplica &other) const noexcept {
        return id < other.id;
    }

private:
    [[nodiscard]] std::uint64_t half(std::size_t at) const noexcept {
        std::uint64_t value = 0;
        for (std::size_t i = 0; i < 8; ++i) {
            value = (value << 8) | id[at + i];
        }
        return value;
    }

    static void put(std::array<std::uint8_t, 16> &id, std::size_t at, std::uint64_t value) noexcept {
        for (std::size_t i = 0; i < 8; ++i) {
            id[at + i] = static_cast<std::uint8_t>(value >> (56 - 8 * i));
        }
    }
};

/// The dot a change is tagged with: its author and its number, from 1 with
/// no gap.
struct CausalDot {
    CausalReplica replica{};
    std::uint64_t seq = 0;

    /// This dot as an entry of a vector.
    [[nodiscard]] CausalEntry entry() const noexcept {
        return replica.entry(seq);
    }

    /// The dot an entry names.
    [[nodiscard]] static CausalDot of(const CausalEntry &entry) noexcept {
        CausalDot dot;
        dot.replica = CausalReplica::of(entry);
        dot.seq = entry.seq;
        return dot;
    }

    bool operator==(const CausalDot &other) const noexcept {
        return replica == other.replica && seq == other.seq;
    }

    bool operator!=(const CausalDot &other) const noexcept {
        return !(*this == other);
    }
};

/// A causal binding's group as the deployment gives it: its name and the view
/// of epoch 0, the only view until view changes land (docs/adr/0085, section
/// 3.3). Each entry of `view` names one instance as `<device>/<machine>`.
struct CausalGroup {
    std::string name;
    std::uint64_t epoch = 0;
    std::vector<std::string> view;
};

/// What a tagged envelope or a `causalVector` carries: the view's epoch
/// (envelope key 19) and a vector (key 20) whose first entry is its sender's
/// own. On a change that entry is the change's dot and the rest its context;
/// on a `causalVector` the vector is the sender's delivered vector.
struct CausalTag {
    std::uint64_t epoch = 0;
    std::vector<CausalEntry> vector;

    bool operator==(const CausalTag &other) const noexcept {
        return epoch == other.epoch && vector == other.vector;
    }

    bool operator!=(const CausalTag &other) const noexcept {
        return !(*this == other);
    }
};

/// What the host reads from the copy's own journal when a copy starts: the
/// replica identity of its incarnation and its delivered vector, its own
/// entry being the highest number it used.
struct CausalJournalRecord {
    CausalReplica replica{};
    std::vector<CausalEntry> delivered;

    bool operator==(const CausalJournalRecord &other) const noexcept {
        return replica == other.replica && delivered == other.delivered;
    }
};

/// How an incarnation starts (docs/adr/0085, section 3.3.2).
class CausalStart {
public:
    /// A normal restart: the copy's journal says who it is and what it
    /// delivered, and it continues after its highest number.
    [[nodiscard]] static CausalStart journal(CausalJournalRecord record) {
        return CausalStart(true, record.replica, std::move(record.delivered));
    }

    /// A copy that cannot know its own highest number begins a new
    /// incarnation, with an identity the host has just drawn.
    [[nodiscard]] static CausalStart newIncarnation(CausalReplica fresh) {
        return CausalStart(false, fresh, {});
    }

    /// The start of a copy whose journal gave `record`, or none: a copy
    /// restored from an older save, or set up again from its seed, has no
    /// journal record of its own, and begins under `fresh` instead. It never
    /// asks its peers for its number: the change with the highest one may be
    /// held only by a member that is offline.
    [[nodiscard]] static CausalStart fromJournal(std::optional<CausalJournalRecord> record, CausalReplica fresh) {
        if (record) {
            return journal(std::move(*record));
        }
        return newIncarnation(fresh);
    }

    /// Whether the copy continues its journal's incarnation.
    [[nodiscard]] bool continuesJournal() const noexcept {
        return continues_;
    }

    [[nodiscard]] const CausalReplica &replica() const noexcept {
        return replica_;
    }

    [[nodiscard]] const std::vector<CausalEntry> &delivered() const noexcept {
        return delivered_;
    }

    bool operator==(const CausalStart &other) const noexcept {
        return continues_ == other.continues_ && replica_ == other.replica_ && delivered_ == other.delivered_;
    }

private:
    CausalStart(bool continues, CausalReplica replica, std::vector<CausalEntry> delivered)
        : continues_(continues), replica_(replica), delivered_(std::move(delivered)) {}

    bool continues_;
    CausalReplica replica_;
    std::vector<CausalEntry> delivered_;
};

/// Why an offer was refused.
enum class CausalRefusal : std::uint8_t {
    /// The log holds as many changes not yet stable as the deployment allows;
    /// the list tells it as `noRoomForHistory`, and stability frees room.
    NoRoomForHistory,
    /// The incarnation used every number a dot can carry.
    NumbersExhausted,
};

/// The layer's answer to a change its copy offers (docs/adr/0093, section
/// 3.1).
struct CausalVerdict {
    enum class Kind : std::uint8_t {
        /// Tagged with `dot`; the host records it in the journal, then calls
        /// `committed`.
        Accepted,
        /// Not tagged: it took no number, and no copy takes it in.
        Refused,
    };

    Kind kind = Kind::Refused;
    CausalDot dot{};
    CausalRefusal refusal = CausalRefusal::NoRoomForHistory;

    [[nodiscard]] static CausalVerdict accepted(CausalDot dot) noexcept {
        CausalVerdict verdict;
        verdict.kind = Kind::Accepted;
        verdict.dot = dot;
        return verdict;
    }

    [[nodiscard]] static CausalVerdict refused(CausalRefusal why) noexcept {
        CausalVerdict verdict;
        verdict.kind = Kind::Refused;
        verdict.refusal = why;
        return verdict;
    }

    bool operator==(const CausalVerdict &other) const noexcept {
        if (kind != other.kind) {
            return false;
        }
        return kind == Kind::Accepted ? dot == other.dot : refusal == other.refusal;
    }
};

/// Why an offer could not be answered.
struct CausalOfferError {
    enum class Kind : std::uint8_t {
        /// The message for the tagged change could not be made; no number was
        /// taken.
        Build,
        /// A rule refused its input.
        Rule,
    };

    Kind kind = Kind::Build;
    ::SCE::Forge::AlgorithmError rule = ::SCE::Forge::AlgorithmError::Overflow;

    [[nodiscard]] static CausalOfferError build() noexcept {
        return CausalOfferError{};
    }

    [[nodiscard]] static CausalOfferError ruleFailed(::SCE::Forge::AlgorithmError error) noexcept {
        CausalOfferError failure;
        failure.kind = Kind::Rule;
        failure.rule = error;
        return failure;
    }
};

/// Why something that arrived was not taken in.
struct CausalRejected {
    enum class Kind : std::uint8_t {
        /// Tagged with another epoch than the current view's: raised, not
        /// delivered.
        OtherEpoch,
        /// Tagged by a replica the view does not hold: raised, not delivered.
        NotInView,
        /// A vector with no sender entry, or more entries than a view holds.
        Malformed,
        /// A rule refused its input.
        Rule,
    };

    Kind kind = Kind::Malformed;
    /// The epoch of an OtherEpoch.
    std::uint64_t epoch = 0;
    /// The replica of a NotInView.
    CausalReplica replica{};
    /// The failure of a Rule.
    ::SCE::Forge::AlgorithmError rule = ::SCE::Forge::AlgorithmError::Overflow;

    [[nodiscard]] static CausalRejected otherEpoch(std::uint64_t epoch) noexcept {
        CausalRejected rejected;
        rejected.kind = Kind::OtherEpoch;
        rejected.epoch = epoch;
        return rejected;
    }

    [[nodiscard]] static CausalRejected notInView(CausalReplica replica) noexcept {
        CausalRejected rejected;
        rejected.kind = Kind::NotInView;
        rejected.replica = replica;
        return rejected;
    }

    [[nodiscard]] static CausalRejected malformed() noexcept {
        return CausalRejected{};
    }

    [[nodiscard]] static CausalRejected ruleFailed(::SCE::Forge::AlgorithmError error) noexcept {
        CausalRejected rejected;
        rejected.kind = Kind::Rule;
        rejected.rule = error;
        return rejected;
    }

    bool operator==(const CausalRejected &other) const noexcept {
        return kind == other.kind && epoch == other.epoch && replica == other.replica && rule == other.rule;
    }
};

/// An offer's answer: the verdict, or why there is none.
using CausalOffered = std::variant<CausalVerdict, CausalOfferError>;

/// What a change or a vector that arrived lets this copy send or deliver, in
/// order, or why it was not taken in.
template <typename M> using CausalTaken = std::variant<std::vector<M>, CausalRejected>;

/// The stable cut, or why a rule could not compute it.
using CausalCutResult = std::variant<std::vector<CausalEntry>, CausalRejected>;

/// One copy's causal delivery layer for one group.
template <typename M> class CausalLayer {
public:
    /// A layer for `group`, started as `start` says, whose log holds at most
    /// `room` changes not yet stable.
    CausalLayer(CausalGroup group, const CausalStart &start, std::size_t room)
        : group_(std::move(group)), replica_(start.replica()), delivered_(start.delivered()), room_(room) {
        // The copy's own entry is always present, at 0 before its first
        // change, so its vector says who sent it.
        const bool named = std::any_of(delivered_.begin(), delivered_.end(), [this](const CausalEntry &entry) {
            return CausalReplica::of(entry) == replica_;
        });
        if (!named) {
            delivered_.insert(delivered_.begin(), replica_.entry(0));
        }
    }

    [[nodiscard]] const CausalGroup &group() const noexcept {
        return group_;
    }

    /// The replica identity this incarnation tags its changes with.
    [[nodiscard]] const CausalReplica &replica() const noexcept {
        return replica_;
    }

    /// What this copy delivered, its own entry among them.
    [[nodiscard]] const std::vector<CausalEntry> &delivered() const noexcept {
        return delivered_;
    }

    /// How many changes are held until they are deliverable.
    [[nodiscard]] std::size_t heldLen() const noexcept {
        return held_.size();
    }

    /// How many changes the log keeps until the stable cut passes them.
    [[nodiscard]] std::size_t logLen() const noexcept {
        return log_.size();
    }

    /// The highest number this incarnation used.
    [[nodiscard]] std::uint64_t ownSeq() const noexcept {
        return seen(delivered_, replica_);
    }

    /// Offer a change this copy made. When the log has room, the layer tags
    /// it with the next dot and its delivered vector, asks `build` (called
    /// with the CausalTag, answering a `std::optional<M>`) for the message
    /// that carries that tag, counts the change delivered here and logs it;
    /// the message is released by `committed`. When the log has no room the
    /// change is refused and takes no number, and when `build` answers
    /// nothing, nothing is taken either.
    template <typename Build> CausalOffered offer(Build &&build) {
        // §mesh-10.12, offer and verdict: a log with no room for one more
        // change not yet stable refuses it, and a refused change takes no
        // number; an accepted one counts as delivered here once it is tagged.
        if (log_.size() >= room_) {
            return CausalVerdict::refused(CausalRefusal::NoRoomForHistory);
        }
        const std::uint64_t own = ownSeq();
        if (own == std::numeric_limits<std::uint64_t>::max()) {
            return CausalVerdict::refused(CausalRefusal::NumbersExhausted);
        }
        const std::uint64_t seq = own + 1;
        const CausalEntry dot = replica_.entry(seq);
        auto raised = causalRaise(delivered_, std::vector<CausalEntry>{dot});
        if (!raised.ok()) {
            return CausalOfferError::ruleFailed(raised.error());
        }
        const CausalTag tag = tagOf(dot, raised.value());
        std::optional<M> message = std::forward<Build>(build)(tag);
        if (!message) {
            return CausalOfferError::build();
        }
        delivered_ = raised.value();
        log_.push_back(Logged{dot, std::move(*message), false});
        CausalDot accepted;
        accepted.replica = replica_;
        accepted.seq = seq;
        return CausalVerdict::accepted(accepted);
    }

    /// The host committed the journal record of `dot`: every accepted change
    /// of this incarnation up to it may now leave, in the order it was
    /// accepted. These are the messages to send to the group.
    std::vector<M> committed(const CausalDot &dot) {
        std::vector<M> out;
        if (dot.replica != replica_) {
            return out;
        }
        for (auto &logged : log_) {
            if (!logged.released && CausalReplica::of(logged.dot) == replica_ && logged.dot.seq <= dot.seq) {
                logged.released = true;
                out.push_back(logged.message);
            }
        }
        return out;
    }

    /// A change that arrived, tagged `tag`: what it lets this copy deliver,
    /// in a causal order. A change delivered already is dropped, the next
    /// change of its author whose context is delivered is delivered with
    /// every held change that then follows, and any other is held. An echo
    /// of this copy's own change is dropped.
    CausalTaken<M> receive(const CausalTag &tag, M message) {
        CausalEntry sender;
        if (auto rejected = senderOf(tag, sender)) {
            return *rejected;
        }
        const CausalReplica replica = CausalReplica::of(sender);
        if (replica == replica_) {
            return std::vector<M>{};
        }
        if (auto rejected = admitMember(replica)) {
            return *rejected;
        }
        if (auto rejected = noteKnown(replica, tag.vector)) {
            return *rejected;
        }
        // §mesh-10.12, the rule: a change at or below its author's column is
        // dropped, the next one whose context is delivered is delivered with
        // every held change that then follows, and any other is held.
        const auto admission = causalAdmit(delivered_, sender, tag.vector);
        if (!admission.ok()) {
            return CausalRejected::ruleFailed(admission.error());
        }
        std::vector<M> out;
        switch (admission.value()) {
        case CausalAdmission::Drop:
            break;
        case CausalAdmission::Hold: {
            const bool already =
                std::any_of(held_.begin(), held_.end(), [&sender](const Held &held) { return held.dot == sender; });
            // A hold past the log's room is let go: the change is still the
            // sender's, and a later vector exchange brings it again.
            if (!already && held_.size() < room_) {
                held_.push_back(Held{sender, tag.vector, std::move(message)});
            }
            break;
        }
        case CausalAdmission::Deliver:
            if (auto rejected = deliver(sender, std::move(message), out)) {
                return *rejected;
            }
            if (auto rejected = drainHeld(out)) {
                return *rejected;
            }
            if (auto rejected = prune()) {
                return *rejected;
            }
            break;
        }
        return out;
    }

    /// This copy's `causalVector`: the epoch and its delivered vector, its
    /// own entry first. Sent on a period and whenever a link comes back.
    [[nodiscard]] CausalTag summary() const {
        return tagOf(replica_.entry(ownSeq()), delivered_);
    }

    /// A member's `causalVector`: recorded as that member's latest known
    /// vector (its mark, which moves the stable cut), and answered with
    /// every released change of the log the vector lacks, in the order the
    /// log holds them (a causal order), as they were sent.
    CausalTaken<M> receiveSummary(const CausalTag &tag) {
        CausalEntry sender;
        if (auto rejected = senderOf(tag, sender)) {
            return *rejected;
        }
        const CausalReplica replica = CausalReplica::of(sender);
        if (replica == replica_) {
            return std::vector<M>{};
        }
        if (auto rejected = admitMember(replica)) {
            return *rejected;
        }
        if (auto rejected = noteKnown(replica, tag.vector)) {
            return *rejected;
        }
        std::vector<M> answer;
        for (const auto &logged : log_) {
            if (logged.released && causalLacks(tag.vector, logged.dot)) {
                answer.push_back(logged.message);
            }
        }
        if (auto rejected = prune()) {
            return *rejected;
        }
        return answer;
    }

    /// The stable cut: the pointwise minimum, over the view's members, of the
    /// latest delivered vector known from each, with a member not heard from
    /// at 0 (causal_cut).
    [[nodiscard]] CausalCutResult stableCut() const {
        std::vector<CausalKnown> rows;
        const auto add = [&rows](const CausalReplica &holder, const std::vector<CausalEntry> &vector) {
            const CausalEntry id = holder.entry(0);
            for (const auto &entry : vector) {
                CausalKnown mark;
                mark.holderHi = id.hi;
                mark.holderLo = id.lo;
                mark.hi = entry.hi;
                mark.lo = entry.lo;
                mark.seq = entry.seq;
                rows.push_back(mark);
            }
        };
        add(replica_, delivered_);
        for (const auto &member : known_) {
            add(member.first, member.second);
        }
        const auto members = static_cast<std::uint32_t>(std::min<std::size_t>(
            group_.view.size(), static_cast<std::size_t>(std::numeric_limits<std::uint32_t>::max())));
        auto cut = causalCut(members, rows);
        if (!cut.ok()) {
            return CausalRejected::ruleFailed(cut.error());
        }
        return cut.value();
    }

private:
    /// A change in the log: its dot, the message as it was sent, and whether
    /// it has been released to the peers.
    struct Logged {
        CausalEntry dot;
        M message;
        bool released;
    };

    /// A change held until it is deliverable: its tag and its message.
    struct Held {
        CausalEntry dot;
        std::vector<CausalEntry> context;
        M message;
    };

    /// The number `vector` gives `replica`, 0 when it names none.
    static std::uint64_t seen(const std::vector<CausalEntry> &vector, const CausalReplica &replica) noexcept {
        for (const auto &entry : vector) {
            if (CausalReplica::of(entry) == replica) {
                return entry.seq;
            }
        }
        return 0;
    }

    /// The tag a message of this copy carries: the epoch, and `own` first,
    /// then every other entry of `vector`.
    [[nodiscard]] CausalTag tagOf(const CausalEntry &own, const std::vector<CausalEntry> &vector) const {
        CausalTag tag;
        tag.epoch = group_.epoch;
        tag.vector.reserve(vector.size() + 1);
        tag.vector.push_back(own);
        for (const auto &entry : vector) {
            if (CausalReplica::of(entry) != replica_) {
                tag.vector.push_back(entry);
            }
        }
        return tag;
    }

    /// The sender entry of a tag of this view into `sender`, or why it is
    /// not one.
    [[nodiscard]] std::optional<CausalRejected> senderOf(const CausalTag &tag, CausalEntry &sender) const {
        if (tag.epoch != group_.epoch) {
            return CausalRejected::otherEpoch(tag.epoch);
        }
        if (tag.vector.size() > kCausalVectorBound || tag.vector.empty()) {
            return CausalRejected::malformed();
        }
        sender = tag.vector.front();
        return std::nullopt;
    }

    /// Hold `replica` to the view: a member already heard from, or one more
    /// while the view has a member no replica holds yet. Until view changes
    /// land the view is epoch 0's, and each of its members is held by the
    /// first replica that presents itself for it; a replica past the view's
    /// size is a second incarnation of a member, which is not delivered.
    [[nodiscard]] std::optional<CausalRejected> admitMember(const CausalReplica &replica) const {
        if (known_.count(replica) != 0 || 1 + known_.size() < group_.view.size()) {
            return std::nullopt;
        }
        return CausalRejected::notInView(replica);
    }

    /// Raise the latest vector known of `replica` by `vector`.
    [[nodiscard]] std::optional<CausalRejected> noteKnown(const CausalReplica &replica,
                                                          const std::vector<CausalEntry> &vector) {
        const std::vector<CausalEntry> none;
        const auto it = known_.find(replica);
        const std::vector<CausalEntry> &current = it == known_.end() ? none : it->second;
        auto raised = causalRaise(current, vector);
        if (!raised.ok()) {
            return CausalRejected::ruleFailed(raised.error());
        }
        known_[replica] = raised.value();
        return std::nullopt;
    }

    /// Deliver the change `dot`: raise the delivered vector by it, log it and
    /// hand its message on.
    [[nodiscard]] std::optional<CausalRejected> deliver(const CausalEntry &dot, M message, std::vector<M> &out) {
        auto raised = causalRaise(delivered_, std::vector<CausalEntry>{dot});
        if (!raised.ok()) {
            return CausalRejected::ruleFailed(raised.error());
        }
        delivered_ = raised.value();
        log_.push_back(Logged{dot, message, true});
        out.push_back(std::move(message));
        return std::nullopt;
    }

    /// Deliver every held change that is now deliverable, each after the ones
    /// its context names, and let go of every held change now delivered.
    [[nodiscard]] std::optional<CausalRejected> drainHeld(std::vector<M> &out) {
        while (true) {
            std::optional<std::size_t> next;
            std::size_t index = 0;
            while (index < held_.size()) {
                const auto admission = causalAdmit(delivered_, held_[index].dot, held_[index].context);
                if (!admission.ok()) {
                    return CausalRejected::ruleFailed(admission.error());
                }
                if (admission.value() == CausalAdmission::Drop) {
                    held_.erase(held_.begin() + static_cast<std::ptrdiff_t>(index));
                } else if (admission.value() == CausalAdmission::Deliver) {
                    next = index;
                    break;
                } else {
                    ++index;
                }
            }
            if (!next) {
                return std::nullopt;
            }
            Held held = std::move(held_[*next]);
            held_.erase(held_.begin() + static_cast<std::ptrdiff_t>(*next));
            if (auto rejected = deliver(held.dot, std::move(held.message), out)) {
                return rejected;
            }
        }
    }

    /// Drop every logged change the stable cut covers: it is delivered at
    /// every member of the view, so no repair can ask for it.
    [[nodiscard]] std::optional<CausalRejected> prune() {
        const CausalCutResult cut = stableCut();
        if (const auto *rejected = std::get_if<CausalRejected>(&cut)) {
            return *rejected;
        }
        const auto &covered = std::get<std::vector<CausalEntry>>(cut);
        log_.erase(std::remove_if(log_.begin(), log_.end(),
                                  [&covered](const Logged &logged) { return !causalLacks(covered, logged.dot); }),
                   log_.end());
        return std::nullopt;
    }

    CausalGroup group_;
    CausalReplica replica_;
    /// What this copy delivered, its own entry among them.
    std::vector<CausalEntry> delivered_;
    /// The latest delivered vector known of each other member, by replica.
    std::map<CausalReplica, std::vector<CausalEntry>> known_;
    std::vector<Held> held_;
    std::vector<Logged> log_;
    /// The most changes the log may hold not yet stable.
    std::size_t room_;
};

/// The tag an envelope carries in keys 19 (`causal_epoch`) and 20
/// (`causal_vector`): an empty optional when it carries neither, and
/// Malformed when it carries one without the other.
using CausalEnvelopeTag = std::variant<std::optional<CausalTag>, CausalRejected>;

[[nodiscard]] inline CausalEnvelopeTag envelopeCausalTag(const MeshEnvelope &env) {
    if (!env.causal_epoch && !env.causal_vector) {
        return std::optional<CausalTag>{};
    }
    if (env.causal_epoch && env.causal_vector) {
        CausalTag tag;
        tag.epoch = *env.causal_epoch;
        tag.vector = *env.causal_vector;
        return std::optional<CausalTag>(std::move(tag));
    }
    return CausalRejected::malformed();
}

/// `env` with `tag` in keys 19 and 20; false, and `env` untouched, when the
/// vector holds more entries than a view.
[[nodiscard]] inline bool stampCausalTag(MeshEnvelope &env, const CausalTag &tag) {
    if (tag.vector.size() > kCausalVectorBound) {
        return false;
    }
    env.causal_epoch = tag.epoch;
    env.causal_vector = tag.vector;
    return true;
}

}  // namespace SCE::Mesh
