// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file AuditHost.h
 * @brief The audit log of the standard host (docs/SCE_HOST_FEATURES.md, section 15; docs/adr/0109, D14):
 *        append a typed entry to a stream, redacted by its schema's marks, chained, and checkpointed under
 *        the host's audit key.
 *
 * `sce-host-audit-append` takes the stream's name, the entry (a record of the schema the invoke's `sce:entry`
 * names, as its JSON payload) and the marks the schema declares, and answers the sequence the entry was
 * appended at. Before the entry is written it is redacted: a `secret` field becomes `sce-secret`, a `personal`
 * text `sce-personal:` and the lowercase hexadecimal of its HMAC-SHA-256 under the log's pseudonymisation key,
 * unless the stream keeps personal values in the clear. The entry is written in its canonical form, chained on
 * the row before it, and checkpointed every `checkpoint_every` rows and at a stop (docs/SCE_AUDIT_LOG.md).
 *
 * This host keeps its log in memory, for its own life, and draws its audit key and its pseudonymisation key
 * from the random source at their first use: the durable log is the Rust host core's (`sce-rust-session-store`,
 * `sce_rust_crypto::audit`). The two agree byte for byte on tests/host_features/audit_cross_language.json.
 */

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

#include "core/LogMacros.h"
#include "host/Cose.h"
#include "host/CoseCrypto.h"
#include "host/CoseHost.h"
#include "host/Metrics.h"
#include "host/StandardHost.h"

