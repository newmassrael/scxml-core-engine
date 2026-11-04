// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The requests a host-served <send> and a host-run <invoke> hand the host, read
// from sce-build/tests/fixtures/host_processor/host_request_cases.json, the table
// the Interpreter (HostRequestTest.cpp) and the generated C++ engine
// (HostRequestAotTest.cpp) are both held to (docs/adr/0084, D4). One reader and
// one comparison, so the two engines are asked the same question field by field.

#pragma once

#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <string>
#include <vector>

#include "core/HostProcessor.h"

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT must name the checkout: the host request table is read from it"
#endif

namespace SCE::Tests::HostRequestCases {

/// The directory the table and the documents it names are in.
inline std::string fixtureDirectory() {
    return std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/host_processor/";
}

/// The cases of the table, every one of them.
inline nlohmann::json cases() {
    std::ifstream file(fixtureDirectory() + "host_request_cases.json");
    if (!file.is_open()) {
        ADD_FAILURE() << "cannot read " << fixtureDirectory() << "host_request_cases.json";
        return nlohmann::json::array();
    }
    return nlohmann::json::parse(file).at("cases");
}

/// The cases of the table whose document is @p document.
inline std::vector<nlohmann::json> casesOf(const std::string &document) {
    std::vector<nlohmann::json> out;
    for (const auto &row : cases()) {
        if (row.at("document").get<std::string>() == document) {
            out.push_back(row);
        }
    }
    return out;
}

/// A case's `params` as the request holds them.
inline std::map<std::string, std::vector<std::string>> paramsOf(const nlohmann::json &request) {
    std::map<std::string, std::vector<std::string>> params;
    for (const auto &item : request.at("params").items()) {
        params[item.key()] = item.value().get<std::vector<std::string>>();
    }
    return params;
}

/// Hold @p actual, a send's request, to the case's `request`, naming the case.
inline void expectSendRequest(const nlohmann::json &row, const SCE::HostSendRequest &actual) {
    const auto &expected = row.at("request");
    const std::string label = row.at("name").get<std::string>();
    EXPECT_EQ(actual.processorType, expected.at("processorType").get<std::string>()) << label;
    EXPECT_EQ(actual.eventName, expected.at("eventName").get<std::string>()) << label;
    EXPECT_EQ(actual.target, expected.at("target").get<std::string>()) << label;
    EXPECT_EQ(actual.content, expected.at("content").get<std::string>()) << label;
    EXPECT_EQ(actual.params, paramsOf(expected)) << label;
    EXPECT_EQ(actual.sendId, expected.at("sendId").get<std::string>()) << label;
    EXPECT_EQ(actual.eventData, expected.at("eventData").get<std::string>()) << label;
    EXPECT_EQ(actual.invokeId, expected.at("invokeId").get<std::string>()) << label;
}

/// Hold @p actual, an invoke's request as its invoker was handed it, to the
/// case's `request`, naming the case. The token and the session are the
/// engine's own and are not compared.
inline void expectInvokeRequest(const nlohmann::json &row, const SCE::HostInvokeRequest &actual) {
    const auto &expected = row.at("request");
    const std::string label = row.at("name").get<std::string>();
    EXPECT_EQ(actual.processorType, expected.at("processorType").get<std::string>()) << label;
    EXPECT_EQ(actual.invokeId, expected.at("invokeId").get<std::string>()) << label;
    EXPECT_EQ(actual.src, expected.at("src").get<std::string>()) << label;
    EXPECT_EQ(actual.params, paramsOf(expected)) << label;
    EXPECT_EQ(actual.eventData, expected.at("eventData").get<std::string>()) << label;
    EXPECT_EQ(actual.content, expected.at("content").get<std::string>()) << label;
    EXPECT_FALSE(actual.restarted) << label;
}

}  // namespace SCE::Tests::HostRequestCases
