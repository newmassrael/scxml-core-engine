// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file CredentialHost.h
 * @brief The credential operations of the standard host (docs/SCE_HOST_FEATURES.md, section 14;
 *        docs/adr/0109, D3): issue a bearer token for a name, and revoke a name's tokens.
 *
 * A statechart decides who gets a token; the host keeps it. A token is `sce1.` followed by the base64url,
 * without padding, of the HMAC-SHA-256, under the key ring's issuing key (`CoseFeature::issuingTag`), of the
 * deterministic CBOR array `[session id, invoke id, start, name]`: the invocation's identity (section 1) and
 * the name. A repeat under one identity, as a restore starts the invocation again, is the same token. The
 * host keeps the SHA-256 of each token with its name, the moment it was issued, its expiry and whether it is
 * revoked, never the token.
 *
 * This host keeps its tokens in memory, for its own life, as it keeps its key ring: the durable credential
 * store, and the installation pins of the `https` server, are the Rust host core's (`sce-rust-session-store`).
 * A revocation here revokes tokens only, since this host keeps no pin.
 */

#include <array>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <map>
#include <optional>
#include <string>
#include <vector>

#include "core/LogMacros.h"
#include "host/Cose.h"
#include "host/CoseHost.h"
#include "host/StandardHost.h"

namespace SCE::StandardHost {

/// Issue a bearer token for a name.
inline constexpr const char *CREDENTIAL_ISSUE = "sce-host-credential-issue";
/// Revoke a name's tokens.
inline constexpr const char *CREDENTIAL_REVOKE = "sce-host-credential-revoke";

/// Every operation type of the feature, in the order the specification lists them.
inline constexpr std::array<const char *, 2> CREDENTIAL_TYPES = {CREDENTIAL_ISSUE, CREDENTIAL_REVOKE};

/// What every token's text begins with: the version of the token's form.
inline constexpr const char *TOKEN_PREFIX = "sce1.";

/// The bound of a request's `name`, in bytes (`sce:max-size` of the field).
inline constexpr size_t CREDENTIAL_NAME_MAX = 256;

/// A token as the host keeps it: its digest is the key it is found by.
struct StoredToken {
    std::string name;
    /// When it was issued, in milliseconds since the Unix epoch.
    uint64_t issuedMs = 0;
    /// When it stops proving its name, in milliseconds since the Unix epoch.
    uint64_t expiresMs = 0;
    bool revoked = false;
};

/// The two operations, over the key ring of `keyRing`, the primitives of `crypto` and the time of `clock`.
///
/// - Issue answers the token for `name`, which expires `ttl_s` seconds after the clock reads. It fails
///   `unavailable` when the clock gives no time, the random source gives no issuing key or the library
///   could not be used.
/// - Revoke marks every token of `name` not revoked yet as revoked and answers how many it marked. A revoked
///   token is kept, so that an issue repeated under its identity answers it revoked rather than live.
///
/// A request is refused (`invalid_argument`) when its `name` is empty or past its bound, or when an issue's
/// `ttl_s` is not a count of seconds from 1 to the largest a uint32 holds.
class CredentialFeature : public Feature {
public:
    CredentialFeature(CoseFeature &keyRing, CoseCrypto &crypto, ClockSource &clock)
        : keyRing_(keyRing), crypto_(crypto), clock_(clock) {}

    CredentialFeature(const CredentialFeature &) = delete;
    CredentialFeature &operator=(const CredentialFeature &) = delete;

    /// Every token the host keeps, by its digest, in the order of the digests' bytes.
    const std::map<Cose::Bytes, StoredToken> &tokens() const {
        return tokens_;
    }

    std::vector<std::string> types() const override {
        return std::vector<std::string>(CREDENTIAL_TYPES.begin(), CREDENTIAL_TYPES.end());
    }

    bool accepts(const std::string &operation, const Params &params, std::string &why) const override {
        const std::string name = argument(params, "name");
        if (name.empty() || name.size() > CREDENTIAL_NAME_MAX) {
            why = "name is empty or past its bound";
            return false;
        }
        if (operation == CREDENTIAL_ISSUE && !ttlSeconds(argument(params, "ttl_s"))) {
            why = "ttl_s is not a count of seconds from 1 to " + std::to_string(std::numeric_limits<uint32_t>::max());
            return false;
        }
        return true;
    }

