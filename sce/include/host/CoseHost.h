// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file CoseHost.h
 * @brief The COSE operations of the standard host (docs/adr/0107, D7), random bytes, keyed digests and keys
 *        (docs/adr/0109, D5 to D7) as a feature of the reference host: sign and verify a `COSE_Sign1`, seal
 *        and open a `COSE_Encrypt0`, derive, generate, export and retire a key, draw random bytes, and tag
 *        and verify a message with HMAC-SHA-256.
 *
 * The algorithms are fixed, not negotiated: EdDSA on Ed25519, ChaCha20/Poly1305 with a 96-bit nonce the
 * host draws from its random source, HKDF with SHA-256 and HMAC-SHA-256. No key enters a statechart: a
 * machine names keys by id. The signing keys, the sealing keys, the MAC keys and the trust sets are the
 * host's configuration; the keys the host generates and derives are its key ring's, each of one use, its
 * material wiped when it leaves memory (docs/SCE_HOST_FEATURES.md, section 13). This host keeps its key ring
 * in memory, for its own life: the durable key store with a key-encryption key is the Rust host core's
 * (`sce-rust-session-store`, `sce_rust_crypto::keyring`).
 *
 * The primitives come through `CoseCrypto` (host/CoseCrypto.h), which the embedding implements on a
 * cryptographic library it links (host/SodiumCoseCrypto.h, on libsodium); the structures come from
 * host/Cose.h. So the library, the structures and the operations change apart.
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
#include "host/CoseCrypto.h"
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
/// Draw random bytes.
inline constexpr const char *RANDOM_BYTES = "sce-host-random-bytes";
/// Tag a message with HMAC-SHA-256.
inline constexpr const char *MAC = "sce-host-mac";
/// Verify a tag.
inline constexpr const char *MAC_VERIFY = "sce-host-mac-verify";
/// Generate a key of one use.
inline constexpr const char *KEY_GENERATE = "sce-host-key-generate";
/// Export a signing key's public key as a COSE_Key.
inline constexpr const char *KEY_PUBLIC = "sce-host-key-public";
/// Retire a key of the key ring.
inline constexpr const char *KEY_RETIRE = "sce-host-key-retire";

/// Every operation type of the feature, in the order the specification lists them.
inline constexpr std::array<const char *, 11> COSE_TYPES = {COSE_SIGN1,   COSE_VERIFY1, COSE_ENCRYPT0, COSE_DECRYPT0,
                                                            KEY_DERIVE,   RANDOM_BYTES, MAC,           MAC_VERIFY,
                                                            KEY_GENERATE, KEY_PUBLIC,   KEY_RETIRE};

/// The bounds of the records' fields that no build moves, in bytes. The payload, the plaintext, the objects
/// made of them, a keyed digest's message and a draw are named bounds, which the feature holds as its build
/// gave them (NamedBounds).
namespace CoseBounds {
inline constexpr size_t KEY_ID = 64;
inline constexpr size_t TRUST_SET = 64;
inline constexpr size_t EXTERNAL_AAD = 256;
inline constexpr size_t SECRET = 256;
inline constexpr size_t SALT = 64;
inline constexpr size_t INFO = 256;
inline constexpr size_t TAG = 32;
inline constexpr size_t LABEL = 64;
}  // namespace CoseBounds

/// The numbers of the named bounds of the records the feature judges a request and its answer by, in bytes
/// (docs/adr/0109, D17). Each is a bound the library's records name (`bytes[cose_payload=4096]`), which a
/// build moves with `sce-codegen --limit name=N`; the machine the build writes declares each it names as a
/// static member of `<Machine>Limits` (`COSE_PAYLOAD` and its neighbours), and the embedding gives the feature
/// those numbers. A value built by default holds the library's numbers, the bounds of a build given no limit.
struct NamedBounds {
    /// `cose_payload`: what a signing signs and a verification answers.
    size_t cosePayload = 4096;
    /// `cose_signed`: the object a signing makes and a verification reads.
    size_t coseSigned = 4352;
    /// `cose_plaintext`: what a seal seals and an opening answers.
    size_t cosePlaintext = 4352;
    /// `cose_sealed`: the object a seal makes and an opening reads.
    size_t coseSealed = 4480;
    /// `mac_message`: the message a keyed digest tags or verifies.
    size_t macMessage = 4096;
    /// `random_bytes`: the most bytes a draw gives.
    size_t randomBytes = 256;
};

