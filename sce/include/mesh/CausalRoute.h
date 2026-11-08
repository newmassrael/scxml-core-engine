// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh CausalRoute — the C++ router's side of one causal binding.
//
// A binding a deploy marks `delivery: causal` is the route of a causal
// broadcast group (docs/adr/0085, section 3.2; SCE_MESH.md §mesh-10.12).
// This class is what the generated TransportRouter holds for it, in place of
// the duplicate window and the sequence buffer it replaces: the binding's
// CausalLayer, over the envelopes themselves, and the envelope work around it
// that the Rust router does for its causal peers (backends/rust/mesh/src/
// router.rs, `offer`, `committed`, `announce` and `receive_causal`).
//
// * A send to the binding is offered (`offer`): the layer answers a verdict,
//   and an accepted change leaves only when the host reports its journal
//   record committed (`committed`), which gives the envelopes to send.
// * This copy's `causalVector` (pattern 23) is owed to the peer on a period
//   (`announceDue`) and whenever its link comes back; `announce` writes it.
// * An envelope that arrives (`receive`) is a change, which the layer
//   delivers, holds or drops, or a `causalVector`, which the layer records as
//   its sender's mark and answers with every change of the log it lacks. An
//   envelope of another epoch, or by a replica the view does not hold, is
//   raised as CAUSAL_VIEW_MISMATCH; one without its tag, or with half of it,
//   as ENVELOPE_CORRUPT; neither is delivered.
//
// Transport-free and not synchronised, like CausalLayer: the generated router
// serialises the calls and moves the envelopes.

#pragma once

#include "common/Uuid.h"
#include "mesh/CausalLayer.h"
#include "mesh/CommunicationError.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/PatternKind.h"
#include "mesh/PayloadCodec.h"

#include <sce/forge/algorithm.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace SCE::Mesh {

/// Why a causal route could not judge what it was handed.
struct CausalRouteFault {
    enum class Kind : std::uint8_t {
        /// The host has not started the binding's layer (`CausalRoute::start`).
        NotStarted,
        /// A rule refused its input.
        Rule,
    };

    Kind kind = Kind::NotStarted;
    ::SCE::Forge::AlgorithmError rule = ::SCE::Forge::AlgorithmError::Overflow;
};

/// What an envelope that arrived on a causal binding comes to.
struct CausalReceipt {
    /// Envelopes to send to the binding's peer: the changes its vector lacks,
    /// as they were sent, their tags intact.
    std::vector<MeshEnvelope> send;
    /// Envelopes to hand to the engine, in a causal order.
    std::vector<MeshEnvelope> deliver;
    /// The row the envelope raises when it is not taken in.
    std::optional<CommunicationError> raise;
    /// Set when the route could not judge the envelope; nothing is delivered.
    std::optional<CausalRouteFault> fault;
};

class CausalRoute {
public:
    using Layer = CausalLayer<MeshEnvelope>;

    /// The route of `group`, for the machine `machine` (the `source` of its
    /// `causalVector`) over the transport `transport` (the column of an
    /// ENVELOPE_CORRUPT it raises). Not started: the host starts it with the
    /// incarnation it read from the copy's journal.
    CausalRoute(std::string machine, std::string transport, CausalGroup group)
        : machine_(std::move(machine)), transport_(std::move(transport)), group_(std::move(group)) {}

    /// Start the binding's layer: `start` is the incarnation the host read
    /// from the copy's journal, or a new one; `room` the most changes the
    /// layer may hold not yet stable; `announce_every_ms` the period of its
    /// `causalVector`. Starting again begins the layer again.
    void start(const CausalStart &start, std::size_t room, std::int64_t announce_every_ms) {
        layer_.emplace(group_, start, room);
        announce_every_ms_ = announce_every_ms;
        announced_ms_.reset();
    }

    [[nodiscard]] bool started() const noexcept {
        return layer_.has_value();
    }

    [[nodiscard]] const CausalGroup &group() const noexcept {
        return group_;
    }

    /// The binding's layer, for the host to read (its replica, its delivered
    /// vector, what it holds); null until started.
    [[nodiscard]] const Layer *layer() const noexcept {
        return layer_ ? &*layer_ : nullptr;
    }

    /// Offer a change the document sent to the binding (docs/adr/0085,
    /// section 3, "offer and verdict"): the verdict is the host's to act on,
    /// and nothing leaves before the host commits an accepted change's
    /// journal record. An envelope that cannot be written with its tag takes
    /// no number. Empty when the route is not started.
    [[nodiscard]] std::optional<CausalOffered> offer(const MeshEnvelope &env) {
        if (!layer_) {
            return std::nullopt;
        }
        // §mesh-10.12, offer and verdict: the change is tagged on the
        // envelope itself, keys 19 and 20, and released only by `committed`.
        return layer_->offer([&env](const CausalTag &tag) -> std::optional<MeshEnvelope> {
            MeshEnvelope stamped = env;
            if (!stampCausalTag(stamped, tag) || encodeEnvelope(stamped).empty()) {
                return std::nullopt;
            }
            return stamped;
        });
    }

