// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The https binding's adapter on the system's libcurl (docs/adr/0107, D2),
// held to the conformance kit (https_kit/HttpsKit.h). The platform's trust
// for a case is the CA bundle libcurl reads, which is how a distribution
// states it; the platform's proxy is HTTPS_PROXY, which libcurl reads itself,
// so the factory sets the variable and never the handle.

#include "https_kit/HttpsKit.h"

#include "mesh/transports/https/CurlHttpsTransport.h"

#include <gtest/gtest.h>

#include <cstdlib>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace {

namespace Kit = SCE::Mesh::HttpsKit;

/// The proxy variables as a case sets them, put back as they were after it.
class ProxyEnvironment {
public:
    ProxyEnvironment() {
        for (const char *name : {"https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY"}) {
            saved_.emplace_back(name, read(name));
            put(name, std::nullopt);
        }
    }

    static std::optional<std::string> read(const char *name) {
#if defined(_WIN32)
        char *value = nullptr;
        std::size_t length = 0;
        if (_dupenv_s(&value, &length, name) != 0 || value == nullptr) {
            return std::nullopt;
        }
        std::string copy(value);
        std::free(value);
        return copy;
#else
        const char *value = std::getenv(name);
        return value != nullptr ? std::optional<std::string>(value) : std::nullopt;
#endif
    }

    ~ProxyEnvironment() {
        for (const auto &entry : saved_) {
            put(entry.first.c_str(), entry.second);
        }
    }

    ProxyEnvironment(const ProxyEnvironment &) = delete;
    ProxyEnvironment &operator=(const ProxyEnvironment &) = delete;

    static void put(const char *name, const std::optional<std::string> &value) {
#if defined(_WIN32)
        _putenv_s(name, value ? value->c_str() : "");
#else
        if (value) {
            setenv(name, value->c_str(), 1);
        } else {
            unsetenv(name);
        }
#endif
    }

private:
    std::vector<std::pair<std::string, std::optional<std::string>>> saved_;
};

class CurlFactory final : public Kit::AdapterFactory {
public:
    Kit::AdapterUnderTest make(const Kit::Network &network) override {
        auto environment = std::make_shared<ProxyEnvironment>();
        if (!network.proxy.empty()) {
            // Upper case only: libcurl reads it as well as the lower-case
            // name, and the case holds that it does.
            ProxyEnvironment::put("HTTPS_PROXY", "http://" + network.proxy);
        }
        SCE::Mesh::CurlHttpsTransport::Options options;
        options.ca_bundle = network.trusted_pem;
        return Kit::AdapterUnderTest{std::make_unique<SCE::Mesh::CurlHttpsTransport>(options), environment};
    }
};

class CurlAdapter : public ::testing::Test {
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

    static Kit::Kit *kit_;
    CurlFactory factory_;
};

Kit::Kit *CurlAdapter::kit_ = nullptr;

TEST_F(CurlAdapter, CarriesTheBindingDirectly) {
    const auto result = kit_->directExchange(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(CurlAdapter, TunnelsThroughTheProxyHttpsProxyNames) {
    const auto result = kit_->throughConnectProxy(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(CurlAdapter, HonoursANetworkThatInspectsTlsWithAnAuthorityTheBundleTrusts) {
    const auto result = kit_->throughInspectingProxy(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(CurlAdapter, RefusesAServerTheBundleDoesNotTrustAsTls) {
    const auto result = kit_->untrustedServerIsTls(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(CurlAdapter, ReportsAResponseDroppedAfterTheServerActed) {
    const auto result = kit_->droppedResponseAfterServerActed(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

TEST_F(CurlAdapter, EndsAnExchangeHeldPastItsBoundAsTimeout) {
    const auto result = kit_->heldPastBound(factory_);
    EXPECT_TRUE(result.passed) << result.why;
}

}  // namespace