/// The length of an Ed25519 seed, of a ChaCha20/Poly1305 key and of a generated or derived key.
inline constexpr size_t COSE_KEY_BYTES = 32;

/// The prefix of the id a derived key is named by.
inline constexpr const char *DERIVED_KEY_PREFIX = "derived:";
/// The prefix of the id a generated key is named by.
inline constexpr const char *GENERATED_KEY_PREFIX = "generated:";
/// The random bytes a generated key's id is spelled from, drawn before its key's.
inline constexpr size_t KEY_ID_RANDOM_BYTES = 16;
/// The bytes of a derivation's naming digest a derived key's id is spelled from.
inline constexpr size_t DERIVED_ID_BYTES = 16;

/// HKDF with SHA-256 (RFC 5869): `length` bytes, at most 255 times 32, of the key `ikm` gives with `salt`
/// and `info`. An empty salt is the 32 zero bytes the RFC names, which HMAC pads an empty key to.
inline std::optional<Cose::Bytes> hkdfSha256(CoseCrypto &crypto, const Cose::Bytes &salt, const Cose::Bytes &ikm,
                                             const Cose::Bytes &info, size_t length) {
    constexpr size_t hashBytes = 32;
    if (length > 255 * hashBytes) {
        return std::nullopt;
    }
    auto prk = crypto.hmacSha256(salt.empty() ? Cose::Bytes(hashBytes, uint8_t{0}) : salt, ikm);
    if (!prk) {
        return std::nullopt;
    }
    Cose::Bytes okm;
    Cose::Bytes block;
    for (unsigned counter = 1; okm.size() < length; ++counter) {
        Cose::Bytes input = block;
        input.insert(input.end(), info.begin(), info.end());
        input.push_back(static_cast<uint8_t>(counter));
        auto next = crypto.hmacSha256(*prk, input);
        crypto.wipe(input);
        if (!next) {
            crypto.wipe(*prk);
            crypto.wipe(block);
            crypto.wipe(okm);
            return std::nullopt;
        }
        crypto.wipe(block);
        block = *next;
        crypto.wipe(*next);
        okm.insert(okm.end(), block.begin(), block.end());
    }
    crypto.wipe(*prk);
    crypto.wipe(block);
    okm.resize(length);
    return okm;
}

/// What a key is for. A key the host generates has one use; a derived key seals, opens and derives, as a
/// derived key always has (docs/SCE_HOST_FEATURES.md, sections 10 and 13).
enum class KeyUse { Sign, Seal, Mac, Derive, Derived };

/// What an operation does with a key, which decides the uses it takes and whether a retired key serves it.
enum class KeyPurpose { Sign, Seal, Open, Tag, Verify, DeriveFrom, Export };

/// Whether a key of `use` serves `purpose`.
inline bool keyServes(KeyPurpose purpose, KeyUse use) {
    switch (purpose) {
    case KeyPurpose::Sign:
    case KeyPurpose::Export:
        return use == KeyUse::Sign;
    case KeyPurpose::Seal:
    case KeyPurpose::Open:
        return use == KeyUse::Seal || use == KeyUse::Derived;
    case KeyPurpose::Tag:
    case KeyPurpose::Verify:
        return use == KeyUse::Mac;
    case KeyPurpose::DeriveFrom:
        return use == KeyUse::Derive || use == KeyUse::Derived;
    }
    return false;
}

/// Whether `purpose` makes something new under a key, which a retired key no longer does.
inline bool keyProtects(KeyPurpose purpose) {
    return purpose == KeyPurpose::Sign || purpose == KeyPurpose::Seal || purpose == KeyPurpose::Tag ||
           purpose == KeyPurpose::DeriveFrom;
}

/// The use a generation names: `sign`, `seal`, `mac` or `derive`, and nothing else.
inline std::optional<KeyUse> generatedUse(const std::string &name) {
    if (name == "sign") {
        return KeyUse::Sign;
    }
    if (name == "seal") {
        return KeyUse::Seal;
    }
    if (name == "mac") {
        return KeyUse::Mac;
    }
    if (name == "derive") {
        return KeyUse::Derive;
    }
    return std::nullopt;
}

