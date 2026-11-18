// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file CoseHost.h
 * @brief The COSE operations of the standard host (docs/adr/0107, D7) as a feature of the reference host:
 *        sign and verify a `COSE_Sign1`, seal and open a `COSE_Encrypt0`, and derive a key.
 *
 * The algorithms are fixed, not negotiated: EdDSA on Ed25519, ChaCha20/Poly1305 with a 96-bit nonce the
 * host draws from the operating system's random source, and HKDF with SHA-256. No key enters a statechart:
 * a machine names keys by id, the signing keys, the sealing keys and the trust sets are the host's
 * configuration, and a derived key lives in the host and is named by the id the derivation answers.
 *
 * The primitives come through `CoseCrypto`, which the embedding implements on a cryptographic library it
 * links (host/SodiumCoseCrypto.h, on libsodium); the structures come from host/Cose.h. So the library, the
 * structures and the operations change apart.
 *
 * A request's byte strings arrive as their Latin-1 text and a result's leave as it, as every `bytes` field
 * crosses (common/Latin1Bytes.h).
 */

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "common/Latin1Bytes.h"
#include "core/LogMacros.h"
#include "host/Cose.h"
#include "host/StandardHost.h"

namespace SCE::StandardHost {

/// Sign a payload into a tagged COSE_Sign1.
inline constexpr const char *COSE_SIGN1 = "sce-host-cose-sign1";
/// Verify a COSE_Sign1 against a trust set.
inline constexpr const char *COSE_VERIFY1 = "sce-host-cose-verify1";
/// Seal a plaintext into a tagged COSE_Encrypt0.
inline constexpr const char *COSE_ENCRYPT0 = "sce-host-cose-encrypt0";
/// Open a COSE_Encrypt0.
inline constexpr const char *COSE_DECRYPT0 = "sce-host-cose-decrypt0";
/// Derive a key with HKDF.
inline constexpr const char *KEY_DERIVE = "sce-host-key-derive";

/// Every operation type of the COSE feature.
inline constexpr std::array<const char *, 5> COSE_TYPES = {COSE_SIGN1, COSE_VERIFY1, COSE_ENCRYPT0, COSE_DECRYPT0,
                                                           KEY_DERIVE};

/// The bounds of the records' fields, in bytes (`sce:max-size` of each).
namespace CoseBounds {
inline constexpr size_t KEY_ID = 64;
inline constexpr size_t TRUST_SET = 64;
inline constexpr size_t PAYLOAD = 4096;
inline constexpr size_t SIGN1 = 4352;
inline constexpr size_t PLAINTEXT = 4352;
inline constexpr size_t ENCRYPT0 = 4480;
inline constexpr size_t EXTERNAL_AAD = 256;
inline constexpr size_t SECRET = 256;
inline constexpr size_t SALT = 64;
inline constexpr size_t INFO = 256;
}  // namespace CoseBounds

/// The length of an Ed25519 seed, of a ChaCha20/Poly1305 key and of a derived key.
inline constexpr size_t COSE_KEY_BYTES = 32;

/// The prefix of the id a derived key is named by: `derived:<n>`, n counting the host's derivations from 1.
inline constexpr const char *DERIVED_KEY_PREFIX = "derived:";

/// The primitives the COSE operations are made of. A primitive that cannot be carried out answers
/// nothing, which the operation reports as `unavailable`.
class CoseCrypto {
public:
    virtual ~CoseCrypto() = default;
    /// Whether the library could be started; a host whose library could not answers `unavailable`.
    virtual bool ready() = 0;
    /// The Ed25519 public key of a 32-byte seed (RFC 8032, section 5.1.5).
    virtual std::optional<Cose::Bytes> ed25519PublicKey(const Cose::Bytes &seed) = 0;
    /// The Ed25519 signature of `message` by the key of `seed` (RFC 8032, section 5.1.6).
    virtual std::optional<Cose::Bytes> ed25519Sign(const Cose::Bytes &seed, const Cose::Bytes &message) = 0;
    /// Whether `signature` is the Ed25519 signature of `message` by `publicKey`.
    virtual bool ed25519Verify(const Cose::Bytes &publicKey, const Cose::Bytes &message,
                               const Cose::Bytes &signature) = 0;
    /// ChaCha20/Poly1305 (RFC 8439, section 2.8): the ciphertext followed by its 16-byte tag.
    virtual std::optional<Cose::Bytes> aeadSeal(const Cose::Bytes &key, const Cose::Bytes &nonce,
                                                const Cose::Bytes &plaintext, const Cose::Bytes &aad) = 0;
    /// The plaintext of a ChaCha20/Poly1305 ciphertext and tag, or nothing when the tag does not verify.
    virtual std::optional<Cose::Bytes> aeadOpen(const Cose::Bytes &key, const Cose::Bytes &nonce,
                                                const Cose::Bytes &ciphertext, const Cose::Bytes &aad) = 0;
    /// HMAC-SHA-256 (RFC 2104) of `data` under a key of any length.
    virtual std::optional<Cose::Bytes> hmacSha256(const Cose::Bytes &key, const Cose::Bytes &data) = 0;
    /// `size` bytes from the operating system's random source.
    virtual std::optional<Cose::Bytes> random(size_t size) = 0;
};

/// HKDF with SHA-256 (RFC 5869): `length` bytes, at most 255 times 32, of the key `ikm` gives with `salt`
/// and `info`. An empty salt is the 32 zero bytes the RFC names, which HMAC pads an empty key to.
inline std::optional<Cose::Bytes> hkdfSha256(CoseCrypto &crypto, const Cose::Bytes &salt, const Cose::Bytes &ikm,
                                             const Cose::Bytes &info, size_t length) {
    constexpr size_t hashBytes = 32;
    if (length > 255 * hashBytes) {
        return std::nullopt;
    }
    const auto prk = crypto.hmacSha256(salt.empty() ? Cose::Bytes(hashBytes, uint8_t{0}) : salt, ikm);
    if (!prk) {
        return std::nullopt;
    }
    Cose::Bytes okm;
    Cose::Bytes block;
    for (unsigned counter = 1; okm.size() < length; ++counter) {
        Cose::Bytes input = block;
        input.insert(input.end(), info.begin(), info.end());
        input.push_back(static_cast<uint8_t>(counter));
        const auto next = crypto.hmacSha256(*prk, input);
        if (!next) {
            return std::nullopt;
        }
        block = *next;
        okm.insert(okm.end(), block.begin(), block.end());
    }
    okm.resize(length);
    return okm;
}

/// The COSE operations, over the primitives of `crypto` and the keys the embedding configures.
///
/// - Sign1 signs with the signing key `key_id` names; its protected header carries `alg` EdDSA and `kid`,
///   the id's bytes.
/// - Verify1 answers `valid` when the object is a tagged COSE_Sign1 whose protected `alg` is EdDSA, which
///   names no critical header, whose payload is within the result's bound, and whose signature verifies
///   over the `Sig_structure` of its protected header as carried, `external_aad` and its payload under a
///   key of the trust set: the one its `kid` (of either header) names, or without a `kid` the first of
///   the set, in its order, that verifies it. `key_id` is then that key's id and `payload` the payload.
///   Anything else is `valid` false, not a failure.
/// - Encrypt0 seals with the sealing or derived key `key_id` names, under a nonce from the random source;
///   `kid` and the nonce are unprotected.
/// - Decrypt0 answers `opened` when the object is a tagged COSE_Encrypt0 whose protected `alg` is
///   ChaCha20/Poly1305, which names no critical header and no partial IV, whose nonce is 12 bytes, and
///   whose ciphertext opens under the key `key_id` names with the `Enc_structure` as additional data. The
///   object's `kid` is not read: the request names the key.
/// - KeyDerive derives 32 bytes with HKDF-SHA-256 from `secret`, or from the sealing or derived key
///   `input_key_id` names, and keeps them as a derived key, a sealing key from then on.
///
/// Failures are `unavailable` (the library or the random source could not be used) and `unknown_key` (a
/// key id or trust set that names nothing of the kind the operation uses). A request is refused
/// (`invalid_argument`) when its `key_id` or `trust_set` is empty, when a byte string is not one or is
/// past its bound, or when a derivation names both or neither of `secret` and `input_key_id`.
class CoseFeature : public Feature {
public:
    explicit CoseFeature(CoseCrypto &crypto) : crypto_(crypto) {}