namespace SCE::StandardHost {

/// Append an entry to a stream of the audit log.
inline constexpr const char *AUDIT_APPEND = "sce-host-audit-append";

/// The operation of the feature.
inline constexpr std::array<const char *, 1> AUDIT_TYPES = {AUDIT_APPEND};

/// The bounds of the request's fields, in bytes (`sce:max-size` of each).
namespace AuditBounds {
inline constexpr size_t STREAM = 64;
inline constexpr size_t ENTRY = 4096;
inline constexpr size_t MARKS = 1024;
}  // namespace AuditBounds

/// What a `secret` field holds in a written entry.
inline constexpr const char *AUDIT_SECRET_MARKER = "sce-secret";
/// What a `personal` text written as its keyed digest begins with.
inline constexpr const char *AUDIT_PERSONAL_PREFIX = "sce-personal:";
/// The id the audit key signs a checkpoint under, its `kid`.
inline constexpr const char *AUDIT_SIGNING_KEY_ID = "audit:sign";
/// The context a checkpoint's payload begins with: the version of its form.
inline constexpr const char *AUDIT_CHECKPOINT_CONTEXT = "sce-audit-1";

/// How a stream writes a `personal` field.
enum class AuditPersonal {
    /// As its keyed digest under the pseudonymisation key.
    Pseudonymised,
    /// As it is.
    Clear,
};

/// What a host keeps of one stream.
struct AuditStreamPolicy {
    /// A checkpoint every this many rows; 0 signs only at a stop.
    uint64_t checkpointEvery = 0;
    /// How many days an entry is kept before retention may delete its segment.
    uint32_t retainDays = 0;
    AuditPersonal personal = AuditPersonal::Pseudonymised;
};

/// One row of a stream.
struct AuditRow {
    uint64_t sequence = 0;
    uint64_t atMs = 0;
    std::string entry;
    Cose::Bytes chain;
};

/// A signature over a stream at a sequence.
struct AuditCheckpoint {
    uint64_t sequence = 0;
    Cose::Bytes chain;
    Cose::Bytes signature;
};

/// What a reader found a stream to be (docs/SCE_AUDIT_LOG.md, section 5).
struct AuditVerdict {
    enum class Kind {
        /// Every row from `from` through `through` chains on its anchor, and every checkpoint is signed and holds
        /// the chain at its sequence.
        Verified,
        /// The checkpoint at `sequence` is not signed by the audit key over what it holds.
        Forged,
        /// The first row held, `sequence`, follows no checkpoint and is not the stream's first.
        Unanchored,
        /// The row after `from` is not the next number.
        Missing,
        /// Row `sequence` does not chain on the rows before it.
        Changed,
        /// A checkpoint, or the head, is at `sequence`, past the last row held, `through`.
        Truncated,
    };
    Kind kind = Kind::Verified;
    uint64_t from = 0;
    uint64_t through = 0;
    uint64_t sequence = 0;
};

namespace audit {

/// `E_n`, the bytes row `sequence` of `stream` is chained by: `[stream, sequence, at_ms, entry]`.
inline Cose::Bytes entryItem(const std::string &stream, uint64_t sequence, uint64_t atMs, const std::string &entry) {
    Cose::Bytes item;
    Cose::Cbor::array(item, 4);
    Cose::Cbor::textString(item, stream);
    Cose::Cbor::head(item, Cose::Cbor::Major::Unsigned, sequence);
    Cose::Cbor::head(item, Cose::Cbor::Major::Unsigned, atMs);
    Cose::Cbor::byteString(item, Cose::Bytes(entry.begin(), entry.end()));
    return item;
}

/// The chain of row `sequence` after `previous`: `SHA-256(previous || E_n)`.
inline std::optional<Cose::Bytes> chained(CoseCrypto &crypto, const Cose::Bytes &previous, const std::string &stream,
                                          uint64_t sequence, uint64_t atMs, const std::string &entry) {
    Cose::Bytes input = previous;
    const Cose::Bytes item = entryItem(stream, sequence, atMs, entry);
    input.insert(input.end(), item.begin(), item.end());
    return crypto.sha256(input);
}

/// What a checkpoint signs: `["sce-audit-1", stream, sequence, chain]`.
inline Cose::Bytes checkpointPayload(const std::string &stream, uint64_t sequence, const Cose::Bytes &chain) {
    Cose::Bytes payload;
    Cose::Cbor::array(payload, 4);
    Cose::Cbor::textString(payload, AUDIT_CHECKPOINT_CONTEXT);
    Cose::Cbor::textString(payload, stream);
    Cose::Cbor::head(payload, Cose::Cbor::Major::Unsigned, sequence);
    Cose::Cbor::byteString(payload, chain);
    return payload;
}

/// The bytes of the audit key's id, its `kid`.
inline Cose::Bytes signingKid() {
    const std::string id = AUDIT_SIGNING_KEY_ID;
    return Cose::Bytes(id.begin(), id.end());
}

/// The tagged COSE_Sign1 of a checkpoint, signed with the Ed25519 key of `seed`.
inline std::optional<Cose::Bytes> signCheckpoint(CoseCrypto &crypto, const Cose::Bytes &seed, const std::string &stream,
                                                 uint64_t sequence, const Cose::Bytes &chain) {
    const Cose::Bytes protectedHeader = Cose::sign1ProtectedHeader(signingKid());
    const Cose::Bytes payload = checkpointPayload(stream, sequence, chain);
    const auto signature = crypto.ed25519Sign(seed, Cose::sigStructure(protectedHeader, {}, payload));
    if (!signature) {
        return std::nullopt;
    }
    return Cose::sign1(protectedHeader, payload, *signature);
}

/// Whether `checkpoint` of `stream` is signed by the audit key whose public key is `publicKey`, over its own
/// stream, sequence and chain.
inline bool checkpointVerifies(CoseCrypto &crypto, const Cose::Bytes &publicKey, const std::string &stream,
                               const AuditCheckpoint &checkpoint) {
    const auto object = Cose::readSign1(checkpoint.signature);
    if (!object || object->protectedHeader != Cose::sign1ProtectedHeader(signingKid()) ||
        object->payload != checkpointPayload(stream, checkpoint.sequence, checkpoint.chain)) {
        return false;
    }
    return crypto.ed25519Verify(publicKey, Cose::sigStructure(object->protectedHeader, {}, object->payload),
                                object->signature);
}

inline void writeString(const std::string &text, std::string &out) {
    static const char *hex = "0123456789abcdef";
    out += '"';
    for (const char c : text) {
        const auto u = static_cast<unsigned char>(c);
        switch (c) {
        case '"':
            out += "\\\"";
            break;
        case '\\':
            out += "\\\\";
            break;
        case '\n':
            out += "\\n";
            break;
        case '\r':
            out += "\\r";
            break;
        case '\t':
            out += "\\t";
            break;
        default:
            if (u < 0x20) {
                out += "\\u00";
                out += hex[u >> 4];
                out += hex[u & 0x0Fu];
            } else {
                out += c;
            }
        }
    }
    out += '"';
}

inline void writeCanonical(const nlohmann::json &value, std::string &out) {
    switch (value.type()) {
    case nlohmann::json::value_t::object: {
        // A json object keeps its members in a std::map, whose order is that of the names' bytes.
        out += '{';
        bool first = true;
        for (const auto &[name, member] : value.items()) {
            if (!first) {
                out += ',';
            }
            first = false;
            writeString(name, out);
            out += ':';
            writeCanonical(member, out);
        }
        out += '}';
        break;
    }
    case nlohmann::json::value_t::array: {
        out += '[';
        bool first = true;
        for (const auto &element : value) {
            if (!first) {
                out += ',';
            }
            first = false;
            writeCanonical(element, out);
        }
        out += ']';
        break;
    }
    case nlohmann::json::value_t::string:
        writeString(value.get<std::string>(), out);
        break;
    default:
        // A number, a bool or null, as JSON writes it.
        out += value.dump();
        break;
    }
}

/// `value` in the canonical form of an audit entry (docs/SCE_AUDIT_LOG.md, section 2).
inline std::string canonicalJson(const nlohmann::json &value) {
    std::string out;
    writeCanonical(value, out);
    return out;
}

/// A mark a schema declares on a field.
enum class Mark { Secret, Personal };

/// The marks `text` writes, each the segments of a field's pointer and its mark, or nothing for a text that is
/// not marks: `/a=secret;/b/c=personal`, a segment `*` naming every element of a list.
inline std::optional<std::vector<std::pair<std::vector<std::string>, Mark>>> marks(const std::string &text) {
    std::vector<std::pair<std::vector<std::string>, Mark>> out;
    if (text.empty()) {
        return out;
    }
    size_t start = 0;
    while (start <= text.size()) {
        size_t end = text.find(';', start);
        if (end == std::string::npos) {
            end = text.size();
        }
        const std::string one = text.substr(start, end - start);
        start = end + 1;
        const auto equals = one.find('=');
        if (equals == std::string::npos || one.empty() || one.front() != '/') {
            return std::nullopt;
        }
        const std::string pointer = one.substr(1, equals - 1);
        const std::string name = one.substr(equals + 1);
        Mark mark = Mark::Secret;
        if (name == "secret") {
            mark = Mark::Secret;
        } else if (name == "personal") {
            mark = Mark::Personal;
        } else {
            return std::nullopt;
        }
        std::vector<std::string> segments;
        size_t from = 0;
        while (true) {
            const size_t slash = pointer.find('/', from);
            const std::string segment =
                pointer.substr(from, slash == std::string::npos ? std::string::npos : slash - from);
            if (segment.empty()) {
                return std::nullopt;
            }
            segments.push_back(segment);
            if (slash == std::string::npos) {
                break;
            }
            from = slash + 1;
        }
        out.emplace_back(std::move(segments), mark);
        if (end == text.size()) {
            break;
        }
    }
    return out;
}

/// Why an entry was not redacted.
enum class Unredacted {
    /// Redacted.
    None,
    /// The request is no entry the marks apply to: `invalid_argument`.
    Refused,
    /// The pseudonymisation key could not be used: `unavailable`.
    KeyFailed,
};

/// What gives a personal text's keyed digest, in lowercase hexadecimal, or nothing.
using Pseudonym = std::function<std::optional<std::string>(const std::string &)>;

inline Unredacted redactValue(nlohmann::json &value, Mark mark, AuditPersonal personal, const Pseudonym &pseudonym) {
    if (mark == Mark::Secret) {
        value = AUDIT_SECRET_MARKER;
        return Unredacted::None;
    }
    // An absent personal value names no one.
    if (value.is_null()) {
        return Unredacted::None;
    }
    if (!value.is_string()) {
        return Unredacted::Refused;
    }
    if (personal == AuditPersonal::Pseudonymised) {
        const auto digest = pseudonym(value.get<std::string>());
        if (!digest) {
            return Unredacted::KeyFailed;
        }
        value = std::string(AUDIT_PERSONAL_PREFIX) + *digest;
    }
    return Unredacted::None;
}

inline Unredacted apply(nlohmann::json &value, const std::vector<std::string> &segments, size_t at, Mark mark,
                        AuditPersonal personal, const Pseudonym &pseudonym) {
    if (at >= segments.size()) {
        return Unredacted::Refused;
    }
    if (value.is_null()) {
        return Unredacted::None;
    }
    const bool last = at + 1 == segments.size();
    if (segments[at] == "*") {
        if (!value.is_array()) {
            return Unredacted::Refused;
        }
        for (auto &element : value) {
            const Unredacted done = last ? redactValue(element, mark, personal, pseudonym)
                                         : apply(element, segments, at + 1, mark, personal, pseudonym);
            if (done != Unredacted::None) {
                return done;
            }
        }
        return Unredacted::None;
    }
    if (!value.is_object() || !value.contains(segments[at])) {
        return Unredacted::Refused;
    }
    nlohmann::json &member = value[segments[at]];
    return last ? redactValue(member, mark, personal, pseudonym)
                : apply(member, segments, at + 1, mark, personal, pseudonym);
}

/// The entry `entry`, a JSON object, redacted by `marksText` and written in its canonical form, into `out`.
inline Unredacted redact(const std::string &entry, const std::string &marksText, AuditPersonal personal,
                         const Pseudonym &pseudonym, std::string &out) {
    nlohmann::json value = nlohmann::json::parse(entry, nullptr, false);
    if (value.is_discarded() || !value.is_object()) {
        return Unredacted::Refused;
    }
    const auto parsed = marks(marksText);
    if (!parsed) {
        return Unredacted::Refused;
    }
    for (const auto &[segments, mark] : *parsed) {
        const Unredacted done = apply(value, segments, 0, mark, personal, pseudonym);
        if (done != Unredacted::None) {
            return done;
        }
    }
    out = canonicalJson(value);
    return Unredacted::None;
}

}  // namespace audit

/// The append, over the primitives of `crypto` and the time of `clock`, with its log in memory.
///
/// An append to a stream the host keeps no policy for fails `unknown_stream`; the clock giving no time, the
/// random source giving no key, or the library failing, `unavailable`. A request is refused (`invalid_argument`)
/// when its stream is empty or past its bound, its entry or its marks past theirs, or its entry no JSON object
/// the marks apply to.
class AuditFeature : public Feature {
public:
    AuditFeature(CoseCrypto &crypto, ClockSource &clock) : crypto_(crypto), clock_(clock) {}