/// Whether `id` names a key of the host's key ring, which no configured key's id may.
inline bool isKeyRingId(const std::string &id) {
    return id.rfind(GENERATED_KEY_PREFIX, 0) == 0 || id.rfind(DERIVED_KEY_PREFIX, 0) == 0;
}

/// The first `count` bytes of `bytes` in lowercase hexadecimal, two digits a byte.
inline std::string lowerHex(const Cose::Bytes &bytes, size_t count) {
    static const char *digits = "0123456789abcdef";
    std::string out;
    for (size_t k = 0; k < count && k < bytes.size(); ++k) {
        out += digits[bytes[k] >> 4];
        out += digits[bytes[k] & 0x0Fu];
    }
    return out;
}

/// The operations, over the primitives of `crypto`, the keys the embedding configures and the host's key
/// ring.
///
/// - Sign1 signs with the signing key `key_id` names, configured or generated; its protected header
///   carries `alg` EdDSA and `kid`, the id's bytes.
/// - Verify1 answers `valid` when the object is a tagged COSE_Sign1 whose protected `alg` is EdDSA, which
///   names no critical header, whose payload is within the result's bound, and whose signature verifies
///   over the `Sig_structure` of its protected header as carried, `external_aad` and its payload under a
///   key of the trust set: the one its `kid` (of either header) names, or without a `kid` the first of
///   the set, in its order, that verifies it. `key_id` is then that key's id and `payload` the payload.
///   Anything else is `valid` false, not a failure.
/// - Encrypt0 seals with the sealing key `key_id` names (configured, generated `seal` or derived), under
///   a nonce from the random source; `kid` and the nonce are unprotected.
/// - Decrypt0 answers `opened` when the object is a tagged COSE_Encrypt0 whose protected `alg` is
///   ChaCha20/Poly1305, which names no critical header and no partial IV, whose nonce is 12 bytes, and
///   whose ciphertext opens under the key `key_id` names with the `Enc_structure` as additional data. The
///   object's `kid` is not read: the request names the key.
/// - KeyDerive derives 32 bytes with HKDF-SHA-256 from `secret`, or from the key `input_key_id` names (a
///   sealing, generated `derive` or derived key), and keeps them as a derived key under the id computed
///   from what derived it.
/// - RandomBytes draws `length` bytes; Mac tags a message under a MAC key and MacVerify compares a tag in
///   constant time; KeyGenerate, KeyPublic and KeyRetire keep the life of a key of the key ring.
///
/// Failures are `unavailable` (the library or the random source could not be used), `unknown_key` (a key
/// id or trust set that names nothing of the kind the operation uses) and `key_retired` (a retired key
/// asked to make something new). A request is refused (`invalid_argument`) when its `key_id` or
/// `trust_set` is empty, when a byte string is not one or is past its bound, when a derivation names both
/// or neither of `secret` and `input_key_id`, when a draw's `length` is 0 or past its bound, or when a
/// generation names no use of the four.
class CoseFeature : public Feature {
public:
    /// The operations over `crypto`, judging requests and answers by `bounds`: the numbers the build of the
    /// machine it serves gave the records' named bounds, the library's when none are given (docs/adr/0109,
    /// D17). A request past one is refused, and a verification or an opening whose payload or plaintext is
    /// past one answers as one that does not verify or open.
    explicit CoseFeature(CoseCrypto &crypto, NamedBounds bounds = NamedBounds{}) : crypto_(crypto), bounds_(bounds) {}

    /// The bounds the feature judges by.
    const NamedBounds &namedBounds() const {
        return bounds_;
    }

    CoseFeature(const CoseFeature &) = delete;
    CoseFeature &operator=(const CoseFeature &) = delete;

    /// Every key the feature holds is wiped as it is dropped.
    ~CoseFeature() override {
        wipeTable(signing_);
        wipeTable(sealing_);
        wipeTable(mac_);
        for (auto &entry : keys_) {
            crypto_.wipe(entry.second.material);
        }
        crypto_.wipe(naming_);
        crypto_.wipe(issuing_);
    }

