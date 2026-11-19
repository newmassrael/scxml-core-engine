// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The COSE operations of the reference host (sce/include/host/CoseHost.h; docs/adr/0107, D7) on
// libsodium (host/SodiumCoseCrypto.h): the deterministic CBOR and the structures of host/Cose.h byte for
// byte, the primitives against the vectors their RFCs publish, and each operation's answers, failures and
// refusals through the reference host.
//
// Compiled into standard_host_scenario_test only in a build that found libsodium, whose target
// `sce_host_cose` it links (cmake/SCEHostCose.cmake).

#include "common/Latin1Bytes.h"
#include "host/Cose.h"
#include "host/CoseHost.h"
#include "host/SodiumCoseCrypto.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <limits>
#include <map>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
namespace cose = SCE::StandardHost::Cose;
using cose::Bytes;

Bytes hex(const std::string &text) {
    const auto bytes = HostScenario::bytesOfHex(text);
    EXPECT_TRUE(bytes.has_value()) << text << " is not hex";
    return bytes.value_or(Bytes{});
}

/// `pair`, a byte in hex, `count` times.
std::string repeated(const std::string &pair, size_t count) {
    std::string out;
    for (size_t k = 0; k < count; ++k) {
        out += pair;
    }
    return out;
}

Bytes ascii(const std::string &text) {
    return Bytes(text.begin(), text.end());
}

/// The text a byte string crosses a request or a result as.
std::string latin1(const Bytes &bytes) {
    return Latin1Bytes::textOf(bytes);
}

Bytes bytesOfLatin1(const std::string &text) {
    Bytes out;
    EXPECT_EQ(Latin1Bytes::bytesOf(text, out), "") << "a result's byte string is its Latin-1 text";
    return out;
}

/// What a primitive answered, or a byte no answer of these tests is, for an answer it did not give.
Bytes answered(const std::optional<Bytes> &answer) {
    return answer.value_or(Bytes{0xEE, 0xEE});
}

Bytes integerItem(int64_t value) {
    Bytes out;
    cose::Cbor::integer(out, value);
    return out;
}

Bytes byteItem(const Bytes &bytes) {
    Bytes out;
    cose::Cbor::byteString(out, bytes);
    return out;
}

/// RFC 8032, section 7.1, TEST 1: the seed, its public key and its signature of the empty message.
const char *const kEd25519Seed = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const char *const kEd25519Public = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const char *const kEd25519Signature = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155"
                                      "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
/// RFC 8439, section 2.8.2: the key of the AEAD example.
const char *const kAeadKey = "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f";

/// The primitives on libsodium, but for a random source that gives the nonce the test names.
class FixedNonceCrypto : public host::SodiumCoseCrypto {
public:
    Bytes nonce = Bytes(cose::NONCE_BYTES, uint8_t{0x5A});

    std::optional<Bytes> random(size_t size) override {
        if (size != nonce.size()) {
            return std::nullopt;
        }
        return nonce;
    }
};

/// Primitives whose library could not be started.
class UnstartedCrypto : public host::SodiumCoseCrypto {
public:
    bool ready() override {
        return false;
    }
};

/// A reference host with the COSE feature over the primitives it is given: two signing keys, `device`
/// (RFC 8032's seed) and `other`, a sealing key `box`, and the trust sets `devices` (of `device`), `both`
/// (of `other`, then `device`) and `none` (of no key).
struct CoseRig {
    explicit CoseRig(host::CoseCrypto &crypto) : feature(crypto) {
        EXPECT_TRUE(reference.addFeature(feature));
        EXPECT_TRUE(feature.setSigningKeys({{"device", hex(kEd25519Seed)}, {"other", Bytes(32, uint8_t{0x42})}}));
        EXPECT_TRUE(feature.setSealingKeys({{"box", hex(kAeadKey)}}));
        feature.setTrustSets({{"devices", {"device"}}, {"both", {"other", "device"}}, {"none", {}}});
    }

    host::Outcome start(const std::string &operation, const host::Params &params) {
        const auto outcome = reference.start(operation, host::Invocation{"session", "op", 1}, params);
        EXPECT_TRUE(outcome.has_value()) << operation << " is not served";
        return outcome.value_or(host::Outcome{});
    }

    /// The result record of a completion of `operation`.
    json done(const std::string &operation, const host::Params &params) {
        const host::Outcome outcome = start(operation, params);
        EXPECT_EQ(outcome.kind, host::Outcome::Kind::Done) << operation << ": " << outcome.data;
        return outcome.kind == host::Outcome::Kind::Done ? json::parse(outcome.data) : json::object();
    }

    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature;
};