    AuditFeature(const AuditFeature &) = delete;
    AuditFeature &operator=(const AuditFeature &) = delete;

    /// The keys are wiped as they are dropped.
    ~AuditFeature() override {
        crypto_.wipe(seed_);
        crypto_.wipe(pseudonymKey_);
    }

    /// Keep the streams `streams` names, each with its policy, in place of those kept before.
    void setStreams(std::map<std::string, AuditStreamPolicy> streams) {
        streams_ = std::move(streams);
    }

    /// Report each entry appended to `metrics`, which outlives the feature, from here on (docs/adr/0109,
    /// D16); a repeat under one identity appends nothing and is not reported.
    void setMetrics(SCE::Metrics::Metrics &metrics) {
        metrics_ = &metrics;
    }

    /// The rows of `stream`, oldest first, as the log holds them: what a hand on the medium would edit.
    std::vector<AuditRow> &rows(const std::string &stream) {
        return rows_[stream];
    }

    /// The checkpoints of `stream`, in the order of their sequences, as the log holds them.
    std::vector<AuditCheckpoint> &checkpoints(const std::string &stream) {
        return checkpoints_[stream];
    }

    /// The public key of the audit key, or nothing before the host signed a first checkpoint.
    std::optional<Cose::Bytes> publicKey() {
        if (seed_.empty()) {
            return std::nullopt;
        }
        return crypto_.ed25519PublicKey(seed_);
    }