    /// The HMAC-SHA-256 of `message` under the key ring's issuing key, the key a host issues its bearer
    /// tokens under (docs/SCE_HOST_FEATURES.md section 14; docs/adr/0109, D3). The key is 32 bytes drawn from
    /// the random source at the first issue and kept, for the life of the host, with the ring's other keys;
    /// it never leaves the ring, and no statechart names it. Nothing when the library could not be started,
    /// the source gave no key, or the library could not tag.
    std::optional<Cose::Bytes> issuingTag(const Cose::Bytes &message) {
        if (!crypto_.ready()) {
            return std::nullopt;
        }
        if (issuing_.empty()) {
            auto drawn = crypto_.random(COSE_KEY_BYTES);
            if (!drawn || drawn->size() != COSE_KEY_BYTES) {
                if (drawn) {
                    crypto_.wipe(*drawn);
                }
                return std::nullopt;
            }
            issuing_ = *drawn;
            crypto_.wipe(*drawn);
        }
        return crypto_.hmacSha256(issuing_, message);
    }

    /// The signing keys, each an Ed25519 seed of 32 bytes by its id, replacing those set before. False,
    /// and nothing replaced, when a seed is not 32 bytes or an id is empty, past its bound, or one of the
    /// key ring's (`generated:`, `derived:`).
    bool setSigningKeys(const std::map<std::string, Cose::Bytes> &seeds) {
        return replaceTable(signing_, seeds, COSE_KEY_BYTES);
    }

    /// The sealing keys, each a ChaCha20/Poly1305 key of 32 bytes by its id, replacing those set before.
    /// False, and nothing replaced, as for the signing keys.
    bool setSealingKeys(const std::map<std::string, Cose::Bytes> &keys) {
        return replaceTable(sealing_, keys, COSE_KEY_BYTES);
    }

    /// The MAC keys, each an HMAC-SHA-256 key of any length by its id, replacing those set before. False,
    /// and nothing replaced, when an id is empty, past its bound, or one of the key ring's.
    bool setMacKeys(const std::map<std::string, Cose::Bytes> &keys) {
        return replaceTable(mac_, keys, std::nullopt);
    }

    /// The trust sets, each the ids of the signing keys a signature by which it trusts, by the set's name,
    /// replacing those set before. An id names a signing key of this host, or else a public key of
    /// `setTrustedKeys`; one that names neither when a signature is verified trusts none.
    void setTrustSets(std::map<std::string, std::vector<std::string>> sets) {
        trust_ = std::move(sets);
    }

    /// The public keys a trust set may name, each an Ed25519 public key of 32 bytes by its id, replacing
    /// those set before: what a host that verifies a signer's objects holds of the signer, which never
    /// signs as it, so that a client verifies what its server signed holding nothing but the server's
    /// public key (docs/adr/0109, D9). A signing key of this host of the same id is tried first. False, and
    /// nothing replaced, when a key is not 32 bytes or an id is empty, past its bound, or one of the key
    /// ring's.
    bool setTrustedKeys(const std::map<std::string, Cose::Bytes> &publicKeys) {
        return replaceTable(trusted_, publicKeys, COSE_KEY_BYTES);
    }