    Outcome failure(const std::string &operation, const std::string &code) const override {
        if (operation == CREDENTIAL_ISSUE) {
            return detail::failed(code, {detail::text("token", "")});
        }
        return detail::failed(code, {detail::number("revoked", 0)});
    }

    Outcome run(const std::string &operation, const Invocation &invocation, const Params &params) override {
        const std::string name = argument(params, "name");
        if (operation == CREDENTIAL_ISSUE) {
            return issue(invocation, name, *ttlSeconds(argument(params, "ttl_s")));
        }
        return revoke(name);
    }

private:
    static std::string argument(const Params &params, const char *name) {
        const auto found = params.find(name);
        return found == params.end() ? std::string() : found->second;
    }

    /// An issue's `ttl_s`: its decimal digits as a count from 1 to the largest a uint32 holds, or nothing.
    static std::optional<uint32_t> ttlSeconds(const std::string &digits) {
        if (digits.empty() || digits.size() > 10 || digits.find_first_not_of("0123456789") != std::string::npos) {
            return std::nullopt;
        }
        const unsigned long long seconds = std::stoull(digits);
        if (seconds == 0 || seconds > std::numeric_limits<uint32_t>::max()) {
            return std::nullopt;
        }
        return static_cast<uint32_t>(seconds);
    }

    /// The message whose tag under the issuing key is the token: `[session id, invoke id, start, name]`.
    static Cose::Bytes issueMessage(const Invocation &invocation, const std::string &name) {
        Cose::Bytes message;
        Cose::Cbor::array(message, 4);
        Cose::Cbor::textString(message, invocation.sessionId);
        Cose::Cbor::textString(message, invocation.id);
        Cose::Cbor::head(message, Cose::Cbor::Major::Unsigned, invocation.start);
        Cose::Cbor::textString(message, name);
        return message;
    }

    Outcome issue(const Invocation &invocation, const std::string &name, uint32_t ttlS) {
        const auto reading = clock_.read();
        if (!reading || reading->utcMs < 0) {
            SCE_LOG_INFO("{} {}: the clock gave no time to issue from", CREDENTIAL_ISSUE, invocation.id);
            return failure(CREDENTIAL_ISSUE, "unavailable");
        }
        auto tag = keyRing_.issuingTag(issueMessage(invocation, name));
        if (!tag) {
            SCE_LOG_INFO("{} {}: no tag under the issuing key", CREDENTIAL_ISSUE, invocation.id);
            return failure(CREDENTIAL_ISSUE, "unavailable");
        }
        const auto digits = crypto_.base64Url(*tag);
        crypto_.wipe(*tag);
        if (!digits) {
            return failure(CREDENTIAL_ISSUE, "unavailable");
        }
        const std::string token = TOKEN_PREFIX + *digits;
        const auto digest = crypto_.sha256(Cose::Bytes(token.begin(), token.end()));
        if (!digest) {
            return failure(CREDENTIAL_ISSUE, "unavailable");
        }
        const auto issuedMs = static_cast<uint64_t>(reading->utcMs);
        const uint64_t lifeMs = static_cast<uint64_t>(ttlS) * 1000U;
        const uint64_t expiresMs = issuedMs > std::numeric_limits<uint64_t>::max() - lifeMs
                                       ? std::numeric_limits<uint64_t>::max()
                                       : issuedMs + lifeMs;
        // A token is never replaced: a repeat under its identity keeps the first issue's expiry and its
        // revocation.
        tokens_.emplace(*digest, StoredToken{name, issuedMs, expiresMs, false});
        return detail::record({detail::text("token", token)});
    }

    Outcome revoke(const std::string &name) {
        int64_t revoked = 0;
        for (auto &entry : tokens_) {
            if (entry.second.name == name && !entry.second.revoked) {
                entry.second.revoked = true;
                ++revoked;
            }
        }
        return detail::record({detail::number("revoked", revoked)});
    }

    CoseFeature &keyRing_;
    CoseCrypto &crypto_;
    ClockSource &clock_;
    /// The tokens the host issued, by the SHA-256 of their text.
    std::map<Cose::Bytes, StoredToken> tokens_;
};

}  // namespace SCE::StandardHost