    /// Sign the head of every stream the host keeps unless a checkpoint is kept at it: what a host does at its
    /// orderly stop. Answers each stream it signed, with the sequence; nothing when a key could not be had.
    std::optional<std::vector<std::pair<std::string, uint64_t>>> stop() {
        std::vector<std::pair<std::string, uint64_t>> signed_;
        for (const auto &entry : streams_) {
            const std::string &stream = entry.first;
            const Head head = headOf(stream);
            if (head.sequence == 0) {
                continue;
            }
            auto &kept = checkpoints_[stream];
            if (!kept.empty() && kept.back().sequence == head.sequence) {
                continue;
            }
            const auto signature = sign(stream, head.sequence, head.chain);
            if (!signature) {
                return std::nullopt;
            }
            kept.push_back(AuditCheckpoint{head.sequence, head.chain, *signature});
            signed_.emplace_back(stream, head.sequence);
        }
        return signed_;
    }

    /// Delete, from every stream the host keeps, the segments older than its `retain_days` before `nowMs`, each
    /// through the latest checkpoint before the first row kept, which then anchors the rest. Answers how many
    /// rows it deleted.
    uint64_t retain(uint64_t nowMs) {
        constexpr uint64_t DAY_MS = 86400000;
        uint64_t deleted = 0;
        for (const auto &entry : streams_) {
            const std::string &stream = entry.first;
            const uint64_t span = static_cast<uint64_t>(entry.second.retainDays) * DAY_MS;
            const uint64_t before = nowMs > span ? nowMs - span : 0;
            auto &held = rows_[stream];
            auto &kept = checkpoints_[stream];
            uint64_t bound = held.empty() ? headOf(stream).sequence + 1 : held.back().sequence + 1;
            for (const auto &row : held) {
                if (row.atMs >= before) {
                    bound = row.sequence;
                    break;
                }
            }
            std::optional<uint64_t> cut;
            for (const auto &checkpoint : kept) {
                if (checkpoint.sequence < bound) {
                    cut = checkpoint.sequence;
                }
            }
            if (!cut) {
                continue;
            }
            const size_t before_count = held.size();
            held.erase(
                std::remove_if(held.begin(), held.end(), [&cut](const AuditRow &row) { return row.sequence <= *cut; }),
                held.end());
            kept.erase(std::remove_if(kept.begin(), kept.end(),
                                      [&cut](const AuditCheckpoint &checkpoint) { return checkpoint.sequence < *cut; }),
                       kept.end());
            deleted += before_count - held.size();
        }
        return deleted;
    }

