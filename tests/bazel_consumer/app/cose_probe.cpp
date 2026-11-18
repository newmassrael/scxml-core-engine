// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Bazel consumer probe of the COSE operations: `@sce//:sce_host_cose` alone
// carries the reference host's headers, the libsodium the consumer named and
// SCE_HOST_COSE, so a channel that stops stating the definition fails here at
// compile time, one that stops linking libsodium at link time, and one that
// links a libsodium that does not work when the probe runs: an Ed25519
// signature made from a seed must verify under the public key of that seed.

#include "host/SodiumCoseCrypto.h"

#include <cstdint>

#if !defined(SCE_HOST_COSE)
#error "@sce//:sce_host_cose states SCE_HOST_COSE to what links it"
#endif

int main() {
    using SCE::StandardHost::Cose::Bytes;
    SCE::StandardHost::SodiumCoseCrypto crypto;
    if (!crypto.ready()) {
        return 1;
    }
    const Bytes seed(32, std::uint8_t{0x07});
    const Bytes message{0x73, 0x63, 0x65};
    const auto publicKey = crypto.ed25519PublicKey(seed);
    const auto signature = crypto.ed25519Sign(seed, message);
    return (publicKey && signature && crypto.ed25519Verify(*publicKey, message, *signature)) ? 0 : 1;
}