    /// The signing keys, each an Ed25519 seed of 32 bytes by its id, replacing those set before. False,
    /// and nothing replaced, when a seed is not 32 bytes or an id is empty or past its bound.
    bool setSigningKeys(const std::map<std::string, Cose::Bytes> &seeds) {
        if (!keyTable(seeds)) {
            return false;
        }
        signing_ = seeds;
        return true;
    }

    /// The sealing keys, each a ChaCha20/Poly1305 key of 32 bytes by its id, replacing those set before.
    /// False, and nothing replaced, when a key is not 32 bytes, an id is empty or past its bound, or an id
    /// begins with the prefix derived keys are named by.
    bool setSealingKeys(const std::map<std::string, Cose::Bytes> &keys) {
        if (!keyTable(keys)) {
            return false;
        }
        for (const auto &entry : keys) {
            if (entry.first.rfind(DERIVED_KEY_PREFIX, 0) == 0) {
                return false;
            }
        }
        sealing_ = keys;
        return true;
    }

    /// The trust sets, each the ids of the signing keys a signature by which it trusts, by the set's name,
    /// replacing those set before. An id that names no signing key when a signature is verified trusts none.
    void setTrustSets(std::map<std::string, std::vector<std::string>> sets) {
        trust_ = std::move(sets);
    }