    /// What a reader finds `stream` to be under the audit key whose public key is `publicKey`.
    AuditVerdict verify(const std::string &stream, const Cose::Bytes &publicKey) {
        const auto &held = rows_[stream];
        const auto &kept = checkpoints_[stream];
        AuditVerdict verdict;
        for (const auto &checkpoint : kept) {
            if (!audit::checkpointVerifies(crypto_, publicKey, stream, checkpoint)) {
                verdict.kind = AuditVerdict::Kind::Forged;
                verdict.sequence = checkpoint.sequence;
                return verdict;
            }
        }
        uint64_t sequence = 0;
        Cose::Bytes chain(32, 0);
        if (held.empty()) {
            if (!kept.empty()) {
                sequence = kept.back().sequence;
                chain = kept.back().chain;
            }
        } else if (held.front().sequence != 1) {
            const uint64_t before = held.front().sequence - 1;
            const auto anchor = std::find_if(kept.begin(), kept.end(), [before](const AuditCheckpoint &checkpoint) {
                return checkpoint.sequence == before;
            });
            if (anchor == kept.end()) {
                verdict.kind = AuditVerdict::Kind::Unanchored;
                verdict.sequence = held.front().sequence;
                return verdict;
            }
            sequence = anchor->sequence;
            chain = anchor->chain;
        }
        verdict.from = sequence + 1;
        for (const auto &row : held) {
            if (row.sequence != sequence + 1) {
                verdict.kind = AuditVerdict::Kind::Missing;
                verdict.from = sequence;
                verdict.sequence = row.sequence;
                return verdict;
            }
            const auto expected = audit::chained(crypto_, chain, stream, row.sequence, row.atMs, row.entry);
            const auto checkpoint = std::find_if(kept.begin(), kept.end(), [&row](const AuditCheckpoint &held_) {
                return held_.sequence == row.sequence;
            });
            if (!expected || *expected != row.chain || (checkpoint != kept.end() && checkpoint->chain != *expected)) {
                verdict.kind = AuditVerdict::Kind::Changed;
                verdict.sequence = row.sequence;
                return verdict;
            }
            sequence = row.sequence;
            chain = *expected;
        }
        uint64_t latest = headOf(stream).sequence;
        for (const auto &checkpoint : kept) {
            latest = std::max(latest, checkpoint.sequence);
        }
        if (latest > sequence) {
            verdict.kind = AuditVerdict::Kind::Truncated;
            verdict.sequence = latest;
            verdict.through = sequence;
            return verdict;
        }
        verdict.kind = AuditVerdict::Kind::Verified;
        verdict.through = sequence;
        return verdict;
    }