/// A tagged COSE_Sign1 of the headers given, both as encoded, signed by `seed` over its Sig_structure.
Bytes signedSign1(host::CoseCrypto &crypto, const Bytes &seed, const Bytes &protectedHeader,
                  const Bytes &unprotectedHeader, const Bytes &payload, const Bytes &externalAad) {
    const auto signature = crypto.ed25519Sign(seed, cose::sigStructure(protectedHeader, externalAad, payload));
    EXPECT_TRUE(signature.has_value());
    Bytes out;
    cose::Cbor::tag(out, cose::TAG_SIGN1);
    cose::Cbor::array(out, 4);
    cose::Cbor::byteString(out, protectedHeader);
    out.insert(out.end(), unprotectedHeader.begin(), unprotectedHeader.end());
    cose::Cbor::byteString(out, payload);
    cose::Cbor::byteString(out, signature.value_or(Bytes{}));
    return out;
}

/// A tagged COSE_Encrypt0 of the headers given, both as encoded, sealed under `key` and `nonce`.
Bytes sealedEncrypt0(host::CoseCrypto &crypto, const Bytes &key, const Bytes &nonce, const Bytes &protectedHeader,
                     const Bytes &unprotectedHeader, const Bytes &plaintext, const Bytes &externalAad) {
    const auto ciphertext = crypto.aeadSeal(key, nonce, plaintext, cose::encStructure(protectedHeader, externalAad));
    EXPECT_TRUE(ciphertext.has_value());
    Bytes out;
    cose::Cbor::tag(out, cose::TAG_ENCRYPT0);
    cose::Cbor::array(out, 3);
    cose::Cbor::byteString(out, protectedHeader);
    out.insert(out.end(), unprotectedHeader.begin(), unprotectedHeader.end());
    cose::Cbor::byteString(out, ciphertext.value_or(Bytes{}));
    return out;
}

}  // namespace

/// Every head in its shortest form (RFC 8949, section 4.2.1), as the examples of its appendix A write
/// them, and the least 64-bit integer.
TEST(AStandardHostCose, CborWritesEachIntegerInItsShortestForm) {
    EXPECT_EQ(integerItem(0), hex("00"));
    EXPECT_EQ(integerItem(23), hex("17"));
    EXPECT_EQ(integerItem(24), hex("1818"));
    EXPECT_EQ(integerItem(100), hex("1864"));
    EXPECT_EQ(integerItem(1000), hex("1903e8"));
    EXPECT_EQ(integerItem(1000000), hex("1a000f4240"));
    EXPECT_EQ(integerItem(1000000000000), hex("1b000000e8d4a51000"));
    EXPECT_EQ(integerItem(-1), hex("20"));
    EXPECT_EQ(integerItem(-10), hex("29"));
    EXPECT_EQ(integerItem(-100), hex("3863"));
    EXPECT_EQ(integerItem(-1000), hex("3903e7"));
    EXPECT_EQ(integerItem(std::numeric_limits<int64_t>::min()), hex("3b7fffffffffffffff"));
    EXPECT_EQ(byteItem(Bytes(24, uint8_t{0})).size(), 26u) << "a 24-byte string takes a one-byte length";
}

/// The headers, the structures a signature and a ciphertext cover, and the two tagged objects, byte for
/// byte as RFC 9052 sections 3, 4.2, 4.4, 5.2 and 5.3 lay them out, map keys in the order of their bytes.
TEST(AStandardHostCose, TheStructuresAreWrittenAsRfc9052LaysThemOut) {
    const Bytes sign1Protected = cose::sign1ProtectedHeader(ascii("ab"));
    EXPECT_EQ(sign1Protected, hex("a2012704426162"));
    Bytes sigStructure = hex("84");
    for (const Bytes &part : {hex("6a"), ascii("Signature1"), hex("47"), sign1Protected, hex("40"), hex("4101")}) {
        sigStructure.insert(sigStructure.end(), part.begin(), part.end());
    }
    EXPECT_EQ(cose::sigStructure(sign1Protected, {}, {0x01}), sigStructure);
    EXPECT_EQ(cose::sign1(sign1Protected, {0x01}, Bytes(64, uint8_t{0x11})),
              hex("d28447a2012704426162a041015840" + repeated("11", 64)));

    const Bytes encrypt0Protected = cose::encrypt0ProtectedHeader();
    EXPECT_EQ(encrypt0Protected, hex("a1011818"));
    EXPECT_EQ(cose::encStructure(encrypt0Protected, {0x01, 0x02}), hex("8368456e63727970743044a1011818420102"));
    EXPECT_EQ(cose::encrypt0(encrypt0Protected, ascii("ab"), Bytes(12, uint8_t{0x5A}), {0x01, 0x02, 0x03}),
              hex("d08344a1011818a20442616205"
                  "4c" +
                  repeated("5a", 12) + "43010203"));
}