    /// The ids of the keys the host generated and derived, in the order of their bytes.
    std::vector<std::string> keyRingIds() const {
        std::vector<std::string> ids;
        for (const auto &entry : keys_) {
            ids.push_back(entry.first);
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
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("payload", bounds_.cosePayload) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == COSE_VERIFY1) {
            return named("trust_set") && within("trust_set", TRUST_SET) && bytesWithin("cose", bounds_.coseSigned) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == COSE_ENCRYPT0) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("plaintext", bounds_.cosePlaintext) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == COSE_DECRYPT0) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("cose", bounds_.coseSealed) &&
                   bytesWithin("external_aad", EXTERNAL_AAD);
        }
        if (operation == RANDOM_BYTES) {
            if (!drawLength(argument(params, "length"))) {
                why = "length is not a count from 1 to " + std::to_string(bounds_.randomBytes);
                return false;
            }
            return true;
        }
        if (operation == MAC) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("message", bounds_.macMessage);
        }
        if (operation == MAC_VERIFY) {
            return named("key_id") && within("key_id", KEY_ID) && bytesWithin("message", bounds_.macMessage) &&
                   bytesWithin("tag", TAG);
        }
        if (operation == KEY_GENERATE) {
            if (!generatedUse(argument(params, "key_use"))) {
                why = "key_use is none of sign, seal, mac and derive";
                return false;
            }
            return within("label", LABEL);
        }
        if (operation == KEY_PUBLIC || operation == KEY_RETIRE) {
            return named("key_id") && within("key_id", KEY_ID);
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
        if (operation == KEY_DERIVE || operation == KEY_GENERATE) {
            return detail::failed(code, {text("key_id", "")});
        }
        if (operation == RANDOM_BYTES) {
            return detail::failed(code, {text("bytes", "")});
        }
        if (operation == MAC) {
            return detail::failed(code, {text("tag", "")});
        }
        if (operation == MAC_VERIFY) {
            return detail::failed(code, {boolean("valid", false)});
        }
        if (operation == KEY_PUBLIC) {
            return detail::failed(code, {text("cose_key", "")});
        }
        if (operation == KEY_RETIRE) {
            return detail::failed(code, {boolean("retired", false)});
        }
        return detail::failed(code, {text("cose", "")});
    }

    Outcome run(const std::string &operation, const Invocation &invocation, const Params &params) override {
        const std::string &id = invocation.id;
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
        if (operation == RANDOM_BYTES) {
            return randomBytes(id, *drawLength(argument(params, "length")));
        }
        if (operation == MAC) {
            return mac(id, argument(params, "key_id"), bytes(params, "message"));
        }
        if (operation == MAC_VERIFY) {
            return macVerify(id, argument(params, "key_id"), bytes(params, "message"), bytes(params, "tag"));
        }
        if (operation == KEY_GENERATE) {
            return keyGenerate(invocation, *generatedUse(argument(params, "key_use")), argument(params, "label"));
        }
        if (operation == KEY_PUBLIC) {
            return keyPublic(id, argument(params, "key_id"));
        }
        if (operation == KEY_RETIRE) {
            return keyRetire(id, argument(params, "key_id"));
        }
        return keyDerive(id, params);
    }

