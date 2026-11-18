// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The https binding's adapter on WinHTTP (docs/adr/0107, D2), held to the
// conformance kit (https_kit/HttpsKit.h) on Windows.
//
// The platform's trust for a case is the Windows certificate store: the
// factory adds the case's authority to the local machine's Root store and
// removes it when the case ends, so WinHTTP validates the chain as it would
// for an authority an administrator deployed. Writing that store needs an
// administrator, which a hosted Windows runner is; a run without one fails
// with that reason rather than skipping, since a skipped case holds nothing.
// The explicit proxy is the adapter's `proxy` option: the user's proxy
// settings are the machine's, and a test does not rewrite them.

#include "https_kit/HttpsKit.h"

#include "mesh/transports/https/WinHttpHttpsTransport.h"

#include <gtest/gtest.h>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <wincrypt.h>

#include <chrono>
#include <fstream>
#include <iterator>
#include <memory>
#include <string>
#include <vector>

namespace {

namespace Kit = SCE::Mesh::HttpsKit;

/// An authority in the local machine's Root store for the life of a case.
class TrustedAuthority {
public:
    explicit TrustedAuthority(const std::string &derPath) {
        std::ifstream in(derPath, std::ios::binary);
        const std::vector<char> der((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
        if (der.empty()) {
            error_ = "the authority's DER file could not be read: " + derPath;
            return;
        }
        store_ = CertOpenStore(CERT_STORE_PROV_SYSTEM_W, 0, 0, CERT_SYSTEM_STORE_LOCAL_MACHINE, L"Root");
        if (store_ == nullptr) {
            error_ = "the local machine's Root store could not be opened (error " + std::to_string(GetLastError()) +
                     "); the WinHTTP kit needs an administrator";
            return;
        }
        if (!CertAddEncodedCertificateToStore(store_, X509_ASN_ENCODING, reinterpret_cast<const BYTE *>(der.data()),
                                              static_cast<DWORD>(der.size()), CERT_STORE_ADD_REPLACE_EXISTING,
                                              &added_)) {
            error_ = "the authority could not be added to the Root store (error " + std::to_string(GetLastError()) +
                     "); the WinHTTP kit needs an administrator";
        }
    }

    ~TrustedAuthority() {
        if (added_ != nullptr) {
            // CertDeleteCertificateFromStore frees the context it is given.
            CertDeleteCertificateFromStore(added_);
        }
        if (store_ != nullptr) {
            CertCloseStore(store_, 0);
        }
    }

    TrustedAuthority(const TrustedAuthority &) = delete;
    TrustedAuthority &operator=(const TrustedAuthority &) = delete;

    const std::string &error() const {
        return error_;
    }

private:
    HCERTSTORE store_ = nullptr;
    PCCERT_CONTEXT added_ = nullptr;
    std::string error_;
};

class WinHttpFactory final : public Kit::AdapterFactory {
public:
    Kit::AdapterUnderTest make(const Kit::Network &network) override {
        auto trust = std::make_shared<TrustedAuthority>(network.trusted_der);
        if (!trust->error().empty()) {
            problems_.push_back(trust->error());
        }
        SCE::Mesh::WinHttpHttpsTransport::Options options;
        options.proxy = network.proxy;
        return Kit::AdapterUnderTest{std::make_unique<SCE::Mesh::WinHttpHttpsTransport>(options), trust};
    }

    std::vector<std::string> problems_;
};

class WinHttpAdapter : public ::testing::Test {
protected:
    static void SetUpTestSuite() {
        kit_ = new Kit::Kit();
    }

    static void TearDownTestSuite() {
        delete kit_;
        kit_ = nullptr;
    }

    void SetUp() override {
        ASSERT_TRUE(kit_->ready()) << "the kit could not make its authorities and certificates";
    }

    void TearDown() override {
        for (const auto &problem : factory_.problems_) {
            ADD_FAILURE() << problem;
        }
    }

    static Kit::Kit *kit_;
    WinHttpFactory factory_;
};

Kit::Kit *WinHttpAdapter::kit_ = nullptr;

TEST_F(WinHttpAdapter, CarriesTheBindingDirectly) {
    const auto result = kit_->directExchange(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(WinHttpAdapter, TunnelsThroughAnExplicitProxy) {
    const auto result = kit_->throughConnectProxy(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(WinHttpAdapter, HonoursANetworkThatInspectsTlsWithAnAuthorityTheStoreTrusts) {
    const auto result = kit_->throughInspectingProxy(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(WinHttpAdapter, RefusesAServerTheStoreDoesNotTrustAsTls) {
    const auto result = kit_->untrustedServerIsTls(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(WinHttpAdapter, ReportsAResponseDroppedAfterTheServerActed) {
    const auto result = kit_->droppedResponseAfterServerActed(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(WinHttpAdapter, EndsAnExchangeHeldPastItsBoundAsTimeout) {
    const auto result = kit_->heldPastBound(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST(WinHttpAdapterOptions, AClientCertificateNotInTheStoreIsTls) {
    SCE::Mesh::WinHttpHttpsTransport::Options options;
    options.client_certificate_thumbprint = "00112233445566778899aabbccddeeff00112233";
    SCE::Mesh::WinHttpHttpsTransport transport(options);
    ASSERT_TRUE(transport.valid());
    SCE::Mesh::HttpsRequest request;
    request.url = "https://localhost:1/sce-mesh/1/phone";
    request.timeout = std::chrono::milliseconds(1000);
    const auto answer = transport.exchange(request);
    EXPECT_EQ(answer.kind, SCE::Mesh::HttpsExchange::Kind::Failure);
    EXPECT_EQ(answer.failure, SCE::Mesh::HttpsExchange::Failure::Tls);
}

}  // namespace
