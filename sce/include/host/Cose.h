// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file Cose.h
 * @brief The COSE structures the standard host's COSE operations make and read (RFC 9052; docs/adr/0107,
 *        D7): a tagged `COSE_Sign1` and `COSE_Encrypt0`, their header maps, and the `Sig_structure` and
 *        `Enc_structure` a signature and a ciphertext cover, in deterministic CBOR (RFC 8949, section 4.2.1).
 *
 * Bytes only: no key and no cryptography is here, so the primitives (host/CoseHost.h, `CoseCrypto`) and
 * these structures change apart. The CBOR is the subset the structures use: integers, byte and text
 * strings, arrays, maps and tags, each of definite length. What it writes is deterministic: every head in
 * its shortest form and a map's keys in the bytewise order of their encodings. What it reads is any
 * well-formed item of definite length, so a structure another implementation wrote in a longer form is
 * still read; a signature covers the protected header as it was carried, never as it is written again.
 */

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <optional>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::StandardHost::Cose {

using Bytes = std::vector<uint8_t>;

/// EdDSA (RFC 9053, section 2.2), on Ed25519 here.
inline constexpr int64_t ALG_EDDSA = -8;
/// ChaCha20/Poly1305 with a 256-bit key and a 128-bit tag (RFC 9053, section 4.3).
inline constexpr int64_t ALG_CHACHA20_POLY1305 = 24;

/// The header labels this host reads or writes (RFC 9052, section 3.1).
inline constexpr int64_t HEADER_ALG = 1;
inline constexpr int64_t HEADER_CRIT = 2;
inline constexpr int64_t HEADER_KID = 4;
inline constexpr int64_t HEADER_IV = 5;
inline constexpr int64_t HEADER_PARTIAL_IV = 6;

/// The CBOR tags of the two structures (RFC 9052, section 2).
inline constexpr uint64_t TAG_ENCRYPT0 = 16;
inline constexpr uint64_t TAG_SIGN1 = 18;

/// The nonce of ChaCha20/Poly1305 (RFC 8439), carried as the `IV` header.
inline constexpr size_t NONCE_BYTES = 12;

namespace Cbor {

/// The major types (RFC 8949, section 3.1).
enum class Major : uint8_t {
    Unsigned = 0,
    Negative = 1,
    Bytes = 2,
    Text = 3,
    Array = 4,
    Map = 5,
    Tag = 6,
    Simple = 7,
};

/// The head of an item: its major type and its argument, in the shortest form (RFC 8949, section 4.2.1).
inline void head(Bytes &out, Major major, uint64_t argument) {
    const auto type = static_cast<uint8_t>(static_cast<unsigned>(major) << 5);
    if (argument < 24) {
        out.push_back(static_cast<uint8_t>(type | argument));
        return;
    }
    unsigned width = 8;
    uint8_t info = 27;
    if (argument <= 0xFFu) {
        width = 1;
        info = 24;
    } else if (argument <= 0xFFFFu) {
        width = 2;
        info = 25;
    } else if (argument <= 0xFFFFFFFFu) {
        width = 4;
        info = 26;
    }
    out.push_back(static_cast<uint8_t>(type | info));
    for (unsigned k = width; k > 0; --k) {
        out.push_back(static_cast<uint8_t>((argument >> (8 * (k - 1))) & 0xFFu));
    }
}

/// An integer: a negative one is written as -1 minus its argument.
inline void integer(Bytes &out, int64_t value) {
    if (value >= 0) {
        head(out, Major::Unsigned, static_cast<uint64_t>(value));
    } else {
        head(out, Major::Negative, static_cast<uint64_t>(-(value + 1)));
    }
}

inline void byteString(Bytes &out, const Bytes &bytes) {
    head(out, Major::Bytes, bytes.size());
    out.insert(out.end(), bytes.begin(), bytes.end());
}

inline void textString(Bytes &out, std::string_view text) {
    head(out, Major::Text, text.size());
    for (const char c : text) {
        out.push_back(static_cast<uint8_t>(c));
    }
}

inline void array(Bytes &out, uint64_t count) {
    head(out, Major::Array, count);
}

inline void tag(Bytes &out, uint64_t number) {
    head(out, Major::Tag, number);
}

/// A map whose keys are the integers given, each with its value already encoded, the keys in the bytewise
/// order of their encodings. The keys are distinct: every caller writes a fixed set of labels.
inline Bytes integerMap(const std::vector<std::pair<int64_t, Bytes>> &entries) {
    std::vector<std::pair<Bytes, Bytes>> encoded;
    encoded.reserve(entries.size());
    for (const auto &entry : entries) {
        Bytes key;
        integer(key, entry.first);
        encoded.emplace_back(std::move(key), entry.second);
    }
    std::sort(
        encoded.begin(), encoded.end(),
        [](const std::pair<Bytes, Bytes> &lhs, const std::pair<Bytes, Bytes> &rhs) { return lhs.first < rhs.first; });
    Bytes out;
    head(out, Major::Map, encoded.size());
    for (const auto &entry : encoded) {
        out.insert(out.end(), entry.first.begin(), entry.first.end());
        out.insert(out.end(), entry.second.begin(), entry.second.end());
    }
    return out;
}

/// One data item as read.
struct Item {
    Major major = Major::Unsigned;
    /// The argument of its head: an unsigned integer's value, a negative one's -1 minus its value, a tag's
    /// number, a simple value or the bits of a float; a string's length, an array's count of items and a
    /// map's count of pairs.
    uint64_t argument = 0;
    /// A byte or text string's content.
    Bytes content;
    /// An array's items; a map's keys and values, alternating; a tag's one item.
    std::vector<Item> items;
};

/// The deepest nesting a read follows. The structures here nest four deep; a header value an
/// implementation adds may nest further, but not without bound.
inline constexpr unsigned MAX_DEPTH = 16;

/// Reads one item after another from bytes.
class Reader {
public:
    explicit Reader(const Bytes &bytes) : bytes_(bytes) {}

    /// The next item, or nothing when the bytes do not begin with a well-formed item of definite length.
    std::optional<Item> next(unsigned depth = 0) {
        if (depth > MAX_DEPTH || at_ >= bytes_.size()) {
            return std::nullopt;
        }
        const uint8_t initial = bytes_[at_++];
        Item item;
        item.major = static_cast<Major>(initial >> 5);
        const auto info = static_cast<uint8_t>(initial & 0x1Fu);
        if (info < 24) {
            item.argument = info;
        } else if (info <= 27) {
            const size_t width = size_t{1} << (info - 24);
            if (remaining() < width) {
                return std::nullopt;
            }
            for (size_t k = 0; k < width; ++k) {
                item.argument = (item.argument << 8) | bytes_[at_++];
            }
        } else {
            // 28 to 30 are reserved; 31 is an indefinite length, which a deterministic encoding never uses.
            return std::nullopt;
        }
        switch (item.major) {
        case Major::Unsigned:
        case Major::Negative:
            return item;
        case Major::Bytes:
        case Major::Text: {
            if (item.argument > remaining()) {
                return std::nullopt;
            }
            const auto length = static_cast<size_t>(item.argument);
            const auto from = bytes_.begin() + static_cast<std::ptrdiff_t>(at_);
            item.content.assign(from, from + static_cast<std::ptrdiff_t>(length));
            at_ += length;
            return item;
        }
        case Major::Array:
        case Major::Map: {
            // Every item takes a byte at least, so a count past what is left is not well-formed.
            const uint64_t pairs = item.major == Major::Map ? 2 : 1;
            if (item.argument > remaining() / pairs) {
                return std::nullopt;
            }
            const auto count = static_cast<size_t>(item.argument * pairs);
            item.items.reserve(count);
            for (size_t k = 0; k < count; ++k) {
                auto inner = next(depth + 1);
                if (!inner) {
                    return std::nullopt;
                }
                item.items.push_back(std::move(*inner));
            }
            return item;
        }
        case Major::Tag: {
            auto inner = next(depth + 1);
            if (!inner) {
                return std::nullopt;
            }
            item.items.push_back(std::move(*inner));
            return item;
        }
        case Major::Simple:
            // A simple value below 32 in a byte of its own is not well-formed (RFC 8949, section 3.3).
            if (info == 24 && item.argument < 32) {
                return std::nullopt;
            }
            return item;
        }
        return std::nullopt;
    }

    /// Whether every byte has been read.
    bool done() const {
        return at_ == bytes_.size();
    }

private:
    size_t remaining() const {
        return bytes_.size() - at_;
    }

    const Bytes &bytes_;
    size_t at_ = 0;
};

/// The one item `bytes` is, or nothing when they are not exactly one well-formed item of definite length.
inline std::optional<Item> decode(const Bytes &bytes) {
    Reader reader(bytes);
    auto item = reader.next();
    if (!item || !reader.done()) {
        return std::nullopt;
    }
    return item;
}

/// The value of an integer item, when it is one an `int64_t` holds.
inline std::optional<int64_t> integerOf(const Item &item) {
    constexpr auto most = static_cast<uint64_t>(std::numeric_limits<int64_t>::max());
    if (item.major == Major::Unsigned && item.argument <= most) {
        return static_cast<int64_t>(item.argument);
    }
    if (item.major == Major::Negative && item.argument <= most) {
        return -1 - static_cast<int64_t>(item.argument);
    }
    return std::nullopt;
}

}  // namespace Cbor

/// What a header map says, of the labels this host reads, and every label it holds.
struct Headers {
    /// The algorithm, when it is an integer one; one named by text is none of this host's.
    std::optional<int64_t> alg;
    std::optional<Bytes> kid;
    std::optional<Bytes> iv;
    bool partialIv = false;
    bool crit = false;
    /// Every label of the map, to hold the two maps of a structure apart.
    std::vector<Cbor::Item> labels;
};

/// Whether two labels are the same label: the same integer or the same text.
inline bool sameLabel(const Cbor::Item &a, const Cbor::Item &b) {
    return a.major == b.major && a.argument == b.argument && a.content == b.content;
}

/// What the header map `map` says, or nothing when it is not one: a label that is not an integer or a
/// text, a label twice, or a value of a label this host reads that is not of its type (RFC 9052, section
/// 3.1).
inline std::optional<Headers> readHeaders(const Cbor::Item &map) {
    using Cbor::Major;
    if (map.major != Major::Map) {
        return std::nullopt;
    }
    Headers headers;
    for (size_t k = 0; k + 1 < map.items.size(); k += 2) {
        const Cbor::Item &label = map.items[k];
        const Cbor::Item &value = map.items[k + 1];
        const bool numbered = label.major == Major::Unsigned || label.major == Major::Negative;
        if (!numbered && label.major != Major::Text) {
            return std::nullopt;
        }
        for (const Cbor::Item &seen : headers.labels) {
            if (sameLabel(seen, label)) {
                return std::nullopt;
            }
        }
        headers.labels.push_back(label);
        const std::optional<int64_t> number = numbered ? Cbor::integerOf(label) : std::nullopt;
        if (!number) {
            continue;
        }
        if (*number == HEADER_ALG) {
            if (value.major != Major::Unsigned && value.major != Major::Negative && value.major != Major::Text) {
                return std::nullopt;
            }
            headers.alg = Cbor::integerOf(value);
        } else if (*number == HEADER_KID || *number == HEADER_IV) {
            if (value.major != Major::Bytes) {
                return std::nullopt;
            }
            (*number == HEADER_KID ? headers.kid : headers.iv) = value.content;
        } else if (*number == HEADER_PARTIAL_IV) {
            headers.partialIv = true;
        } else if (*number == HEADER_CRIT) {
            headers.crit = true;
        }
    }
    return headers;
}

/// The protected header carried as `bucket`, a byte string: an empty one is an empty map (RFC 9052,
/// section 3).
inline std::optional<Headers> readProtected(const Cbor::Item &bucket) {
    if (bucket.major != Cbor::Major::Bytes) {
        return std::nullopt;
    }
    if (bucket.content.empty()) {
        return Headers{};
    }
    const auto map = Cbor::decode(bucket.content);
    if (!map) {
        return std::nullopt;
    }
    return readHeaders(*map);
}

/// Whether no label is in both maps of a structure (RFC 9052, section 3).
inline bool disjoint(const Headers &protectedHeaders, const Headers &unprotectedHeaders) {
    for (const Cbor::Item &label : protectedHeaders.labels) {
        for (const Cbor::Item &other : unprotectedHeaders.labels) {
            if (sameLabel(label, other)) {
                return false;
            }
        }
    }
    return true;
}

// COSE_Key (RFC 9052, section 7; RFC 9053, section 7.2).

/// The parameters of a COSE_Key this host writes, and the values of an Ed25519 public key's.
inline constexpr int64_t KEY_KTY = 1;
inline constexpr int64_t KEY_KID = 2;
inline constexpr int64_t KEY_CRV = -1;
inline constexpr int64_t KEY_X = -2;
inline constexpr int64_t KTY_OKP = 1;
inline constexpr int64_t CRV_ED25519 = 6;

/// The COSE_Key of an Ed25519 public key: `{1: 1, 2: kid, -1: 6, -2: x}`, key type OKP, its key id, curve
/// Ed25519 and the public key, its labels in the bytewise order of their encodings.
inline Bytes coseKeyEd25519(const Bytes &kid, const Bytes &publicKey) {
    Bytes kty;
    Cbor::integer(kty, KTY_OKP);
    Bytes kidValue;
    Cbor::byteString(kidValue, kid);
    Bytes crv;
    Cbor::integer(crv, CRV_ED25519);
    Bytes x;
    Cbor::byteString(x, publicKey);
    return Cbor::integerMap({{KEY_KTY, kty}, {KEY_KID, kidValue}, {KEY_CRV, crv}, {KEY_X, x}});
}

/// The COSE_Key RFC 9679 takes the thumbprint of for an Ed25519 public key: the members its key type
/// requires and nothing else, `{1: 1, -1: 6, -2: x}`, in deterministic order. It names the algorithm and the
/// curve with the key, so it is also how a key is offered where only its bytes would be ambiguous.
inline Bytes coseKeyEd25519Required(const Bytes &publicKey) {
    Bytes kty;
    Cbor::integer(kty, KTY_OKP);
    Bytes crv;
    Cbor::integer(crv, CRV_ED25519);
    Bytes x;
    Cbor::byteString(x, publicKey);
    return Cbor::integerMap({{KEY_KTY, kty}, {KEY_CRV, crv}, {KEY_X, x}});
}

// COSE_Sign1 (RFC 9052, section 4.2).

/// The protected header of a COSE_Sign1 this host makes, as the bytes it carries: `alg` EdDSA and `kid`.
inline Bytes sign1ProtectedHeader(const Bytes &kid) {
    Bytes alg;
    Cbor::integer(alg, ALG_EDDSA);
    Bytes kidValue;
    Cbor::byteString(kidValue, kid);
    return Cbor::integerMap({{HEADER_ALG, alg}, {HEADER_KID, kidValue}});
}

/// The `Sig_structure` of a COSE_Sign1 (RFC 9052, section 4.4): what its signature is made over.
inline Bytes sigStructure(const Bytes &protectedHeader, const Bytes &externalAad, const Bytes &payload) {
    Bytes out;
    Cbor::array(out, 4);
    Cbor::textString(out, "Signature1");
    Cbor::byteString(out, protectedHeader);
    Cbor::byteString(out, externalAad);
    Cbor::byteString(out, payload);
    return out;
}

/// A tagged COSE_Sign1 with its payload attached and no unprotected header.
inline Bytes sign1(const Bytes &protectedHeader, const Bytes &payload, const Bytes &signature) {
    Bytes out;
    Cbor::tag(out, TAG_SIGN1);
    Cbor::array(out, 4);
    Cbor::byteString(out, protectedHeader);
    const Bytes unprotectedHeader = Cbor::integerMap({});
    out.insert(out.end(), unprotectedHeader.begin(), unprotectedHeader.end());
    Cbor::byteString(out, payload);
    Cbor::byteString(out, signature);
    return out;
}

/// A COSE_Sign1 as read.
struct Sign1 {
    /// The protected header as it was carried, which the signature covers.
    Bytes protectedHeader;
    Headers protectedHeaders;
    Headers unprotectedHeaders;
    Bytes payload;
    Bytes signature;
};

/// The COSE_Sign1 `cose` is, or nothing when it is not a well-formed tagged one with its payload attached.
inline std::optional<Sign1> readSign1(const Bytes &cose) {
    using Cbor::Major;
    const auto tagged = Cbor::decode(cose);
    if (!tagged || tagged->major != Major::Tag || tagged->argument != TAG_SIGN1) {
        return std::nullopt;
    }
    const Cbor::Item &body = tagged->items.front();
    if (body.major != Major::Array || body.items.size() != 4) {
        return std::nullopt;
    }
    auto protectedHeaders = readProtected(body.items[0]);
    auto unprotectedHeaders = readHeaders(body.items[1]);
    if (!protectedHeaders || !unprotectedHeaders || !disjoint(*protectedHeaders, *unprotectedHeaders)) {
        return std::nullopt;
    }
    // A detached payload (`nil`) is one this host was not given.
    if (body.items[2].major != Major::Bytes || body.items[3].major != Major::Bytes) {
        return std::nullopt;
    }
    Sign1 out;
    out.protectedHeader = body.items[0].content;
    out.protectedHeaders = std::move(*protectedHeaders);
    out.unprotectedHeaders = std::move(*unprotectedHeaders);
    out.payload = body.items[2].content;
    out.signature = body.items[3].content;
    return out;
}

// COSE_Encrypt0 (RFC 9052, section 5.2).

/// The protected header of a COSE_Encrypt0 this host makes, as the bytes it carries: `alg` ChaCha20/Poly1305.
inline Bytes encrypt0ProtectedHeader() {
    Bytes alg;
    Cbor::integer(alg, ALG_CHACHA20_POLY1305);
    return Cbor::integerMap({{HEADER_ALG, alg}});
}

/// The `Enc_structure` of a COSE_Encrypt0 (RFC 9052, section 5.3): the additional data its ciphertext
/// authenticates.
inline Bytes encStructure(const Bytes &protectedHeader, const Bytes &externalAad) {
    Bytes out;
    Cbor::array(out, 3);
    Cbor::textString(out, "Encrypt0");
    Cbor::byteString(out, protectedHeader);
    Cbor::byteString(out, externalAad);
    return out;
}

/// A tagged COSE_Encrypt0 with its ciphertext attached, `kid` and the nonce (`IV`) unprotected.
inline Bytes encrypt0(const Bytes &protectedHeader, const Bytes &kid, const Bytes &nonce, const Bytes &ciphertext) {
    Bytes out;
    Cbor::tag(out, TAG_ENCRYPT0);
    Cbor::array(out, 3);
    Cbor::byteString(out, protectedHeader);
    Bytes kidValue;
    Cbor::byteString(kidValue, kid);
    Bytes ivValue;
    Cbor::byteString(ivValue, nonce);
    const Bytes unprotectedHeader = Cbor::integerMap({{HEADER_KID, kidValue}, {HEADER_IV, ivValue}});
    out.insert(out.end(), unprotectedHeader.begin(), unprotectedHeader.end());
    Cbor::byteString(out, ciphertext);
    return out;
}

/// A COSE_Encrypt0 as read.
struct Encrypt0 {
    /// The protected header as it was carried, which the ciphertext authenticates.
    Bytes protectedHeader;
    Headers protectedHeaders;
    Headers unprotectedHeaders;
    Bytes ciphertext;
};

/// The COSE_Encrypt0 `cose` is, or nothing when it is not a well-formed tagged one with its ciphertext
/// attached.
inline std::optional<Encrypt0> readEncrypt0(const Bytes &cose) {
    using Cbor::Major;
    const auto tagged = Cbor::decode(cose);
    if (!tagged || tagged->major != Major::Tag || tagged->argument != TAG_ENCRYPT0) {
        return std::nullopt;
    }
    const Cbor::Item &body = tagged->items.front();
    if (body.major != Major::Array || body.items.size() != 3) {
        return std::nullopt;
    }
    auto protectedHeaders = readProtected(body.items[0]);
    auto unprotectedHeaders = readHeaders(body.items[1]);
    if (!protectedHeaders || !unprotectedHeaders || !disjoint(*protectedHeaders, *unprotectedHeaders)) {
        return std::nullopt;
    }
    if (body.items[2].major != Major::Bytes) {
        return std::nullopt;
    }
    Encrypt0 out;
    out.protectedHeader = body.items[0].content;
    out.protectedHeaders = std::move(*protectedHeaders);
    out.unprotectedHeaders = std::move(*unprotectedHeaders);
    out.ciphertext = body.items[2].content;
    return out;
}

}  // namespace SCE::StandardHost::Cose
