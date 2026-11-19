// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file SodiumCoseCrypto.h
 * @brief The primitives of the COSE operations (host/CoseHost.h) on libsodium.
 *
 * libsodium is found, not vendored (docs/adr/0107, D7): a cryptographic library takes its security fixes
 * from one place, and a program that already links one gets no second copy. The CMake target
 * `sce_host_cose` (cmake/SCEHostCose.cmake) links the one the build found; an embedding that builds
 * otherwise links its own.
 */

#include <algorithm>
#include <cstddef>
#include <optional>
#include <string>

#include <sodium.h>

#include "host/CoseHost.h"

namespace SCE::StandardHost {

/// The COSE primitives on libsodium: Ed25519 (`crypto_sign_ed25519`), ChaCha20/Poly1305 in its IETF form
/// with a 96-bit nonce (`crypto_aead_chacha20poly1305_ietf`), HMAC-SHA-256 (`crypto_auth_hmacsha256`),
/// SHA-256 (`crypto_hash_sha256`), the operating system's random source (`randombytes_buf`), the comparison
/// of a secret in constant time (`sodium_memcmp`), the wiping of key material (`sodium_memzero`) and the
/// base64url of a bearer token in constant time (`sodium_bin2base64`).
class SodiumCoseCrypto : public CoseCrypto {
public:
    SodiumCoseCrypto() : ready_(sodium_init() >= 0) {}

    bool ready() override {
        return ready_;
    }

    std::optional<Cose::Bytes> ed25519PublicKey(const Cose::Bytes &seed) override {
        if (seed.size() != crypto_sign_ed25519_SEEDBYTES) {
            return std::nullopt;
        }
        Cose::Bytes publicKey(crypto_sign_ed25519_PUBLICKEYBYTES);
        Cose::Bytes secretKey(crypto_sign_ed25519_SECRETKEYBYTES);
        const int made = crypto_sign_ed25519_seed_keypair(publicKey.data(), secretKey.data(), seed.data());
        sodium_memzero(secretKey.data(), secretKey.size());
        if (made != 0) {
            return std::nullopt;
        }
        return publicKey;
    }

    std::optional<Cose::Bytes> ed25519Sign(const Cose::Bytes &seed, const Cose::Bytes &message) override {
        if (seed.size() != crypto_sign_ed25519_SEEDBYTES) {
            return std::nullopt;
        }
        Cose::Bytes publicKey(crypto_sign_ed25519_PUBLICKEYBYTES);
        Cose::Bytes secretKey(crypto_sign_ed25519_SECRETKEYBYTES);
        Cose::Bytes signature(crypto_sign_ed25519_BYTES);
        const bool signedIt =
            crypto_sign_ed25519_seed_keypair(publicKey.data(), secretKey.data(), seed.data()) == 0 &&
            crypto_sign_ed25519_detached(signature.data(), nullptr, in(message), message.size(), secretKey.data()) == 0;
        sodium_memzero(secretKey.data(), secretKey.size());
        if (!signedIt) {
            return std::nullopt;
        }
        return signature;
    }

    bool ed25519Verify(const Cose::Bytes &publicKey, const Cose::Bytes &message,
                       const Cose::Bytes &signature) override {
        if (publicKey.size() != crypto_sign_ed25519_PUBLICKEYBYTES || signature.size() != crypto_sign_ed25519_BYTES) {
            return false;
        }
        return crypto_sign_ed25519_verify_detached(signature.data(), in(message), message.size(), publicKey.data()) ==
               0;
    }

    std::optional<Cose::Bytes> aeadSeal(const Cose::Bytes &key, const Cose::Bytes &nonce, const Cose::Bytes &plaintext,
                                        const Cose::Bytes &aad) override {
        if (key.size() != crypto_aead_chacha20poly1305_ietf_KEYBYTES ||
            nonce.size() != crypto_aead_chacha20poly1305_ietf_NPUBBYTES) {
            return std::nullopt;
        }
        Cose::Bytes ciphertext(plaintext.size() + crypto_aead_chacha20poly1305_ietf_ABYTES);
        unsigned long long written = 0;
        if (crypto_aead_chacha20poly1305_ietf_encrypt(ciphertext.data(), &written, in(plaintext), plaintext.size(),
                                                      in(aad), aad.size(), nullptr, nonce.data(), key.data()) != 0) {
            return std::nullopt;
        }
        ciphertext.resize(static_cast<size_t>(written));
        return ciphertext;
    }