/// The reader takes back what the writer wrote, and refuses what is not one well-formed item of definite
/// length, or a header map RFC 9052 section 3 does not allow.
TEST(AStandardHostCose, TheReaderRefusesWhatIsNotAWellFormedObject) {
    const Bytes written = cose::sign1(cose::sign1ProtectedHeader(ascii("k")), {0x07}, Bytes(64, uint8_t{0x22}));
    const auto read = cose::readSign1(written);
    ASSERT_TRUE(read.has_value());
    EXPECT_EQ(read->protectedHeader, cose::sign1ProtectedHeader(ascii("k")));
    EXPECT_EQ(read->protectedHeaders.alg.value_or(0), cose::ALG_EDDSA);
    EXPECT_EQ(read->protectedHeaders.kid.value_or(Bytes{}), ascii("k"));
    EXPECT_EQ(read->payload, Bytes{0x07});
    EXPECT_EQ(read->signature, Bytes(64, uint8_t{0x22}));

    EXPECT_FALSE(cose::Cbor::decode(hex("5f")).has_value()) << "an indefinite length";
    EXPECT_FALSE(cose::Cbor::decode(hex("1c")).has_value()) << "a reserved additional information";
    EXPECT_FALSE(cose::Cbor::decode(hex("1901")).has_value()) << "a head cut short";
    EXPECT_FALSE(cose::Cbor::decode(hex("0000")).has_value()) << "a byte after the item";
    EXPECT_FALSE(cose::Cbor::decode(hex("8501")).has_value()) << "an array longer than the bytes left";
    EXPECT_FALSE(cose::Cbor::decode(hex("f810")).has_value()) << "a simple value below 32 in a byte of its own";
    EXPECT_FALSE(cose::Cbor::decode(hex(repeated("81", 20) + "00")).has_value())
        << "nesting past the depth a read follows";
    EXPECT_TRUE(cose::Cbor::decode(hex("81818100")).has_value());

    EXPECT_FALSE(cose::readSign1(hex("d18440a04040")).has_value()) << "the tag of another structure";
    EXPECT_FALSE(cose::readSign1(hex("8440a04040")).has_value()) << "an untagged object";
    EXPECT_FALSE(cose::readSign1(hex("d28440a0f640")).has_value()) << "a detached payload";
    EXPECT_FALSE(cose::readSign1(hex("d28445a201270127a0410140")).has_value()) << "the label alg twice";
    EXPECT_FALSE(cose::readSign1(hex("d28446a2012704416ba104416b410140")).has_value()) << "kid in both headers";
    EXPECT_FALSE(cose::readSign1(hex("d28443a10401a0410140")).has_value()) << "a kid that is no byte string";
    EXPECT_FALSE(cose::readEncrypt0(hex("d28440a04040")).has_value()) << "a COSE_Sign1 is no COSE_Encrypt0";
}

