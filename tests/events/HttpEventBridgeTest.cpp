// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML C.2.1 — how an inbound BasicHTTP message is named.
//
// "If a single instance of the parameter '_scxmleventname' is present, the
//  SCXML Processor MUST use its value as the name of the SCXML event that it
//  raises. If multiple instances of the parameter are present, the behavior is
//  platform-specific. If the parameter '_scxmleventname' is not present, the
//  SCXML Processor MUST use the name of the HTTP method that was used to
//  deliver the message as the name of the event that it raises."
//
// This surface previously returned the literal "event1" for every request whose
// bridge had W3C compliance enabled — which is the default. No test caught it
// because the W3C harness (tests/w3c/W3CHttpTestServer.cpp) parses the form body
// itself and never reaches the bridge, so the defect lived only on the
// production path. These cases pin the rule at the bridge.

#include "events/HttpEventBridge.h"
#include <gtest/gtest.h>

namespace {

using SCE::HttpBridgeConfig;
using SCE::HttpEventBridge;
using SCE::HttpRequest;

HttpRequest formPost(const std::string &body) {
    HttpRequest request;
    request.method = "POST";
    request.url = "http://localhost:8080/scxml";
    request.headers["Content-Type"] = "application/x-www-form-urlencoded";
    request.body = body;
    return request;
}

std::string nameOf(HttpRequest request) {
    HttpEventBridge bridge{HttpBridgeConfig{}};
    return bridge.httpToScxmlEvent(request).eventName;
}

TEST(HttpEventBridgeEventName, SingleParameterNamesTheEvent) {
    EXPECT_EQ(nameOf(formPost("_scxmleventname=turnOn&level=3")), "turnOn");
}

TEST(HttpEventBridgeEventName, ParameterIsUrlDecoded) {
    // The sending half url-encodes the value, so the receiving half must decode
    // it or a dotted event name would arrive mangled.
    EXPECT_EQ(nameOf(formPost("_scxmleventname=done%2Estate%2Ea")), "done.state.a");
}

TEST(HttpEventBridgeEventName, ParameterIsFoundAfterOtherParameters) {
    EXPECT_EQ(nameOf(formPost("level=3&_scxmleventname=turnOn")), "turnOn");
}

TEST(HttpEventBridgeEventName, AbsentParameterFallsBackToTheHttpMethodName) {
    // The spec's fallback, not the configured default event name.
    EXPECT_EQ(nameOf(formPost("level=3")), "POST");

    HttpRequest put = formPost("level=3");
    put.method = "PUT";
    EXPECT_EQ(nameOf(put), "PUT");
}

TEST(HttpEventBridgeEventName, EmptyBodyFallsBackToTheHttpMethodName) {
    EXPECT_EQ(nameOf(formPost("")), "POST");
}

TEST(HttpEventBridgeEventName, MultipleInstancesResolveToTheFirst) {
    // Multiple instances are platform-specific per the spec; SCE takes the
    // first so the choice is deterministic rather than accidental.
    EXPECT_EQ(nameOf(formPost("_scxmleventname=first&_scxmleventname=second")), "first");
}

TEST(HttpEventBridgeEventName, QueryParameterIsHonouredWhenTheBodyHasNone) {
    HttpRequest request = formPost("level=3");
    request.queryParams["_scxmleventname"] = "fromQuery";
    EXPECT_EQ(nameOf(request), "fromQuery");
}

TEST(HttpEventBridgeEventName, UrlQueryStringIsHonouredWhenNothingElseCarriesIt) {
    HttpRequest request = formPost("level=3");
    request.url = "http://localhost:8080/scxml?_scxmleventname=fromUrl";
    EXPECT_EQ(nameOf(request), "fromUrl");
}

}  // namespace
