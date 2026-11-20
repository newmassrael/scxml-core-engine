// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Message signatures of the Mesh binding over HTTP (SCE_MESH.md section
// 22.9, docs/adr/0109 D9): HTTP Message Signatures (RFC 9421) with Ed25519,
// over a body bound by its digest (RFC 9530), on the C++ client's side. The
// Rust host core's server and client make and check the same bytes
// (backends/rust/mesh/src/https/signature.rs).
//
// Two signatures, each under the label `sce`, with `alg="ed25519"` and
// `tag="sce-mesh-1"`:
//   * A response of status 200, signed by the server and verified here
//     against the trust set the binding names (`verify:`), the key chosen by
//     `keyid`: `@status`, `content-digest`, `sce-mesh-epoch`,
//     `sce-mesh-first` when the response carries one, and of the request it
//     answers (`;req`) `@method`, `@target-uri`, `sce-mesh-ack` and
//     `sce-mesh-nonce`, 16 random bytes the client draws for each exchange.
//   * A request of the `key` authenticator, signed here by the key the host
//     generated once: `@method`, `@target-uri`, `content-digest`,
//     `sce-mesh-ack`, `sce-mesh-epoch` when the request carries one,
//     `sce-mesh-hold-ms` and `sce-mesh-nonce`, with `created`; `keyid` the
//     key's COSE key thumbprint (RFC 9679) in base64url, the key itself
//     offered in `Sce-Mesh-Key`.
//
// What a signature covers is fixed by the profile and by what the message
// carries, never read from the message. The primitives (Ed25519, SHA-256,
// random bytes) are the host's, through host/CoseCrypto.h, which links no
// library: the host hands its implementation (host/SodiumCoseCrypto.h) to the
// client.

#pragma once

#include "host/CoseCrypto.h"
#include "mesh/transports/HttpsTransport.h"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Mesh::Https::Signatures {

/// The field that names what a signature covers and how (RFC 9421 section 4.1).
inline constexpr const char *kSignatureInputHeader = "Signature-Input";
/// The field that carries a signature (RFC 9421 section 4.2).
inline constexpr const char *kSignatureHeader = "Signature";
/// The digest of a message's content (RFC 9530 section 2).
inline constexpr const char *kContentDigestHeader = "Content-Digest";
/// The request header of the client's nonce, base64url without padding.
inline constexpr const char *kNonceHeader = "Sce-Mesh-Nonce";
/// The request header that offers the client's public key: the COSE_Key its
/// thumbprint digests, base64url without padding.
inline constexpr const char *kKeyHeader = "Sce-Mesh-Key";
/// The label both signatures are kept under.
inline constexpr const char *kLabel = "sce";
/// The algorithm of RFC 9421 section 3.3.6.
inline constexpr const char *kAlgorithm = "ed25519";
/// The application tag of RFC 9421 section 2.3.
inline constexpr const char *kTag = "sce-mesh-1";
/// The random bytes of a nonce.
inline constexpr std::size_t kNonceBytes = 16;

using Bytes = std::vector<std::uint8_t>;

// ── Structured field values (RFC 8941), as far as the profile reads and writes them ──

/// A bare item (RFC 8941 section 3.3). A decimal is not one the profile reads.
struct BareItem {
    enum class Kind { Integer, String, Token, ByteSequence, Boolean };
    Kind kind = Kind::Boolean;
    std::int64_t integer = 0;
    /// A string's or a token's text.
    std::string text;
    Bytes bytes;
    bool boolean = true;

    static BareItem ofInteger(std::int64_t value);
    static BareItem ofString(std::string value);
    static BareItem ofBytes(Bytes value);

    bool operator==(const BareItem &other) const;

    bool operator!=(const BareItem &other) const {
        return !(*this == other);
    }
};

/// The parameters of an item or an inner list, in order.
using Parameters = std::vector<std::pair<std::string, BareItem>>;

/// An item and its parameters.
struct Item {
    BareItem bare;
    Parameters parameters;

    bool operator==(const Item &other) const {
        return bare == other.bare && parameters == other.parameters;
    }
};

/// A member of a dictionary: an item, or an inner list with parameters of its own.
struct Member {
    bool innerList = false;
    /// The item, when the member is not an inner list.
    Item item;
    /// The inner list's items and parameters, when it is one.
    std::vector<Item> items;
    Parameters parameters;
};

/// A dictionary's members, in order.
using Dictionary = std::vector<std::pair<std::string, Member>>;

/// The members of a dictionary field (RFC 8941 section 4.2.2); empty for a
/// text that is not one.
std::optional<Dictionary> parseDictionary(std::string_view text);

/// The serialization of an inner list (RFC 8941 section 4.1.1.1): what the
/// `@signature-params` line of a signature base holds.
std::string serializeInnerList(const std::vector<Item> &items, const Parameters &parameters);

// ── The signature base (RFC 9421 section 2.5) ──

/// A component a signature covers: a derived component (`@method`) or a field
/// by its name in lower case, and whether it is read from the request a
/// response answers (`;req`).
struct Component {
    std::string name;
    bool req = false;
};