    /// The ids of the keys derived so far, in the order derived.
    std::vector<std::string> derived() const {
        std::vector<std::string> ids;
        for (size_t n = 1; n <= derived_.size(); ++n) {
            ids.push_back(DERIVED_KEY_PREFIX + std::to_string(n));
        }
        return ids;
    }

    std::vector<std::string> types() const override {
        return std::vector<std::string>(COSE_TYPES.begin(), COSE_TYPES.end());
    }

    bool accepts(const std::string &operation, const Params &params, std::string &why) const override {
        const auto within = [&params, &why](const char *name, size_t bound) {
            const auto found = params.find(name);
            if (found != params.end() && found->second.size() > bound) {
                why = std::string(name) + " is past its bound";
                return false;
            }
            return true;
        };
        const auto bytesWithin = [&params, &why](const char *name, size_t bound) {
            const auto found = params.find(name);
            if (found == params.end()) {
                return true;
            }
            std::vector<uint8_t> decoded;
            if (const std::string notBytes = Latin1Bytes::bytesOf(found->second, decoded); !notBytes.empty()) {
                why = std::string(name) + " " + notBytes;
                return false;
            }
            if (decoded.size() > bound) {
                why = std::string(name) + " is past its bound";
                return false;
            }
            return true;
        };
        using namespace CoseBounds;
        const auto named = [&params, &why](const char *name) {
            if (argument(params, name).empty()) {
                why = std::string(name) + " is empty";
                return false;
            }
            return true;
        };
        if (operation == COSE_SIGN1) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("payload", PAYLOAD) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == COSE_VERIFY1) {
            return named("trust_set") && within("trust_set", TRUST_SET) && bytesWithin("cose", SIGN1) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == COSE_ENCRYPT0) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("plaintext", PLAINTEXT) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == COSE_DECRYPT0) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("cose", ENCRYPT0) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (!(within("input_key_id", KEY_ID) && bytesWithin("secret", SECRET) && bytesWithin("salt", SALT) &&
              bytesWithin("info", INFO))) {
            return false;
        }
        if (argument(params, "secret").empty() == argument(params, "input_key_id").empty()) {
            why = "a derivation takes exactly one of secret and input_key_id";
            return false;
        }
        return true;
    }

    Outcome failure(const std::string &operation, const std::string &code) const override {
        using detail::boolean;
        using detail::text;
        if (operation == COSE_VERIFY1) {
            return detail::failed(code, {boolean("valid", false), text("key_id", ""), text("payload", "")});
        }
        if (operation == COSE_DECRYPT0) {
            return detail::failed(code, {boolean("opened", false), text("plaintext", "")});
        }
        if (operation == KEY_DERIVE) {
            return detail::failed(code, {text("key_id", "")});
        }
        return detail::failed(code, {text("cose", "")});
    }

    Outcome run(const std::string &operation, const std::string &id, const Params &params) override {
        if (!crypto_.ready()) {
            SCE_LOG_INFO("{} {}: the cryptographic library could not be started", operation, id);
            return failure(operation, "unavailable");
        }
        if (operation == COSE_SIGN1) {
            return sign1(id, argument(params, "key_id"), bytes(params, "payload"), bytes(params, "external_aad"));
        }
        if (operation == COSE_VERIFY1) {
            return verify1(id, argument(params, "trust_set"), bytes(params, "cose"), bytes(params, "external_aad"));
        }
        if (operation == COSE_ENCRYPT0) {
            return encrypt0(id, argument(params, "key_id"), bytes(params, "plaintext"), bytes(params, "external_aad"));
        }
        if (operation == COSE_DECRYPT0) {
            return decrypt0(id, argument(params, "key_id"), bytes(params, "cose"), bytes(params, "external_aad"));
        }
        return keyDerive(id, params);
    }