    std::vector<std::string> types() const override {
        return std::vector<std::string>(AUDIT_TYPES.begin(), AUDIT_TYPES.end());
    }

    bool accepts(const std::string &, const Params &params, std::string &why) const override {
        const std::string stream = argument(params, "stream");
        const std::string entry = argument(params, "entry");
        const std::string marks = argument(params, "sensitivity");
        if (stream.empty() || stream.size() > AuditBounds::STREAM) {
            why = "stream is empty or past its bound";
            return false;
        }
        if (entry.size() > AuditBounds::ENTRY || marks.size() > AuditBounds::MARKS) {
            why = "entry or sensitivity is past its bound";
            return false;
        }
        // A dry run of the redaction, whose digests are never written.
        const audit::Pseudonym nothing = [](const std::string &) { return std::optional<std::string>(""); };
        std::string written;
        if (audit::redact(entry, marks, AuditPersonal::Pseudonymised, nothing, written) != audit::Unredacted::None) {
            why = "entry is no JSON object the marks apply to";
            return false;
        }
        return true;
    }

    Outcome failure(const std::string &, const std::string &code) const override {
        return detail::failed(code, {detail::text("sequence", "0")});
    }

    Outcome run(const std::string &operation, const Invocation &invocation, const Params &params) override {
        const std::string stream = argument(params, "stream");
        const auto policy = streams_.find(stream);
        if (policy == streams_.end()) {
            SCE_LOG_INFO("{} {}: no stream '{}' is kept", operation, invocation.id, stream);
            return failure(operation, "unknown_stream");
        }
        const auto reading = clock_.read();
        if (!reading || reading->utcMs < 0) {
            SCE_LOG_INFO("{} {}: the clock gave no time to append at", operation, invocation.id);
            return failure(operation, "unavailable");
        }
        // A repeat answers the first sequence before anything is drawn or digested.
        if (const auto first = identities_.find(invocation); first != identities_.end()) {
            return detail::record({detail::text("sequence", std::to_string(first->second))});
        }
        const audit::Pseudonym pseudonym = [this](const std::string &text) -> std::optional<std::string> {
            if (!keep(pseudonymKey_)) {
                return std::nullopt;
            }
            const auto digest = crypto_.hmacSha256(pseudonymKey_, Cose::Bytes(text.begin(), text.end()));
            if (!digest) {
                return std::nullopt;
            }
            return lowerHex(*digest, digest->size());
        };
        std::string written;
        switch (audit::redact(argument(params, "entry"), argument(params, "sensitivity"), policy->second.personal,
                              pseudonym, written)) {
        case audit::Unredacted::None:
            break;
        case audit::Unredacted::Refused:
            return Outcome{Outcome::Kind::Refuse, R"({"reason":"invalid_argument"})"};
        case audit::Unredacted::KeyFailed:
            return failure(operation, "unavailable");
        }
        const auto atMs = static_cast<uint64_t>(reading->utcMs);
        const Head head = headOf(stream);
        const uint64_t sequence = head.sequence + 1;
        const auto chain = audit::chained(crypto_, head.chain, stream, sequence, atMs, written);
        if (!chain) {
            return failure(operation, "unavailable");
        }
        std::optional<Cose::Bytes> signature;
        const uint64_t every = policy->second.checkpointEvery;
        if (every != 0 && sequence % every == 0) {
            signature = sign(stream, sequence, *chain);
            if (!signature) {
                return failure(operation, "unavailable");
            }
        }
        rows_[stream].push_back(AuditRow{sequence, atMs, written, *chain});
        heads_[stream] = Head{sequence, *chain};
        if (signature) {
            checkpoints_[stream].push_back(AuditCheckpoint{sequence, *chain, *signature});
        }
        identities_[invocation] = sequence;
        if (metrics_ != nullptr) {
            metrics_->count(SCE::Metrics::Counter::auditAppends(), 1);
        }
        return detail::record({detail::text("sequence", std::to_string(sequence))});
    }

private:
    /// The last row appended to a stream, which retention leaves.
    struct Head {
        uint64_t sequence = 0;
        Cose::Bytes chain = Cose::Bytes(32, 0);
    };