private:
    /// A key of the key ring: its one use, its material, its label and whether it is retired.
    struct KeptKey {
        KeyUse use;
        Cose::Bytes material;
        std::string label;
        bool retired = false;
    };

    /// The key an operation found for a purpose, or the failure code it answers when there is none.
    struct Found {
        const Cose::Bytes *key = nullptr;
        const char *failure = "unknown_key";
    };

    /// A draw's `length`: its decimal digits as a count from 1 to the bound, or nothing.
    std::optional<size_t> drawLength(const std::string &digits) const {
        if (digits.empty() || digits.size() > 5 || digits.find_first_not_of("0123456789") != std::string::npos) {
            return std::nullopt;
        }
        const auto length = static_cast<size_t>(std::stoul(digits));
        if (length == 0 || length > bounds_.randomBytes) {
            return std::nullopt;
        }
        return length;
    }

    void wipeTable(std::map<std::string, Cose::Bytes> &table) {
        for (auto &entry : table) {
            crypto_.wipe(entry.second);
        }
    }

    /// Replace `table` with `keys`, wiping the keys it held: false, and nothing replaced, when an id is
    /// empty, past its bound or one of the key ring's, or a key is not `size` bytes when one is named.
    bool replaceTable(std::map<std::string, Cose::Bytes> &table, const std::map<std::string, Cose::Bytes> &keys,
                      std::optional<size_t> size) {
        for (const auto &entry : keys) {
            if (entry.first.empty() || entry.first.size() > CoseBounds::KEY_ID || isKeyRingId(entry.first) ||
                (size && entry.second.size() != *size)) {
                return false;
            }
        }
        wipeTable(table);
        table = keys;
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

    /// The key `keyId` names for `purpose`: a key of the key ring of a use that serves it, and not retired
    /// when the purpose makes something new; or a configured key of the table the purpose reads.
    Found keyFor(const std::string &keyId, KeyPurpose purpose) const {
        if (isKeyRingId(keyId)) {
            const auto kept = keys_.find(keyId);
            if (kept == keys_.end() || !keyServes(purpose, kept->second.use)) {
                return {};
            }
            if (kept->second.retired && keyProtects(purpose)) {
                return {nullptr, "key_retired"};
            }
            return {&kept->second.material, nullptr};
        }
        const std::map<std::string, Cose::Bytes> *table = &sealing_;
        if (purpose == KeyPurpose::Sign || purpose == KeyPurpose::Export) {
            table = &signing_;
        } else if (purpose == KeyPurpose::Tag || purpose == KeyPurpose::Verify) {
            table = &mac_;
        }
        const auto found = table->find(keyId);
        if (found == table->end()) {
            return {};
        }
        return {&found->second, nullptr};
    }

    /// The identity a generation is kept under: the invocation's, which a restore starts again.
    static std::string generationOf(const Invocation &invocation) {
        return "[" + detail::jsonString(invocation.sessionId) + "," + detail::jsonString(invocation.id) + "," +
               std::to_string(invocation.start) + "]";
    }

    Outcome sign1(const std::string &id, const std::string &keyId, const Cose::Bytes &payload,
                  const Cose::Bytes &externalAad) {
        const Found seed = keyFor(keyId, KeyPurpose::Sign);
        if (seed.key == nullptr) {
            SCE_LOG_INFO("{} {}: no signing key that signs is named {}", COSE_SIGN1, id, keyId);
            return failure(COSE_SIGN1, seed.failure);
        }
        const Cose::Bytes protectedHeader = Cose::sign1ProtectedHeader(bytesOfText(keyId));
        const auto signature =
            crypto_.ed25519Sign(*seed.key, Cose::sigStructure(protectedHeader, externalAad, payload));
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
        if (object->payload.size() > bounds_.cosePayload) {
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
            // A retired signing key still verifies what it signed; a public key the host only trusts
            // verifies with no seed held.
            std::optional<Cose::Bytes> publicKey;
            const Found seed = keyFor(candidate, KeyPurpose::Export);
            if (seed.key != nullptr) {
                publicKey = crypto_.ed25519PublicKey(*seed.key);
                if (!publicKey) {
                    return failure(COSE_VERIFY1, "unavailable");
                }
            } else if (const auto trusted = trusted_.find(candidate); trusted != trusted_.end()) {
                publicKey = trusted->second;
            } else {
                continue;
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
        const Found key = keyFor(keyId, KeyPurpose::Seal);
        if (key.key == nullptr) {
            SCE_LOG_INFO("{} {}: no key that seals is named {}", COSE_ENCRYPT0, id, keyId);
            return failure(COSE_ENCRYPT0, key.failure);
        }
        const auto nonce = crypto_.random(Cose::NONCE_BYTES);
        if (!nonce || nonce->size() != Cose::NONCE_BYTES) {
            SCE_LOG_INFO("{} {}: the random source could not be read", COSE_ENCRYPT0, id);
            return failure(COSE_ENCRYPT0, "unavailable");
        }
        const Cose::Bytes protectedHeader = Cose::encrypt0ProtectedHeader();
        const auto ciphertext =
            crypto_.aeadSeal(*key.key, *nonce, plaintext, Cose::encStructure(protectedHeader, externalAad));
        if (!ciphertext) {
            return failure(COSE_ENCRYPT0, "unavailable");
        }
        return detail::record(
            {byteField("cose", Cose::encrypt0(protectedHeader, bytesOfText(keyId), *nonce, *ciphertext))});
    }

    Outcome decrypt0(const std::string &id, const std::string &keyId, const Cose::Bytes &cose,
                     const Cose::Bytes &externalAad) {
        const Found key = keyFor(keyId, KeyPurpose::Open);
        if (key.key == nullptr) {
            SCE_LOG_INFO("{} {}: no key that opens is named {}", COSE_DECRYPT0, id, keyId);
            return failure(COSE_DECRYPT0, key.failure);
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
        const auto plaintext = crypto_.aeadOpen(*key.key, *nonce, object->ciphertext,
                                                Cose::encStructure(object->protectedHeader, externalAad));
        if (!plaintext) {
            return unopened("its ciphertext does not open under the key");
        }
        if (plaintext->size() > bounds_.cosePlaintext) {
            return unopened("its plaintext is past the bound of the result");
        }
        return detail::record({detail::boolean("opened", true), byteField("plaintext", *plaintext)});
    }

    /// The key ring's naming key, drawn from the random source at its first derivation; false when the
    /// source gave none.
    bool namingKey() {
        if (!naming_.empty()) {
            return true;
        }
        auto drawn = crypto_.random(COSE_KEY_BYTES);
        if (!drawn || drawn->size() != COSE_KEY_BYTES) {
            if (drawn) {
                crypto_.wipe(*drawn);
            }
            return false;
        }
        naming_ = *drawn;
        crypto_.wipe(*drawn);
        return true;
    }

    Outcome keyDerive(const std::string &id, const Params &params) {
        const Cose::Bytes secret = bytes(params, "secret");
        // The deterministic CBOR of the input as the id names it: the text of the input key's id, or the
        // byte string of the SHA-256 of the secret.
        Cose::Bytes input;
        Cose::Bytes ikm;
        if (!secret.empty()) {
            const auto digest = crypto_.sha256(secret);
            if (!digest) {
                return failure(KEY_DERIVE, "unavailable");
            }
            Cose::Cbor::byteString(input, *digest);
            ikm = secret;
        } else {
            const std::string inputKeyId = argument(params, "input_key_id");
            const Found from = keyFor(inputKeyId, KeyPurpose::DeriveFrom);
            if (from.key == nullptr) {
                SCE_LOG_INFO("{} {}: no key to derive from is named {}", KEY_DERIVE, id, inputKeyId);
                return failure(KEY_DERIVE, from.failure);
            }
            Cose::Cbor::textString(input, inputKeyId);
            ikm = *from.key;
        }
        const Cose::Bytes salt = bytes(params, "salt");
        const Cose::Bytes info = bytes(params, "info");
        if (!namingKey()) {
            crypto_.wipe(ikm);
            return failure(KEY_DERIVE, "unavailable");
        }
        Cose::Bytes named;
        Cose::Cbor::array(named, 3);
        named.insert(named.end(), input.begin(), input.end());
        Cose::Cbor::byteString(named, salt);
        Cose::Cbor::byteString(named, info);
        const auto digest = crypto_.hmacSha256(naming_, named);
        if (!digest) {
            crypto_.wipe(ikm);
            return failure(KEY_DERIVE, "unavailable");
        }
        const std::string keyId = DERIVED_KEY_PREFIX + lowerHex(*digest, DERIVED_ID_BYTES);
        if (keys_.find(keyId) == keys_.end()) {
            auto okm = hkdfSha256(crypto_, salt, ikm, info, COSE_KEY_BYTES);
            if (!okm) {
                crypto_.wipe(ikm);
                return failure(KEY_DERIVE, "unavailable");
            }
            keys_[keyId] = KeptKey{KeyUse::Derived, *okm, std::string(), false};
            crypto_.wipe(*okm);
        }
        crypto_.wipe(ikm);
        SCE_LOG_DEBUG("{} {}: derived {}", KEY_DERIVE, id, keyId);
        return detail::record({detail::text("key_id", keyId)});
    }

    Outcome randomBytes(const std::string &id, size_t length) {
        const auto drawn = crypto_.random(length);
        if (!drawn || drawn->size() != length) {
            SCE_LOG_INFO("{} {}: the random source could not be read", RANDOM_BYTES, id);
            return failure(RANDOM_BYTES, "unavailable");
        }
        return detail::record({byteField("bytes", *drawn)});
    }

    Outcome mac(const std::string &id, const std::string &keyId, const Cose::Bytes &message) {
        const Found key = keyFor(keyId, KeyPurpose::Tag);
        if (key.key == nullptr) {
            SCE_LOG_INFO("{} {}: no key that tags is named {}", MAC, id, keyId);
            return failure(MAC, key.failure);
        }
        const auto tag = crypto_.hmacSha256(*key.key, message);
        if (!tag) {
            return failure(MAC, "unavailable");
        }
        return detail::record({byteField("tag", *tag)});
    }

    Outcome macVerify(const std::string &id, const std::string &keyId, const Cose::Bytes &message,
                      const Cose::Bytes &tag) {
        const Found key = keyFor(keyId, KeyPurpose::Verify);
        if (key.key == nullptr) {
            SCE_LOG_INFO("{} {}: no key that verifies a tag is named {}", MAC_VERIFY, id, keyId);
            return failure(MAC_VERIFY, key.failure);
        }
        const auto expected = crypto_.hmacSha256(*key.key, message);
        if (!expected) {
            return failure(MAC_VERIFY, "unavailable");
        }
        return detail::record({detail::boolean("valid", crypto_.sameSecret(*expected, tag))});
    }

    Outcome keyGenerate(const Invocation &invocation, KeyUse use, const std::string &label) {
        const std::string generation = generationOf(invocation);
        if (const auto made = generations_.find(generation); made != generations_.end()) {
            SCE_LOG_INFO("{} {}: start {} of session {} repeated, answered with its key", KEY_GENERATE, invocation.id,
                         invocation.start, invocation.sessionId);
            return detail::record({detail::text("key_id", made->second)});
        }
        auto drawn = crypto_.random(KEY_ID_RANDOM_BYTES + COSE_KEY_BYTES);
        if (!drawn || drawn->size() != KEY_ID_RANDOM_BYTES + COSE_KEY_BYTES) {
            if (drawn) {
                crypto_.wipe(*drawn);
            }
            SCE_LOG_INFO("{} {}: the random source could not be read", KEY_GENERATE, invocation.id);
            return failure(KEY_GENERATE, "unavailable");
        }
        const std::string keyId = GENERATED_KEY_PREFIX + lowerHex(*drawn, KEY_ID_RANDOM_BYTES);
        if (keys_.find(keyId) == keys_.end()) {
            KeptKey &kept = keys_[keyId];
            kept.use = use;
            kept.material.assign(drawn->begin() + static_cast<std::ptrdiff_t>(KEY_ID_RANDOM_BYTES), drawn->end());
            kept.label = label;
            generations_[generation] = keyId;
        }
        crypto_.wipe(*drawn);
        return detail::record({detail::text("key_id", keyId)});
    }

    Outcome keyPublic(const std::string &id, const std::string &keyId) {
        const Found seed = keyFor(keyId, KeyPurpose::Export);
        if (seed.key == nullptr) {
            SCE_LOG_INFO("{} {}: no signing key is named {}", KEY_PUBLIC, id, keyId);
            return failure(KEY_PUBLIC, seed.failure);
        }
        const auto publicKey = crypto_.ed25519PublicKey(*seed.key);
        if (!publicKey) {
            return failure(KEY_PUBLIC, "unavailable");
        }
        return detail::record({byteField("cose_key", Cose::coseKeyEd25519(bytesOfText(keyId), *publicKey))});
    }

    Outcome keyRetire(const std::string &id, const std::string &keyId) {
        // A configured key is the embedding's, not the key ring's to retire.
        const auto kept = isKeyRingId(keyId) ? keys_.find(keyId) : keys_.end();
        if (kept == keys_.end()) {
            SCE_LOG_INFO("{} {}: the key ring holds no key named {}", KEY_RETIRE, id, keyId);
            return failure(KEY_RETIRE, "unknown_key");
        }
        const bool wasActive = !kept->second.retired;
        kept->second.retired = true;
        return detail::record({detail::boolean("retired", wasActive)});
    }

    CoseCrypto &crypto_;
    NamedBounds bounds_;
    std::map<std::string, Cose::Bytes> signing_;
    std::map<std::string, Cose::Bytes> sealing_;
    std::map<std::string, Cose::Bytes> mac_;
    std::map<std::string, std::vector<std::string>> trust_;
    /// The public keys a trust set may name, which verify and sign nothing (`setTrustedKeys`).
    std::map<std::string, Cose::Bytes> trusted_;
    /// The keys the host generated and derived, by id.
    std::map<std::string, KeptKey> keys_;
    /// The key each generation made, by the identity of the invocation that asked.
    std::map<std::string, std::string> generations_;
    /// The naming key derived keys' ids are computed under; empty until the first derivation.
    Cose::Bytes naming_;
    /// The key bearer tokens are issued under; empty until the first issue.
    Cose::Bytes issuing_;
};

}  // namespace SCE::StandardHost