/// The primitives on libsodium give what RFC 8032 (Ed25519), RFC 4231 (HMAC-SHA-256), RFC 5869 (HKDF) and
/// RFC 8439 (ChaCha20/Poly1305) publish for their examples.
TEST(AStandardHostCose, TheLibsodiumPrimitivesGiveThePublishedVectors) {
    host::SodiumCoseCrypto crypto;
    ASSERT_TRUE(crypto.ready());

    EXPECT_EQ(answered(crypto.ed25519PublicKey(hex(kEd25519Seed))), hex(kEd25519Public));
    EXPECT_EQ(answered(crypto.ed25519Sign(hex(kEd25519Seed), {})), hex(kEd25519Signature));
    EXPECT_TRUE(crypto.ed25519Verify(hex(kEd25519Public), {}, hex(kEd25519Signature)));
    EXPECT_FALSE(crypto.ed25519Verify(hex(kEd25519Public), {0x00}, hex(kEd25519Signature)));

    EXPECT_EQ(answered(crypto.hmacSha256(Bytes(20, uint8_t{0x0b}), ascii("Hi There"))),
              hex("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"));
    EXPECT_EQ(answered(crypto.hmacSha256(Bytes(131, uint8_t{0xaa}),
                                         ascii("Test Using Larger Than Block-Size Key - Hash Key First"))),
              hex("60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"));

    EXPECT_EQ(answered(host::hkdfSha256(crypto, hex("000102030405060708090a0b0c"), Bytes(22, uint8_t{0x0b}),
                                        hex("f0f1f2f3f4f5f6f7f8f9"), 42)),
              hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"));
    EXPECT_EQ(answered(host::hkdfSha256(crypto, {}, Bytes(22, uint8_t{0x0b}), {}, 42)),
              hex("8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"));
    EXPECT_FALSE(host::hkdfSha256(crypto, {}, {0x01}, {}, 255 * 32 + 1).has_value());

    const Bytes key = hex(kAeadKey);
    const Bytes nonce = hex("070000004041424344454647");
    const Bytes aad = hex("50515253c0c1c2c3c4c5c6c7");
    const Bytes plaintext = ascii("Ladies and Gentlemen of the class of '99: If I could offer you only one tip for "
                                  "the future, sunscreen would be it.");
    const auto sealed = crypto.aeadSeal(key, nonce, plaintext, aad);
    ASSERT_TRUE(sealed.has_value());
    ASSERT_EQ(sealed->size(), plaintext.size() + 16);
    EXPECT_EQ(Bytes(sealed->begin(), sealed->begin() + 16), hex("d31a8d34648e60db7b86afbc53ef7ec2"));
    EXPECT_EQ(Bytes(sealed->end() - 16, sealed->end()), hex("1ae10b594f09e26a7e902ecbd0600691"));
    EXPECT_EQ(answered(crypto.aeadOpen(key, nonce, *sealed, aad)), plaintext);
    Bytes forged = *sealed;
    forged.back() ^= 0x01;
    EXPECT_FALSE(crypto.aeadOpen(key, nonce, forged, aad).has_value());

    const auto sealedEmpty = crypto.aeadSeal(key, nonce, {}, {});
    ASSERT_TRUE(sealedEmpty.has_value());
    const auto openedEmpty = crypto.aeadOpen(key, nonce, *sealedEmpty, {});
    ASSERT_TRUE(openedEmpty.has_value()) << "an empty plaintext seals and opens";
    EXPECT_TRUE(openedEmpty->empty());
}

/// A COSE_Sign1 the host made is the one RFC 9052 lays out over the deterministic Ed25519 signature of its
/// Sig_structure, and it verifies under a trust set of its key with the same external data, and under
/// nothing else. A byte string crosses as its Latin-1 text both ways, a NUL and 0xFF included.
TEST(AStandardHostCose, ASignatureVerifiesUnderATrustSetOfItsKeyAndNothingElse) {
    host::SodiumCoseCrypto crypto;
    CoseRig rig(crypto);
    const Bytes payload = {0x00, 0x41, 0x7f, 0x80, 0xff};
    const Bytes aad = ascii("ctx");
    const host::Outcome signing = rig.start(
        host::COSE_SIGN1, {{"key_id", "device"}, {"payload", latin1(payload)}, {"external_aad", latin1(aad)}});
    ASSERT_EQ(signing.kind, host::Outcome::Kind::Done);
    EXPECT_EQ(signing.data.rfind(R"({"error":"",)", 0), 0u) << "error is the first field: " << signing.data;
    const std::string signedText = json::parse(signing.data).at("cose").get<std::string>();
    const Bytes protectedHeader = cose::sign1ProtectedHeader(ascii("device"));
    const auto signature = crypto.ed25519Sign(hex(kEd25519Seed), cose::sigStructure(protectedHeader, aad, payload));
    ASSERT_TRUE(signature.has_value());
    EXPECT_EQ(bytesOfLatin1(signedText), cose::sign1(protectedHeader, payload, *signature));

    const auto verify = [&rig](const std::string &trustSet, const std::string &object, const Bytes &externalAad) {
        return rig.done(host::COSE_VERIFY1,
                        {{"trust_set", trustSet}, {"cose", object}, {"external_aad", latin1(externalAad)}});
    };
    const json valid = {{"error", ""}, {"valid", true}, {"key_id", "device"}, {"payload", latin1(payload)}};
    const json verified = verify("devices", signedText, aad);
    EXPECT_EQ(verified, valid);
    EXPECT_EQ(bytesOfLatin1(verified.at("payload").get<std::string>()), payload);

    const json invalid = {{"error", ""}, {"valid", false}, {"key_id", ""}, {"payload", ""}};
    EXPECT_EQ(verify("devices", signedText, ascii("other")), invalid) << "other external data";
    EXPECT_EQ(verify("none", signedText, aad), invalid) << "a trust set without the key";
    Bytes forged = bytesOfLatin1(signedText);
    forged.back() ^= 0x01;
    EXPECT_EQ(verify("devices", latin1(forged), aad), invalid) << "a signature changed";
    EXPECT_EQ(verify("devices", "not a COSE object", aad), invalid);

    const json byOther =
        rig.done(host::COSE_SIGN1, {{"key_id", "other"}, {"payload", latin1(payload)}, {"external_aad", latin1(aad)}});
    EXPECT_EQ(verify("devices", byOther.at("cose").get<std::string>(), aad), invalid) << "a key the set does not trust";

    const Bytes seed = hex(kEd25519Seed);
    const Bytes kid = byteItem(ascii("device"));
    const Bytes noHeaders = cose::Cbor::integerMap({});
    const Bytes es256 = cose::Cbor::integerMap({{cose::HEADER_ALG, integerItem(-7)}, {cose::HEADER_KID, kid}});
    EXPECT_EQ(verify("devices", latin1(signedSign1(crypto, seed, es256, noHeaders, payload, aad)), aad), invalid)
        << "an object that names another algorithm";
    const Bytes critical = cose::Cbor::integerMap({{cose::HEADER_ALG, integerItem(cose::ALG_EDDSA)},
                                                   {cose::HEADER_CRIT, hex("8118ff")},
                                                   {cose::HEADER_KID, kid}});
    EXPECT_EQ(verify("devices", latin1(signedSign1(crypto, seed, critical, noHeaders, payload, aad)), aad), invalid)
        << "an object that names a critical header";
    const Bytes algOnly = cose::Cbor::integerMap({{cose::HEADER_ALG, integerItem(cose::ALG_EDDSA)}});
    const Bytes kidOnly = cose::Cbor::integerMap({{cose::HEADER_KID, kid}});
    EXPECT_EQ(verify("devices", latin1(signedSign1(crypto, seed, algOnly, kidOnly, payload, aad)), aad), valid)
        << "a kid in the unprotected header names the key as well";
    EXPECT_EQ(verify("devices", latin1(signedSign1(crypto, seed, kidOnly, algOnly, payload, aad)), aad), invalid)
        << "an algorithm the signature does not cover";
    const std::string unnamed = latin1(signedSign1(crypto, seed, algOnly, noHeaders, payload, aad));
    EXPECT_EQ(verify("both", unnamed, aad), valid) << "without a kid, each key of the set is tried in order";
    EXPECT_EQ(verify("both", signedText, aad), valid) << "with a kid, the key of the set it names";
    EXPECT_EQ(verify("none", unnamed, aad), invalid) << "without a kid, and no key in the set";
    const std::string otherKid =
        latin1(signedSign1(crypto, seed, cose::sign1ProtectedHeader(ascii("other")), noHeaders, payload, aad));
    EXPECT_EQ(verify("both", otherKid, aad), invalid) << "a kid that names a key other than the signer's";

    EXPECT_EQ(verify("unknown", signedText, aad),
              (json{{"error", "unknown_key"}, {"valid", false}, {"key_id", ""}, {"payload", ""}}));
    EXPECT_EQ(rig.done(host::COSE_SIGN1, {{"key_id", "unknown"}, {"payload", ""}, {"external_aad", ""}}),
              (json{{"error", "unknown_key"}, {"cose", ""}}));
    EXPECT_EQ(rig.done(host::COSE_SIGN1, {{"key_id", "box"}, {"payload", ""}, {"external_aad", ""}}),
              (json{{"error", "unknown_key"}, {"cose", ""}}))
        << "a sealing key does not sign";
}

/// A COSE_Encrypt0 the host made is the one RFC 9052 lays out, its nonce drawn from the random source and
/// unprotected beside `kid`, and it opens under its key with the same external data, and under nothing else.
TEST(AStandardHostCose, ASealedPlaintextOpensUnderItsKeyAndExternalDataOnly) {
    FixedNonceCrypto crypto;
    CoseRig rig(crypto);
    const Bytes plaintext = {0x00, 0x01, 0xfe, 0xff};
    const Bytes aad = ascii("ctx");
    const json made = rig.done(host::COSE_ENCRYPT0,
                               {{"key_id", "box"}, {"plaintext", latin1(plaintext)}, {"external_aad", latin1(aad)}});
    EXPECT_EQ(made.at("error"), "");
    const std::string sealedText = made.at("cose").get<std::string>();
    const Bytes protectedHeader = cose::encrypt0ProtectedHeader();
    const auto ciphertext =
        crypto.aeadSeal(hex(kAeadKey), crypto.nonce, plaintext, cose::encStructure(protectedHeader, aad));
    ASSERT_TRUE(ciphertext.has_value());
    EXPECT_EQ(bytesOfLatin1(sealedText), cose::encrypt0(protectedHeader, ascii("box"), crypto.nonce, *ciphertext));

    const auto openWith = [&rig](const std::string &keyId, const std::string &object, const Bytes &externalAad) {
        return rig.done(host::COSE_DECRYPT0,
                        {{"key_id", keyId}, {"cose", object}, {"external_aad", latin1(externalAad)}});
    };
    EXPECT_EQ(openWith("box", sealedText, aad),
              (json{{"error", ""}, {"opened", true}, {"plaintext", latin1(plaintext)}}));
    const json unopened = {{"error", ""}, {"opened", false}, {"plaintext", ""}};
    EXPECT_EQ(openWith("box", sealedText, ascii("other")), unopened) << "other external data";
    Bytes forged = bytesOfLatin1(sealedText);
    forged.back() ^= 0x01;
    EXPECT_EQ(openWith("box", latin1(forged), aad), unopened) << "a tag changed";
    EXPECT_EQ(openWith("box", "not a COSE object", aad), unopened);
    EXPECT_EQ(openWith("unknown", sealedText, aad),
              (json{{"error", "unknown_key"}, {"opened", false}, {"plaintext", ""}}));
    EXPECT_EQ(rig.done(host::COSE_ENCRYPT0, {{"key_id", "device"}, {"plaintext", ""}, {"external_aad", ""}}),
              (json{{"error", "unknown_key"}, {"cose", ""}}))
        << "a signing key does not seal";

    const Bytes key = hex(kAeadKey);
    const Bytes nonceItem = byteItem(crypto.nonce);
    const Bytes withNonce = cose::Cbor::integerMap({{cose::HEADER_IV, nonceItem}});
    const Bytes aesGcm = cose::Cbor::integerMap({{cose::HEADER_ALG, integerItem(3)}});
    EXPECT_EQ(
        openWith("box", latin1(sealedEncrypt0(crypto, key, crypto.nonce, aesGcm, withNonce, plaintext, aad)), aad),
        unopened)
        << "an object that names another algorithm";
    const Bytes partial =
        cose::Cbor::integerMap({{cose::HEADER_IV, nonceItem}, {cose::HEADER_PARTIAL_IV, byteItem({0x01})}});
    EXPECT_EQ(openWith("box",
                       latin1(sealedEncrypt0(crypto, key, crypto.nonce, protectedHeader, partial, plaintext, aad)),
                       aad),
              unopened)
        << "an object with a partial IV";
    const Bytes shortNonce = cose::Cbor::integerMap({{cose::HEADER_IV, byteItem(Bytes(8, uint8_t{0x5A}))}});
    EXPECT_EQ(openWith("box",
                       latin1(sealedEncrypt0(crypto, key, crypto.nonce, protectedHeader, shortNonce, plaintext, aad)),
                       aad),
              unopened)
        << "a nonce that is not 12 bytes";
    EXPECT_EQ(openWith("box",
                       latin1(sealedEncrypt0(crypto, key, crypto.nonce, protectedHeader, withNonce, plaintext, aad)),
                       aad),
              (json{{"error", ""}, {"opened", true}, {"plaintext", latin1(plaintext)}}))
        << "an object with no kid opens under the key the request names";

    host::SodiumCoseCrypto drawnCrypto;
    CoseRig drawn(drawnCrypto);
    const host::Params ask = {{"key_id", "box"}, {"plaintext", latin1(plaintext)}, {"external_aad", ""}};
    EXPECT_NE(drawn.done(host::COSE_ENCRYPT0, ask).at("cose"), drawn.done(host::COSE_ENCRYPT0, ask).at("cose"))
        << "each sealing draws its own nonce";
}

/// A derivation takes exactly one input, answers the next `derived:<n>`, and keeps HKDF-SHA-256's 32 bytes
/// as a sealing key; it derives from a secret, a sealing key or a derived key, and from nothing else.
TEST(AStandardHostCose, AKeyIsDerivedFromExactlyOneInputAndSealsAfterwards) {
    host::SodiumCoseCrypto crypto;
    CoseRig rig(crypto);
    const auto derive = [&rig](const std::string &secret, const std::string &input) {
        return rig.start(host::KEY_DERIVE,
                         {{"secret", secret}, {"input_key_id", input}, {"salt", "salt"}, {"info", "info"}});
    };
    EXPECT_EQ(derive("s", "box").kind, host::Outcome::Kind::Refuse) << "both inputs";
    EXPECT_EQ(derive("", "").kind, host::Outcome::Kind::Refuse) << "neither input";
    EXPECT_EQ(derive("", "box").data, R"({"error":"","key_id":"derived:1"})");
    EXPECT_EQ(derive("\x01\x02", "").data, R"({"error":"","key_id":"derived:2"})");
    EXPECT_EQ(derive("", "derived:2").data, R"({"error":"","key_id":"derived:3"})");
    const std::string unknown = R"({"error":"unknown_key","key_id":""})";
    EXPECT_EQ(derive("", "derived:4").data, unknown);
    EXPECT_EQ(derive("", "derived:0").data, unknown);
    EXPECT_EQ(derive("", "derived:01").data, unknown);
    EXPECT_EQ(derive("", "device").data, unknown) << "a signing key is not keying material";
    EXPECT_EQ(rig.feature.derived(), (std::vector<std::string>{"derived:1", "derived:2", "derived:3"}));

    const Bytes plaintext = ascii("sealed");
    const json made = rig.done(host::COSE_ENCRYPT0,
                               {{"key_id", "derived:2"}, {"plaintext", latin1(plaintext)}, {"external_aad", ""}});
    const std::string sealedText = made.at("cose").get<std::string>();
    const auto object = cose::readEncrypt0(bytesOfLatin1(sealedText));
    ASSERT_TRUE(object.has_value());
    const auto okm = host::hkdfSha256(crypto, ascii("salt"), {0x01, 0x02}, ascii("info"), 32);
    ASSERT_TRUE(okm.has_value());
    EXPECT_EQ(answered(crypto.aeadOpen(*okm, object->unprotectedHeaders.iv.value_or(Bytes{}), object->ciphertext,
                                       cose::encStructure(object->protectedHeader, {}))),
              plaintext)
        << "derived:2 is HKDF-SHA-256 of its secret, salt and info";
    const auto openWith = [&rig, &sealedText](const std::string &keyId) {
        return rig.done(host::COSE_DECRYPT0, {{"key_id", keyId}, {"cose", sealedText}, {"external_aad", ""}});
    };
    EXPECT_EQ(openWith("derived:2").at("plaintext"), latin1(plaintext));
    EXPECT_EQ(openWith("derived:3").at("opened"), false);

    EXPECT_FALSE(rig.feature.setSealingKeys({{"derived:9", hex(kAeadKey)}})) << "the prefix of derived keys";
    EXPECT_FALSE(rig.feature.setSealingKeys({{"short", Bytes(16, uint8_t{0x01})}}));
    EXPECT_FALSE(rig.feature.setSigningKeys({{"", hex(kEd25519Seed)}}));
    EXPECT_EQ(rig.done(host::COSE_ENCRYPT0, {{"key_id", "box"}, {"plaintext", ""}, {"external_aad", ""}}).at("error"),
              "")
        << "a refused table leaves the keys set before";
}

/// The host keeps the contract of section 1 for the COSE feature as for its own operations: it serves its
/// types, records each start, refuses a request the feature does not start, fails one the embedding
/// injected a failure for with the record at its zero, and fails `unavailable` when the library could
/// not be started.
TEST(AStandardHostCose, TheHostKeepsTheContractOfEveryOperationForTheFeature) {
    host::SodiumCoseCrypto crypto;
    CoseRig rig(crypto);
    const std::vector<std::string> served = rig.reference.types();
    for (const char *type : host::COSE_TYPES) {
        EXPECT_NE(std::find(served.begin(), served.end(), type), served.end()) << type;
    }
    EXPECT_EQ(served.size(), host::TYPES.size() + host::COSE_TYPES.size());
    EXPECT_FALSE(rig.reference.addFeature(rig.feature)) << "a feature whose types are served already";

    EXPECT_EQ(rig.start(host::COSE_SIGN1, {{"key_id", "device"}, {"payload", "\xC4\x80"}, {"external_aad", ""}}).kind,
              host::Outcome::Kind::Refuse)
        << "a payload with a character past U+00FF, which no byte spells";
    EXPECT_EQ(rig.start(host::COSE_SIGN1, {{"key_id", "device"},
                                           {"payload", std::string(host::CoseBounds::PAYLOAD + 1, 'p')},
                                           {"external_aad", ""}})
                  .kind,
              host::Outcome::Kind::Refuse)
        << "a payload past its bound";
    EXPECT_EQ(
        rig.start(host::COSE_SIGN1,
                  {{"key_id", std::string(host::CoseBounds::KEY_ID + 1, 'k')}, {"payload", ""}, {"external_aad", ""}})
            .kind,
        host::Outcome::Kind::Refuse)
        << "a key id past its bound";
    EXPECT_EQ(rig.start(host::COSE_DECRYPT0, {{"key_id", ""}, {"cose", ""}, {"external_aad", ""}}).kind,
              host::Outcome::Kind::Refuse)
        << "an empty key id";
    EXPECT_EQ(rig.start(host::COSE_VERIFY1, {{"trust_set", ""}, {"cose", ""}, {"external_aad", ""}}).kind,
              host::Outcome::Kind::Refuse)
        << "an empty trust set";

    rig.reference.setFaults({{host::COSE_VERIFY1, "unavailable"}});
    EXPECT_EQ(rig.start(host::COSE_VERIFY1, {{"trust_set", "devices"}, {"cose", ""}, {"external_aad", ""}}).data,
              R"({"error":"unavailable","valid":false,"key_id":"","payload":""})");
    rig.reference.setFaults({});
    const std::vector<host::Start> starts = rig.reference.takeStarts();
    ASSERT_EQ(starts.size(), 6u) << "every start is recorded, a refused one included";
    EXPECT_EQ(starts.back().operation, host::COSE_VERIFY1);

    UnstartedCrypto unstarted;
    CoseRig down(unstarted);
    EXPECT_EQ(down.done(host::COSE_SIGN1, {{"key_id", "device"}, {"payload", "p"}, {"external_aad", ""}}),
              (json{{"error", "unavailable"}, {"cose", ""}}));
    EXPECT_EQ(down.done(host::COSE_DECRYPT0, {{"key_id", "box"}, {"cose", ""}, {"external_aad", ""}}),
              (json{{"error", "unavailable"}, {"opened", false}, {"plaintext", ""}}));
    EXPECT_EQ(down.done(host::KEY_DERIVE, {{"secret", "s"}, {"input_key_id", ""}, {"salt", ""}, {"info", ""}}),
              (json{{"error", "unavailable"}, {"key_id", ""}}));
}

/// The objects the Rust and the C++ reference hosts agree on, byte for byte (docs/SCE_HOST_FEATURES.md, section
/// 10): tests/host_features/cose_cross_language.json, which the Rust host is held to by
/// backends/rust/host-features/tests/cose_cross_language.rs. This host writes every object of the file through its
/// operations, a seal under the nonce the case fixes, and verifies and opens every object of it; the Rust host does
/// the same, so an object either writes is one the other reads.
TEST(AStandardHostCose, ACoseObjectCrossesBetweenTheHosts) {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" / "cose_cross_language.json");
    ASSERT_TRUE(in.good()) << "the vector file is not readable";
    const json vectors = json::parse(in);

    FixedNonceCrypto crypto;
    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature feature(crypto);
    ASSERT_TRUE(reference.addFeature(feature));

    // The signing keys, each held to the public key the file gives for its seed, and the trust set of them all,
    // in the file's order; the sealing keys.
    const std::string trustSet = "devices";
    std::map<std::string, Bytes> seeds;
    std::vector<std::string> signers;
    for (const json &signer : vectors.at("signing_keys")) {
        const std::string id = signer.at("id").get<std::string>();
        const Bytes seed = hex(signer.at("seed").get<std::string>());
        EXPECT_EQ(answered(crypto.ed25519PublicKey(seed)), hex(signer.at("public_key").get<std::string>()))
            << id << ": the file's public key is the one its seed gives";
        seeds[id] = seed;
        signers.push_back(id);
    }
    ASSERT_TRUE(feature.setSigningKeys(seeds));
    std::map<std::string, Bytes> sealing;
    for (const json &key : vectors.at("sealing_keys")) {
        sealing[key.at("id").get<std::string>()] = hex(key.at("key").get<std::string>());
    }
    ASSERT_TRUE(feature.setSealingKeys(sealing));
    feature.setTrustSets({{trustSet, signers}});

    const auto done = [&reference](const std::string &operation, const host::Params &params) {
        const auto outcome = reference.start(operation, host::Invocation{"session", "op", 1}, params);
        if (!outcome.has_value() || outcome->kind != host::Outcome::Kind::Done) {
            ADD_FAILURE() << operation << " did not complete";
            return json::object();
        }
        return json::parse(outcome->data);
    };

    const json invalid = {{"error", ""}, {"valid", false}, {"key_id", ""}, {"payload", ""}};
    for (const json &item : vectors.at("sign1")) {
        const std::string name = item.at("name").get<std::string>();
        const std::string keyId = item.at("key_id").get<std::string>();
        const Bytes payload = hex(item.at("payload").get<std::string>());
        const Bytes aad = hex(item.at("external_aad").get<std::string>());
        const std::string object = latin1(hex(item.at("cose").get<std::string>()));
        EXPECT_EQ(
            done(host::COSE_SIGN1, {{"key_id", keyId}, {"payload", latin1(payload)}, {"external_aad", latin1(aad)}}),
            (json{{"error", ""}, {"cose", object}}))
            << name << ": the object the Rust host verifies";
        EXPECT_EQ(done(host::COSE_VERIFY1, {{"trust_set", trustSet}, {"cose", object}, {"external_aad", latin1(aad)}}),
                  (json{{"error", ""}, {"valid", true}, {"key_id", keyId}, {"payload", latin1(payload)}}))
            << name << ": the object the Rust host signed";
        Bytes other = aad;
        other.push_back(uint8_t{'!'});
        EXPECT_EQ(
            done(host::COSE_VERIFY1, {{"trust_set", trustSet}, {"cose", object}, {"external_aad", latin1(other)}}),
            invalid)
            << name << ": the external data is part of what was signed";
    }

    const json unopened = {{"error", ""}, {"opened", false}, {"plaintext", ""}};
    for (const json &item : vectors.at("encrypt0")) {
        const std::string name = item.at("name").get<std::string>();
        const std::string keyId = item.at("key_id").get<std::string>();
        const Bytes plaintext = hex(item.at("plaintext").get<std::string>());
        const Bytes aad = hex(item.at("external_aad").get<std::string>());
        const Bytes object = hex(item.at("cose").get<std::string>());
        crypto.nonce = hex(item.at("nonce").get<std::string>());
        EXPECT_EQ(done(host::COSE_ENCRYPT0,
                       {{"key_id", keyId}, {"plaintext", latin1(plaintext)}, {"external_aad", latin1(aad)}}),
                  (json{{"error", ""}, {"cose", latin1(object)}}))
            << name << ": the object the Rust host opens";
        EXPECT_EQ(
            done(host::COSE_DECRYPT0, {{"key_id", keyId}, {"cose", latin1(object)}, {"external_aad", latin1(aad)}}),
            (json{{"error", ""}, {"opened", true}, {"plaintext", latin1(plaintext)}}))
            << name << ": the object the Rust host sealed";
        Bytes tampered = object;
        tampered.back() ^= 0x01;
        EXPECT_EQ(
            done(host::COSE_DECRYPT0, {{"key_id", keyId}, {"cose", latin1(tampered)}, {"external_aad", latin1(aad)}}),
            unopened)
            << name << ": the tag authenticates the ciphertext";
    }
}

}  // namespace SCE::Tests