    static std::string argument(const Params &params, const char *name) {
        const auto found = params.find(name);
        return found == params.end() ? std::string() : found->second;
    }

    Head headOf(const std::string &stream) const {
        const auto found = heads_.find(stream);
        return found == heads_.end() ? Head{} : found->second;
    }

    /// `key`, drawn from the random source at its first use: false when the source gave none.
    bool keep(Cose::Bytes &key) {
        if (!key.empty()) {
            return true;
        }
        auto drawn = crypto_.random(COSE_KEY_BYTES);
        if (!drawn || drawn->size() != COSE_KEY_BYTES) {
            if (drawn) {
                crypto_.wipe(*drawn);
            }
            return false;
        }
        key = *drawn;
        crypto_.wipe(*drawn);
        return true;
    }

    /// The checkpoint of `stream` at `sequence`, signed under the audit key, drawn at its first use.
    std::optional<Cose::Bytes> sign(const std::string &stream, uint64_t sequence, const Cose::Bytes &chain) {
        if (!keep(seed_)) {
            return std::nullopt;
        }
        return audit::signCheckpoint(crypto_, seed_, stream, sequence, chain);
    }

    CoseCrypto &crypto_;
    ClockSource &clock_;
    std::map<std::string, AuditStreamPolicy> streams_;
    std::map<std::string, std::vector<AuditRow>> rows_;
    std::map<std::string, std::vector<AuditCheckpoint>> checkpoints_;
    std::map<std::string, Head> heads_;
    /// The sequence each identity was appended at.
    std::map<Invocation, uint64_t> identities_;
    /// The audit key's Ed25519 seed and the pseudonymisation key, each drawn at its first use.
    Cose::Bytes seed_;
    Cose::Bytes pseudonymKey_;
    /// What each append is reported to (docs/adr/0109, D16): none until the embedding gives one.
    SCE::Metrics::Metrics *metrics_ = nullptr;
};

}  // namespace SCE::StandardHost