    /// The host committed the journal record of `dot`: the envelopes of it,
    /// and of every accepted change before it, to send to the peer.
    [[nodiscard]] std::vector<MeshEnvelope> committed(const CausalDot &dot) {
        if (!layer_) {
            return {};
        }
        return layer_->committed(dot);
    }

    /// Whether this copy's `causalVector` is owed on its period at `now_ms`.
    [[nodiscard]] bool announceDue(std::int64_t now_ms) const noexcept {
        if (!layer_) {
            return false;
        }
        return !announced_ms_ || now_ms - *announced_ms_ >= announce_every_ms_;
    }

    /// This copy's `causalVector` (pattern 23: the epoch and its delivered
    /// vector, keys 19 and 20), under the envelope id `id` the host drew.
    /// Empty when the route is not started.
    [[nodiscard]] std::optional<MeshEnvelope> announce(const std::array<std::uint8_t, 16> &id, std::int64_t now_ms) {
        if (!layer_) {
            return std::nullopt;
        }
        MeshEnvelope env;
        env.id = id;
        env.source = machine_;
        env.type = "";
        env.pattern = PatternKind::CausalVector;
        env.datacontenttype = PayloadCodec::None;
        if (!stampCausalTag(env, layer_->summary())) {
            return std::nullopt;
        }
        announced_ms_ = now_ms;
        return env;
    }

    /// An envelope the binding's peer delivered (docs/adr/0085, section 3.2).
    [[nodiscard]] CausalReceipt receive(const MeshEnvelope &env) {
        CausalReceipt receipt;
        const CausalEnvelopeTag carried = envelopeCausalTag(env);
        const auto *present = std::get_if<std::optional<CausalTag>>(&carried);
        // An envelope on a causal route carries its tag; one without it, or
        // with half of it, cannot be placed, and is not delivered.
        if (present == nullptr || !present->has_value()) {
            receipt.raise = corrupt(env.source);
            return receipt;
        }
        const CausalTag &tag = **present;
        if (!layer_) {
            receipt.fault = CausalRouteFault{};
            return receipt;
        }
        // §mesh-10.12: a change goes through the layer's rule, and a
        // `causalVector` is recorded as its sender's mark and answered with
        // every change of the log it lacks; it never reaches the engine.
        CausalTaken<MeshEnvelope> taken =
            env.pattern == PatternKind::CausalVector ? layer_->receiveSummary(tag) : layer_->receive(tag, env);
        if (auto *rejected = std::get_if<CausalRejected>(&taken)) {
            switch (rejected->kind) {
            case CausalRejected::Kind::OtherEpoch:
                receipt.raise =
                    CommunicationError::causalViewMismatch(env.source, rejected->epoch, replicaText(tag.vector));
                break;
            case CausalRejected::Kind::NotInView:
                receipt.raise = CommunicationError::causalViewMismatch(env.source, tag.epoch,
                                                                       SCE::uuid::to_string(rejected->replica.id));
                break;
            case CausalRejected::Kind::Malformed:
                receipt.raise = corrupt(env.source);
                break;
            case CausalRejected::Kind::Rule: {
                CausalRouteFault fault;
                fault.kind = CausalRouteFault::Kind::Rule;
                fault.rule = rejected->rule;
                receipt.fault = fault;
                break;
            }
            }
            return receipt;
        }
        auto &envelopes = std::get<std::vector<MeshEnvelope>>(taken);
        if (env.pattern == PatternKind::CausalVector) {
            receipt.send = std::move(envelopes);
        } else {
            receipt.deliver = std::move(envelopes);
        }
        return receipt;
    }

private:
    /// Row 4, as the generated router raises it for an envelope it cannot
    /// read: the transport, the codec and, here, the sender.
    [[nodiscard]] CommunicationError corrupt(const std::string &source) const {
        CommunicationError err;
        err.reason = ReasonCode::EnvelopeCorrupt;
        err.source = source;
        err.transport = transport_;
        err.codec = "cbor";
        return err;
    }

    /// The replica a vector's first entry names, as RFC 4122 text; empty when
    /// it names none.
    [[nodiscard]] static std::string replicaText(const std::vector<CausalEntry> &vector) {
        if (vector.empty()) {
            return std::string();
        }
        return SCE::uuid::to_string(CausalReplica::of(vector.front()).id);
    }

    std::string machine_;
    std::string transport_;
    CausalGroup group_;
    std::optional<Layer> layer_;
    std::int64_t announce_every_ms_ = 0;
    std::optional<std::int64_t> announced_ms_;
};

}  // namespace SCE::Mesh
