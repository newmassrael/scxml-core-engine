// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file SealedRecord.h
 * @brief The sealed record a server's store keeps each value as, and the entry it keeps each key as
 *        (docs/SCE_SEALED_RECORD.md; docs/adr/0109, D8), on libsodium.
 *
 * A record is `SCE-SEALED-1`, the length of the sealing key's id in one byte, the id, a 192-bit nonce, and the
 * XChaCha20-Poly1305 ciphertext of the value with its 16-byte tag (`crypto_aead_xchacha20poly1305_ietf`). What
 * it authenticates beside the ciphertext is its header, the store's name after its length in one byte, and the
 * row's entry, so a record moved to another row or store, changed by a byte, or opened under another key does
 * not open. An entry is the HMAC-SHA-256 of a key under the store's index key, in lowercase hexadecimal.
 *
 * The C++ host keeps its keys and tokens in memory and has no store of its own: these functions are the record
 * a C++ embedding that keeps rows writes and reads, byte for byte the Rust sealing store's
 * (`sce_rust_crypto::sealed`), which tests/host_features/sealed_cross_language.json holds both to. They need
 * libsodium, as the COSE operations do (cmake/SCEHostCose.cmake).
 */

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include <sodium.h>

namespace SCE::StandardHost::SealedRecord {

using Bytes = std::vector<uint8_t>;

/// What every record begins with: the version of its form.
inline constexpr char MAGIC[] = "SCE-SEALED-1";
/// The bytes of the magic, its terminating NUL left out.
inline constexpr size_t MAGIC_BYTES = sizeof(MAGIC) - 1;
/// The bytes of a sealing key.
inline constexpr size_t KEY_BYTES = crypto_aead_xchacha20poly1305_ietf_KEYBYTES;
/// The bytes of a record's nonce: 192 bits.
inline constexpr size_t NONCE_BYTES = crypto_aead_xchacha20poly1305_ietf_NPUBBYTES;
/// The bytes of a record's tag.
inline constexpr size_t TAG_BYTES = crypto_aead_xchacha20poly1305_ietf_ABYTES;
/// The most bytes a sealing key's id or a store's name has: each is written after its length in one byte.
inline constexpr size_t NAME_MAX = 255;

static_assert(MAGIC_BYTES == 12, "the magic is twelve bytes");
static_assert(KEY_BYTES == 32 && NONCE_BYTES == 24 && TAG_BYTES == 16, "XChaCha20-Poly1305 in its IETF form");

namespace detail {

/// Whether libsodium can be used; its initialisation is idempotent.
inline bool ready() {
    return sodium_init() >= 0;
}

/// A pointer to `bytes` the library may read, which for none is a byte it reads nothing of.
inline const unsigned char *in(const Bytes &bytes) {
    static const unsigned char none = 0;
    return bytes.empty() ? &none : bytes.data();
}

/// `text` after its length in one byte, appended to `out`: nothing, and false, for a text that is empty when
/// `nonEmpty` or longer than NAME_MAX.
inline bool appendName(Bytes &out, const std::string &text, bool nonEmpty) {
    if (text.size() > NAME_MAX || (nonEmpty && text.empty())) {
        return false;
    }
    out.push_back(static_cast<uint8_t>(text.size()));
    out.insert(out.end(), text.begin(), text.end());
    return true;
}

/// The header of a record sealed under `keyId`.
inline std::optional<Bytes> header(const std::string &keyId) {
    Bytes out(MAGIC, MAGIC + MAGIC_BYTES);
    if (!appendName(out, keyId, true)) {
        return std::nullopt;
    }
    return out;
}

/// What a record authenticates: its header, the store's name after its length, the row's entry.
inline std::optional<Bytes> associated(const Bytes &head, const std::string &store, const Bytes &indexed) {
    Bytes out = head;
    if (!appendName(out, store, false)) {
        return std::nullopt;
    }
    out.insert(out.end(), indexed.begin(), indexed.end());
    return out;
}

/// A record read apart: where its header ends, and its key id.
struct Parts {
    size_t headerBytes = 0;
    std::string keyId;
};

inline std::optional<Parts> parts(const Bytes &record) {
    if (record.size() < MAGIC_BYTES + 1 || !std::equal(MAGIC, MAGIC + MAGIC_BYTES, record.begin())) {
        return std::nullopt;
    }
    const size_t length = record[MAGIC_BYTES];
    if (length == 0 || record.size() < MAGIC_BYTES + 1 + length + NONCE_BYTES + TAG_BYTES) {
        return std::nullopt;
    }
    const auto id = record.begin() + static_cast<std::ptrdiff_t>(MAGIC_BYTES + 1);
    return Parts{MAGIC_BYTES + 1 + length, std::string(id, id + static_cast<std::ptrdiff_t>(length))};
}

}  // namespace detail

/// The record of `plaintext` sealed under `key`, whose id is `keyId`, with `nonce`, for the row of the store
/// `store` whose entry is `indexed`; nothing for a key or nonce of another size, an id that is empty or past
/// NAME_MAX bytes, a store's name past it, or a library that cannot be used. A store draws a fresh nonce for
/// every record (`sealFresh`); a test that holds a sealing to its bytes gives one.
inline std::optional<Bytes> seal(const std::string &keyId, const Bytes &key, const Bytes &nonce,
                                 const std::string &store, const Bytes &indexed, const Bytes &plaintext) {
    if (!detail::ready() || key.size() != KEY_BYTES || nonce.size() != NONCE_BYTES) {
        return std::nullopt;
    }
    auto head = detail::header(keyId);
    if (!head) {
        return std::nullopt;
    }
    const auto aad = detail::associated(*head, store, indexed);
    if (!aad) {
        return std::nullopt;
    }
    Bytes record = *head;
    const size_t at = record.size();
    record.insert(record.end(), nonce.begin(), nonce.end());
    record.resize(at + NONCE_BYTES + plaintext.size() + TAG_BYTES);
    unsigned long long written = 0;
    if (crypto_aead_xchacha20poly1305_ietf_encrypt(record.data() + at + NONCE_BYTES, &written, detail::in(plaintext),
                                                   plaintext.size(), detail::in(*aad), aad->size(), nullptr,
                                                   nonce.data(), key.data()) != 0) {
        return std::nullopt;
    }
    record.resize(at + NONCE_BYTES + static_cast<size_t>(written));
    return record;
}

/// `plaintext` sealed as `seal` seals it, with a nonce drawn from the operating system (`randombytes_buf`).
inline std::optional<Bytes> sealFresh(const std::string &keyId, const Bytes &key, const std::string &store,
                                      const Bytes &indexed, const Bytes &plaintext) {
    if (!detail::ready()) {
        return std::nullopt;
    }
    Bytes nonce(NONCE_BYTES);
    randombytes_buf(nonce.data(), nonce.size());
    return seal(keyId, key, nonce, store, indexed, plaintext);
}

/// The id of the key `record` names as the one that sealed it; nothing for bytes that are no record.
inline std::optional<std::string> keyIdOf(const Bytes &record) {
    const auto split = detail::parts(record);
    if (!split) {
        return std::nullopt;
    }
    return split->keyId;
}

/// The plaintext `record` holds, opened under `key` for the row of the store `store` whose entry is `indexed`;
/// nothing when it was sealed for another row or store, under another key, changed since, or is no record.
/// The key is the one `keyIdOf` names.
inline std::optional<Bytes> open(const Bytes &key, const std::string &store, const Bytes &indexed,
                                 const Bytes &record) {
    if (!detail::ready() || key.size() != KEY_BYTES) {
        return std::nullopt;
    }
    const auto split = detail::parts(record);
    if (!split) {
        return std::nullopt;
    }
    const Bytes head(record.begin(), record.begin() + static_cast<std::ptrdiff_t>(split->headerBytes));
    const auto aad = detail::associated(head, store, indexed);
    if (!aad) {
        return std::nullopt;
    }
    const unsigned char *nonce = record.data() + split->headerBytes;
    const unsigned char *sealed = nonce + NONCE_BYTES;
    const size_t sealedBytes = record.size() - split->headerBytes - NONCE_BYTES;
    // One byte at least, so the library is handed a buffer even for an empty plaintext.
    Bytes plaintext(sealedBytes - TAG_BYTES + 1);
    unsigned long long written = 0;
    if (crypto_aead_xchacha20poly1305_ietf_decrypt(plaintext.data(), &written, nullptr, sealed, sealedBytes,
                                                   detail::in(*aad), aad->size(), nonce, key.data()) != 0) {
        sodium_memzero(plaintext.data(), plaintext.size());
        return std::nullopt;
    }
    plaintext.resize(static_cast<size_t>(written));
    return plaintext;
}

/// The keyed digest a store keeps `bytes` as: their HMAC-SHA-256 under `indexKey`.
inline std::optional<Bytes> keyedDigest(const Bytes &indexKey, const Bytes &bytes) {
    if (!detail::ready()) {
        return std::nullopt;
    }
    crypto_auth_hmacsha256_state state;
    Bytes digest(crypto_auth_hmacsha256_BYTES);
    const bool made = crypto_auth_hmacsha256_init(&state, detail::in(indexKey), indexKey.size()) == 0 &&
                      crypto_auth_hmacsha256_update(&state, detail::in(bytes), bytes.size()) == 0 &&
                      crypto_auth_hmacsha256_final(&state, digest.data()) == 0;
    sodium_memzero(&state, sizeof state);
    if (!made) {
        return std::nullopt;
    }
    return digest;
}

/// The entry a store keeps the row of `key` under: the lowercase hexadecimal of the keyed digest of its UTF-8
/// bytes. What an SQL read of the medium shows in place of the key.
inline std::optional<std::string> digestOf(const Bytes &indexKey, const std::string &key) {
    const auto digest = keyedDigest(indexKey, Bytes(key.begin(), key.end()));
    if (!digest) {
        return std::nullopt;
    }
    static const char digits[] = "0123456789abcdef";
    std::string text;
    text.reserve(digest->size() * 2);
    for (const uint8_t byte : *digest) {
        text.push_back(digits[byte >> 4]);
        text.push_back(digits[byte & 0x0F]);
    }
    return text;
}

}  // namespace SCE::StandardHost::SealedRecord
