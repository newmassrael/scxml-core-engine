// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file CoseCrypto.h
 * @brief The cryptographic primitives a host of SCE holds, as one interface the embedding implements on a
 *        library it links (host/SodiumCoseCrypto.h, on libsodium).
 *
 * The standard host's COSE operations (host/CoseHost.h) and the Mesh `https` binding's message signatures
 * (mesh/transports/HttpsSignature.h, docs/adr/0109 D9) make and check their signatures, digests and random
 * bytes through this one interface, so that each primitive has one implementation in a C++ host. It names
 * no library and no operation of the standard host, so a Mesh client that signs includes it without the
 * rest of the host.
 */

#include <cstddef>
#include <optional>
#include <string>

#include "host/Cose.h"

namespace SCE::StandardHost {

/// The primitives the operations are made of. A primitive that cannot be carried out answers nothing, which
/// the operation reports as `unavailable`.
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
    /// SHA-256 (FIPS 180-4) of `data`.
    virtual std::optional<Cose::Bytes> sha256(const Cose::Bytes &data) = 0;
    /// `size` bytes from the operating system's random source.
    virtual std::optional<Cose::Bytes> random(size_t size) = 0;
    /// Whether `a` and `b` are the same bytes, in time that depends on their lengths alone: the one
    /// comparison of a secret, a tag or a digest in the host (docs/adr/0109, D3 and D6). Strings of
    /// different lengths are not the same; a length is not a secret.
    virtual bool sameSecret(const Cose::Bytes &a, const Cose::Bytes &b) = 0;
    /// Overwrite `bytes` with zeros in a way the compiler does not remove, before key material leaves
    /// memory (docs/adr/0109, D7).
    virtual void wipe(Cose::Bytes &bytes) = 0;
    /// The base64url of `bytes` without padding (RFC 4648 section 5), written in time that does not
    /// depend on the bytes: the text of a bearer token, which is a secret (docs/adr/0109, D3). Nothing
    /// when it cannot be written.
    virtual std::optional<std::string> base64Url(const Cose::Bytes &bytes) = 0;
};

}  // namespace SCE::StandardHost