    std::optional<Cose::Bytes> aeadOpen(const Cose::Bytes &key, const Cose::Bytes &nonce, const Cose::Bytes &ciphertext,
                                        const Cose::Bytes &aad) override {
        if (key.size() != crypto_aead_chacha20poly1305_ietf_KEYBYTES ||
            nonce.size() != crypto_aead_chacha20poly1305_ietf_NPUBBYTES ||
            ciphertext.size() < crypto_aead_chacha20poly1305_ietf_ABYTES) {
            return std::nullopt;
        }
        // One byte at least, so the library is handed a buffer even for an empty plaintext.
        Cose::Bytes plaintext(std::max<size_t>(ciphertext.size() - crypto_aead_chacha20poly1305_ietf_ABYTES, 1));
        unsigned long long written = 0;
        if (crypto_aead_chacha20poly1305_ietf_decrypt(plaintext.data(), &written, nullptr, ciphertext.data(),
                                                      ciphertext.size(), in(aad), aad.size(), nonce.data(),
                                                      key.data()) != 0) {
            return std::nullopt;
        }
        plaintext.resize(static_cast<size_t>(written));
        return plaintext;
    }

    std::optional<Cose::Bytes> hmacSha256(const Cose::Bytes &key, const Cose::Bytes &data) override {
        crypto_auth_hmacsha256_state state;
        Cose::Bytes mac(crypto_auth_hmacsha256_BYTES);
        const bool made = crypto_auth_hmacsha256_init(&state, in(key), key.size()) == 0 &&
                          crypto_auth_hmacsha256_update(&state, in(data), data.size()) == 0 &&
                          crypto_auth_hmacsha256_final(&state, mac.data()) == 0;
        sodium_memzero(&state, sizeof state);
        if (!made) {
            return std::nullopt;
        }
        return mac;
    }

    std::optional<Cose::Bytes> sha256(const Cose::Bytes &data) override {
        Cose::Bytes digest(crypto_hash_sha256_BYTES);
        if (crypto_hash_sha256(digest.data(), in(data), data.size()) != 0) {
            return std::nullopt;
        }
        return digest;
    }

    std::optional<Cose::Bytes> random(size_t size) override {
        Cose::Bytes out(size);
        if (size != 0) {
            randombytes_buf(out.data(), size);
        }
        return out;
    }

    bool sameSecret(const Cose::Bytes &a, const Cose::Bytes &b) override {
        if (a.size() != b.size()) {
            return false;
        }
        return a.empty() || sodium_memcmp(a.data(), b.data(), a.size()) == 0;
    }

    void wipe(Cose::Bytes &bytes) override {
        if (!bytes.empty()) {
            sodium_memzero(bytes.data(), bytes.size());
        }
    }

    std::optional<std::string> base64Url(const Cose::Bytes &bytes) override {
        constexpr int variant = sodium_base64_VARIANT_URLSAFE_NO_PADDING;
        // The length libsodium writes includes the terminating NUL.
        std::string text(sodium_base64_ENCODED_LEN(bytes.size(), variant), '\0');
        if (sodium_bin2base64(text.data(), text.size(), in(bytes), bytes.size(), variant) == nullptr) {
            return std::nullopt;
        }
        text.resize(text.size() - 1);
        return text;
    }

private:
    /// A pointer to `bytes` the library may read, which for none is a byte it reads nothing of.
    static const unsigned char *in(const Cose::Bytes &bytes) {
        static const unsigned char none = 0;
        return bytes.empty() ? &none : bytes.data();
    }

    bool ready_;
};

}  // namespace SCE::StandardHost