/// Its component identifier, as a signature's inner list names it.
Item identifier(const Component &component);

/// The signature base of RFC 9421 section 2.5: a line of each covered
/// component and its value, then the `@signature-params` line.
std::string signatureBase(const std::vector<std::pair<Component, std::string>> &covered,
                          const std::string &signatureParams);

// ── What the profile reads and writes ──

/// Base64 (RFC 4648 section 4) with padding: a byte sequence's text.
std::string base64(const Bytes &bytes);
/// The bytes of base64 text, with or without its padding; empty for text that
/// is not base64.
std::optional<Bytes> fromBase64(std::string_view text);

/// The value of the field `name` among `headers`, compared without regard to
/// case: every line of it, each trimmed, joined by ", " (RFC 9421 section
/// 2.1). Empty when there is none.
std::optional<std::string> fieldValue(const std::vector<HttpsHeader> &headers, std::string_view name);

/// A target URI as both ends of the binding read it for `@target-uri`: the
/// scheme and host in lower case, the scheme's own port left out, an empty
/// path written `/`. Empty for a text that is no absolute http or https URI.
std::optional<std::string> normalizeTargetUri(std::string_view url);

/// The Content-Digest of a body (RFC 9530): its SHA-256 as the member
/// `sha-256`. Empty when the host's crypto computed none.
std::optional<std::string> contentDigest(StandardHost::CoseCrypto &crypto, const Bytes &body);

/// Whether `field`, a Content-Digest value, holds the SHA-256 of `body`.
bool digestMatches(StandardHost::CoseCrypto &crypto, std::string_view field, const Bytes &body);

/// The parts of the request a response answers that its signature covers.
struct Answered {
    /// The request's target URI, normalized.
    std::string targetUri;
    /// The request's Sce-Mesh-Ack.
    std::string ack;
    /// The request's Sce-Mesh-Nonce.
    std::string nonce;
};

/// A signer of a trust set: its id and its Ed25519 public key.
struct TrustedKey {
    std::string id;
    Bytes publicKey;
};

/// Why a signed message was not taken.
enum class Refusal {
    /// It was taken.
    None,
    /// No Content-Digest, or one that is not the body's.
    Digest,
    /// No signature under the profile's label, or fields RFC 8941 does not read.
    Unsigned,
    /// A signature that covers another list, names other parameters, or covers
    /// a field the message lacks.
    Profile,
    /// A keyid of no key of the trust set.
    UnknownKey,
    /// The signature does not verify.
    Signature,
};

/// The words a refusal is told with.
const char *refusalText(Refusal refusal);

/// Whether a response of `status` with `headers` and `body` is a 200 a key of
/// `trusted` signed as the profile signs one, answering `answered`; the signer
/// is chosen by `keyid` alone.
Refusal verifyResponse(StandardHost::CoseCrypto &crypto, const std::vector<TrustedKey> &trusted, std::uint16_t status,
                       const std::vector<HttpsHeader> &headers, const Bytes &body, const Answered &answered);

/// What a client signs its requests with under the `key` authenticator: the
/// seed of the Ed25519 key its host generated once, and the id and the offer
/// derived from its public key. The seed is wiped when the signer goes.
class RequestSigner {
public:
    /// The signer of `seed`, through `crypto`, which outlives it; empty when
    /// the seed is not 32 bytes or the crypto derived no public key.
    static std::optional<RequestSigner> make(StandardHost::CoseCrypto &crypto, Bytes seed);

    RequestSigner(const RequestSigner &) = delete;
    RequestSigner &operator=(const RequestSigner &) = delete;
    RequestSigner(RequestSigner &&other) noexcept;
    RequestSigner &operator=(RequestSigner &&other) noexcept;
    ~RequestSigner();

    /// The keyid its signatures name: the base64url of its key's thumbprint.
    const std::string &keyId() const noexcept {
        return keyId_;
    }

    /// The public key.
    const Bytes &publicKey() const noexcept {
        return publicKey_;
    }

    /// The fields that sign a POST to `targetUri` (normalized) whose other
    /// fields are `headers`, made at `created` (seconds since the Unix epoch):
    /// Sce-Mesh-Key, Signature-Input and Signature. `headers` holds
    /// Content-Digest, Sce-Mesh-Ack, Sce-Mesh-Hold-Ms, Sce-Mesh-Nonce and,
    /// when the request names one, Sce-Mesh-Epoch. Empty when one of those is
    /// missing or the crypto signed nothing.
    std::optional<std::vector<HttpsHeader>> sign(const std::string &targetUri, const std::vector<HttpsHeader> &headers,
                                                 std::int64_t created) const;

private:
    RequestSigner(StandardHost::CoseCrypto &crypto, Bytes seed, Bytes publicKey, std::string keyId, std::string offer);

    StandardHost::CoseCrypto *crypto_;
    Bytes seed_;
    Bytes publicKey_;
    std::string keyId_;
    std::string offer_;
};

}  // namespace SCE::Mesh::Https::Signatures