private:
    /// Whether every key of `table` is 32 bytes under an id a request can name.
    static bool keyTable(const std::map<std::string, Cose::Bytes> &table) {
        for (const auto &entry : table) {
            if (entry.first.empty() || entry.first.size() > CoseBounds::KEY_ID ||
                entry.second.size() != COSE_KEY_BYTES) {
                return false;
            }
        }
        return true;
    }

    static std::string argument(const Params &params, const char *name) {
        const auto found = params.find(name);
        return found == params.end() ? std::string() : found->second;
    }

    /// A byte string of a request the host accepted, so its text spells bytes.
    static Cose::Bytes bytes(const Params &params, const char *name) {
        Cose::Bytes out;
        Latin1Bytes::bytesOf(argument(params, name), out);
        return out;
    }

    static Cose::Bytes bytesOfText(const std::string &text) {
        return Cose::Bytes(text.begin(), text.end());
    }

    static detail::Field byteField(const std::string &name, const Cose::Bytes &value) {
        return detail::text(name, Latin1Bytes::textOf(value));
    }

    /// The key a sealing, opening or derivation names: a sealing key, or one derived before.
    const Cose::Bytes *symmetricKey(const std::string &keyId) const {
        if (const auto found = sealing_.find(keyId); found != sealing_.end()) {
            return &found->second;
        }
        const std::string prefix = DERIVED_KEY_PREFIX;
        if (keyId.size() <= prefix.size() || keyId.compare(0, prefix.size(), prefix) != 0) {
            return nullptr;
        }
        const std::string digits = keyId.substr(prefix.size());
        if (digits.front() == '0' || digits.size() > 9 || digits.find_first_not_of("0123456789") != std::string::npos) {
            return nullptr;
        }
        const auto n = static_cast<size_t>(std::stoul(digits));
        return n <= derived_.size() ? &derived_[n - 1] : nullptr;
    }

    Outcome sign1(const std::string &id, const std::string &keyId, const Cose::Bytes &payload,
                  const Cose::Bytes &externalAad) {
        const auto seed = signing_.find(keyId);
        if (seed == signing_.end()) {
            SCE_LOG_INFO("{} {}: no signing key is named {}", COSE_SIGN1, id, keyId);
            return failure(COSE_SIGN1, "unknown_key");
        }
        const Cose::Bytes protectedHeader = Cose::sign1ProtectedHeader(bytesOfText(keyId));
        const auto signature =
            crypto_.ed25519Sign(seed->second, Cose::sigStructure(protectedHeader, externalAad, payload));
        if (!signature) {
            return failure(COSE_SIGN1, "unavailable");
        }
        return detail::record({byteField("cose", Cose::sign1(protectedHeader, payload, *signature))});
    }

    Outcome verify1(const std::string &id, const std::string &trustSet, const Cose::Bytes &cose,
                    const Cose::Bytes &externalAad) {
        const auto set = trust_.find(trustSet);
        if (set == trust_.end()) {
            SCE_LOG_INFO("{} {}: no trust set is named {}", COSE_VERIFY1, id, trustSet);
            return failure(COSE_VERIFY1, "unknown_key");
        }
        const auto invalid = [&id](const char *why) {
            SCE_LOG_INFO("{} {}: does not verify, {}", COSE_VERIFY1, id, why);
            return detail::record(
                {detail::boolean("valid", false), detail::text("key_id", ""), detail::text("payload", "")});
        };
        const auto object = Cose::readSign1(cose);
        if (!object) {
            return invalid("not a tagged COSE_Sign1 with its payload");
        }
        if (object->protectedHeaders.alg != Cose::ALG_EDDSA) {
            return invalid("its protected algorithm is not EdDSA");
        }
        if (object->protectedHeaders.crit || object->unprotectedHeaders.crit) {
            return invalid("it names a critical header");
        }
        if (object->payload.size() > CoseBounds::PAYLOAD) {
            return invalid("its payload is past the bound of the result");
        }
        // With a kid, only the entry of the set it names is tried; without one, each entry in order.
        const std::optional<Cose::Bytes> &kid =
            object->protectedHeaders.kid ? object->protectedHeaders.kid : object->unprotectedHeaders.kid;
        std::vector<std::string> candidates;
        if (kid) {
            const std::string named(kid->begin(), kid->end());
            if (std::find(set->second.begin(), set->second.end(), named) != set->second.end()) {
                candidates.push_back(named);
            }
        } else {
            candidates = set->second;
        }
        const Cose::Bytes toBeSigned = Cose::sigStructure(object->protectedHeader, externalAad, object->payload);
        for (const std::string &candidate : candidates) {
            const auto seed = signing_.find(candidate);
            if (seed == signing_.end()) {
                continue;
            }
            const auto publicKey = crypto_.ed25519PublicKey(seed->second);
            if (!publicKey) {
                return failure(COSE_VERIFY1, "unavailable");
            }
            if (crypto_.ed25519Verify(*publicKey, toBeSigned, object->signature)) {
                return detail::record({detail::boolean("valid", true), detail::text("key_id", candidate),
                                       byteField("payload", object->payload)});
            }
        }
        return invalid(kid ? "its signature does not verify under the key of the set its kid names"
                           : "its signature verifies under no key of the trust set");
    }

    Outcome encrypt0(const std::string &id, const std::string &keyId, const Cose::Bytes &plaintext,
                     const Cose::Bytes &externalAad) {
        const Cose::Bytes *key = symmetricKey(keyId);
        if (key == nullptr) {
            SCE_LOG_INFO("{} {}: no sealing key is named {}", COSE_ENCRYPT0, id, keyId);
            return failure(COSE_ENCRYPT0, "unknown_key");
        }
        const auto nonce = crypto_.random(Cose::NONCE_BYTES);
        if (!nonce || nonce->size() != Cose::NONCE_BYTES) {
            SCE_LOG_INFO("{} {}: the random source could not be read", COSE_ENCRYPT0, id);
            return failure(COSE_ENCRYPT0, "unavailable");
        }
        const Cose::Bytes protectedHeader = Cose::encrypt0ProtectedHeader();
        const auto ciphertext =
            crypto_.aeadSeal(*key, *nonce, plaintext, Cose::encStructure(protectedHeader, externalAad));
        if (!ciphertext) {
            return failure(COSE_ENCRYPT0, "unavailable");
        }
        return detail::record(
            {byteField("cose", Cose::encrypt0(protectedHeader, bytesOfText(keyId), *nonce, *ciphertext))});
    }

    Outcome decrypt0(const std::string &id, const std::string &keyId, const Cose::Bytes &cose,
                     const Cose::Bytes &externalAad) {
        const Cose::Bytes *key = symmetricKey(keyId);
        if (key == nullptr) {
            SCE_LOG_INFO("{} {}: no sealing key is named {}", COSE_DECRYPT0, id, keyId);
            return failure(COSE_DECRYPT0, "unknown_key");
        }
        const auto unopened = [&id](const char *why) {
            SCE_LOG_INFO("{} {}: does not open, {}", COSE_DECRYPT0, id, why);
            return detail::record({detail::boolean("opened", false), detail::text("plaintext", "")});
        };
        const auto object = Cose::readEncrypt0(cose);
        if (!object) {
            return unopened("not a tagged COSE_Encrypt0 with its ciphertext");
        }
        if (object->protectedHeaders.alg != Cose::ALG_CHACHA20_POLY1305) {
            return unopened("its protected algorithm is not ChaCha20/Poly1305");
        }
        if (object->protectedHeaders.crit || object->unprotectedHeaders.crit || object->protectedHeaders.partialIv ||
            object->unprotectedHeaders.partialIv) {
            return unopened("it names a critical header or a partial IV");
        }
        const std::optional<Cose::Bytes> &nonce =
            object->unprotectedHeaders.iv ? object->unprotectedHeaders.iv : object->protectedHeaders.iv;
        if (!nonce || nonce->size() != Cose::NONCE_BYTES) {
            return unopened("it carries no 12-byte nonce");
        }
        const auto plaintext = crypto_.aeadOpen(*key, *nonce, object->ciphertext,
                                                Cose::encStructure(object->protectedHeader, externalAad));
        if (!plaintext) {
            return unopened("its ciphertext does not open under the key");
        }
        if (plaintext->size() > CoseBounds::PLAINTEXT) {
            return unopened("its plaintext is past the bound of the result");
        }
        return detail::record({detail::boolean("opened", true), byteField("plaintext", *plaintext)});
    }

    Outcome keyDerive(const std::string &id, const Params &params) {
        Cose::Bytes ikm = bytes(params, "secret");
        if (ikm.empty()) {
            const std::string inputKeyId = argument(params, "input_key_id");
            const Cose::Bytes *input = symmetricKey(inputKeyId);
            if (input == nullptr) {
                SCE_LOG_INFO("{} {}: no key to derive from is named {}", KEY_DERIVE, id, inputKeyId);
                return failure(KEY_DERIVE, "unknown_key");
            }
            ikm = *input;
        }
        const auto okm = hkdfSha256(crypto_, bytes(params, "salt"), ikm, bytes(params, "info"), COSE_KEY_BYTES);
        if (!okm) {
            return failure(KEY_DERIVE, "unavailable");
        }
        derived_.push_back(*okm);
        const std::string keyId = DERIVED_KEY_PREFIX + std::to_string(derived_.size());
        SCE_LOG_DEBUG("{} {}: derived {}", KEY_DERIVE, id, keyId);
        return detail::record({detail::text("key_id", keyId)});
    }

    CoseCrypto &crypto_;
    std::map<std::string, Cose::Bytes> signing_;
    std::map<std::string, Cose::Bytes> sealing_;
    std::map<std::string, std::vector<std::string>> trust_;
    /// The derived keys, the n-th named `derived:<n>`.
    std::vector<Cose::Bytes> derived_;
};

}  // namespace SCE::StandardHost
