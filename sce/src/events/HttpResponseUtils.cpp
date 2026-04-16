// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/HttpResponseUtils.h"
#include "runtime/JsonUtils.h"
#include "core/LogMacros.h"
#include "events/IEventBridge.h"  // For HttpResponse struct

namespace SCE {

const std::string HttpResponseUtils::JSON_CONTENT_TYPE = "application/json";
const std::string HttpResponseUtils::NO_CACHE_CONTROL = "no-cache";

void HttpResponseUtils::setJsonHeaders(HttpResponse &response) {
    response.headers["Content-Type"] = JSON_CONTENT_TYPE;
}

void HttpResponseUtils::setJsonHeaders(httplib::Response &response) {
    response.set_header("Content-Type", JSON_CONTENT_TYPE);
}

void HttpResponseUtils::setCorsHeaders(httplib::Response &response, const std::string &origin,
                                       const std::string &allowedMethods, const std::string &allowedHeaders) {
    response.set_header("Access-Control-Allow-Origin", origin.empty() ? "*" : origin);
    response.set_header("Access-Control-Allow-Methods", allowedMethods);
    response.set_header("Access-Control-Allow-Headers", allowedHeaders);
    response.set_header("Access-Control-Max-Age", "86400");  // 24 hours

    SCE_LOG_DEBUG("HttpResponseUtils: Set CORS headers for origin: {}", origin.empty() ? "*" : origin);
}

HttpResponse HttpResponseUtils::createSuccessResponse(const std::string &data) {
    HttpResponse response;
    response.statusCode = 200;
    response.body = data;
    setJsonHeaders(response);
    return response;
}

HttpResponse HttpResponseUtils::createErrorResponse(const std::string &errorMessage, int statusCode) {
    json errorObj;
    errorObj["error"] = errorMessage;
    errorObj["status"] = "error";

    HttpResponse response;
    response.statusCode = statusCode;
    response.body = JsonUtils::toCompactString(errorObj);
    setJsonHeaders(response);

    SCE_LOG_DEBUG("HttpResponseUtils: Created error response - status: {}, message: {}", statusCode, errorMessage);
    return response;
}

void HttpResponseUtils::setSuccessResponse(httplib::Response &response, const std::string &data) {
    response.status = 200;
    response.set_content(data, JSON_CONTENT_TYPE);
    SCE_LOG_DEBUG("HttpResponseUtils: Set success response with {} bytes", data.length());
}

void HttpResponseUtils::setErrorResponse(httplib::Response &response, const std::string &errorMessage, int statusCode) {
    json errorObj;
    errorObj["error"] = errorMessage;
    errorObj["status"] = "error";

    std::string errorBody = JsonUtils::toCompactString(errorObj);
    response.status = statusCode;
    response.set_content(errorBody, JSON_CONTENT_TYPE);

    SCE_LOG_DEBUG("HttpResponseUtils: Set error response - status: {}, message: {}", statusCode, errorMessage);
}

void HttpResponseUtils::setNoCacheHeaders(HttpResponse &response) {
    response.headers["Cache-Control"] = NO_CACHE_CONTROL;
}

void HttpResponseUtils::setNoCacheHeaders(httplib::Response &response) {
    response.set_header("Cache-Control", NO_CACHE_CONTROL);
}

}  // namespace SCE