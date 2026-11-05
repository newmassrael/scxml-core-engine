// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "W3CHttpTestServer.h"
#include "common/TestUtils.h"
#include "core/LogMacros.h"
#include "events/HttpEventBridge.h"
#include "runtime/JsonUtils.h"
#include <chrono>
#include <httplib.h>
#include <sstream>
#include <stdexcept>
#include <thread>

namespace SCE {
namespace W3C {

/// The port a server was given, in words: 0 means the kernel assigns one.
static std::string describePort(int requestedPort) {
    return requestedPort == 0 ? std::string("a port the kernel assigns") : "port " + std::to_string(requestedPort);
}

W3CHttpTestServer::W3CHttpTestServer(int port, const std::string &path)
    : requestedPort_(port), path_(path), server_(std::make_unique<httplib::Server>()) {
    // Generate unique instance ID for tracking
    auto now = std::chrono::system_clock::now().time_since_epoch().count();
    instanceId_ = "srv_" + std::to_string(now);

    // Set up POST handler for W3C HTTP event processing
    server_->Post(path_, [this](const httplib::Request &req, httplib::Response &res) { handlePost(req, res); });

    SCE_LOG_INFO("W3CHttpTestServer: Created server instance {} for {}, {}, path {}", instanceId_,
                 SCE_W3C_HTTP_TEST_HOST, describePort(requestedPort_), path_);
}

W3CHttpTestServer::~W3CHttpTestServer() {
    stop();
}

bool W3CHttpTestServer::start() {
    if (running_.load()) {
        SCE_LOG_WARN("W3CHttpTestServer: Server already running");
        return false;
    }

    shutdownRequested_ = false;

    // Set SO_REUSEADDR and SO_REUSEPORT to allow immediate port reuse
    server_->set_socket_options([](socket_t sock) {
        int yes = 1;
        setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, reinterpret_cast<const char *>(&yes), sizeof(yes));
#ifdef SO_REUSEPORT
        setsockopt(sock, SOL_SOCKET, SO_REUSEPORT, reinterpret_cast<const char *>(&yes), sizeof(yes));
#endif
    });

    // Bind port on the calling thread — guarantees the port is ready before returning.
    // Port 0 asks the kernel for one, and the port it chose is what accessUri() publishes.
    if (requestedPort_ == 0) {
        const int assigned = server_->bind_to_any_port(SCE_W3C_HTTP_TEST_HOST);
        if (assigned <= 0) {
            SCE_LOG_ERROR("W3CHttpTestServer: [{}] Failed to bind to {}", instanceId_, describePort(requestedPort_));
            return false;
        }
        boundPort_ = assigned;
    } else {
        if (!server_->bind_to_port(SCE_W3C_HTTP_TEST_HOST, requestedPort_)) {
            SCE_LOG_ERROR("W3CHttpTestServer: [{}] Failed to bind to {}", instanceId_, describePort(requestedPort_));
            return false;
        }
        boundPort_ = requestedPort_;
    }

    // Start the accept loop in a background thread (port is already bound)
    serverThread_ = std::thread([this]() {
        SCE_LOG_INFO("W3CHttpTestServer: [{}] Accepting connections on {}:{}{}", instanceId_, SCE_W3C_HTTP_TEST_HOST,
                     boundPort_, path_);
        running_ = true;

        bool success = server_->listen_after_bind();

        if (!success && !shutdownRequested_.load()) {
            SCE_LOG_ERROR("W3CHttpTestServer: [{}] listen_after_bind failed", instanceId_);
        }
        running_ = false;
        SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Server thread ended", instanceId_);
    });

    // Wait until the server is actually accepting connections
    // by polling with a TCP connect probe
    auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(2);
    bool serverReady = false;
    while (std::chrono::steady_clock::now() < deadline) {
        if (running_.load()) {
            serverReady = true;
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }

    if (!serverReady) {
        SCE_LOG_ERROR("W3CHttpTestServer: [{}] Server failed to become ready within timeout", instanceId_);
        stop();
        boundPort_ = 0;
        return false;
    }

    SCE_LOG_INFO("W3CHttpTestServer: [{}] HTTP server ready on {}", instanceId_, accessUri());
    return true;
}

void W3CHttpTestServer::startOrThrow() {
    if (!start()) {
        throw std::runtime_error("the W3C BasicHTTP fixture listener could not start on " SCE_W3C_HTTP_TEST_HOST ", " +
                                 describePort(requestedPort_) + ", path " + path_ +
                                 " (another process may hold the port; " SCE_W3C_HTTP_PORT_ENV " moves it)");
    }
}

std::string W3CHttpTestServer::accessUri() const {
    if (boundPort_ == 0) {
        throw std::logic_error("W3CHttpTestServer::accessUri: the listener has not bound a port; call start() first");
    }
    return basicHttpAccessUri(boundPort_, path_);
}

void W3CHttpTestServer::stop() {
    if (!running_.load() && !serverThread_.joinable()) {
        return;
    }

    SCE_LOG_INFO("W3CHttpTestServer: [{}] Stopping HTTP server", instanceId_);

    shutdownRequested_ = true;

    if (server_) {
        server_->stop();
    }

    if (serverThread_.joinable()) {
        serverThread_.join();
    }

    // Give OS more time to release the port completely
    std::this_thread::sleep_for(SCE::Test::Utils::LONG_WAIT_MS);

    SCE_LOG_INFO("W3CHttpTestServer: [{}] HTTP server stopped", instanceId_);
}

void W3CHttpTestServer::handlePost(const httplib::Request &req, httplib::Response &res) {
    SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Received POST request to {}", instanceId_, req.path);
    SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Request body: {}", instanceId_, req.body);

    try {
        // Parse incoming HTTP request according to W3C SCXML BasicHTTPEventProcessor spec
        std::string eventName = "event1";  // W3C default event name
        std::string eventData;             // Will be populated based on content type

        // W3C SCXML C.2: Check Content-Type for form data
        std::string contentType = req.get_header_value("Content-Type");
        bool isFormData = (contentType.find("application/x-www-form-urlencoded") != std::string::npos);

        // W3C SCXML C.2: BasicHTTP Event I/O Processor - Form Data Processing
        // W3C SCXML Appendix C.2 specifies application/x-www-form-urlencoded format
        // for HTTP event transmission. Parameters are parsed into _event.data object.
        if (isFormData) {
            // W3C SCXML test 531: Check _scxmleventname parameter with highest priority
            if (req.has_param("_scxmleventname")) {
                eventName = req.get_param_value("_scxmleventname");
                SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Using _scxmleventname parameter: {}", instanceId_, eventName);
            }

            // W3C SCXML test 518, 519, 534: Map form parameters to _event.data fields
            // _scxmleventname is used for both event name AND included in _event.data
            json dataObj = json::object();
            for (const auto &param : req.params) {
                // W3C SCXML C.2: Parse numeric values (test 519: param1 should be number, not string)
                std::string value = param.second;
                bool isNumeric = !value.empty() && (std::isdigit(value[0]) || value[0] == '-' || value[0] == '+');
                if (isNumeric) {
                    try {
                        // Try parsing as integer first
                        size_t pos;
                        int intVal = std::stoi(value, &pos);
                        if (pos == value.length()) {
                            // Successfully parsed as integer
                            dataObj[param.first] = intVal;
                            continue;
                        }
                        // Try parsing as double
                        double doubleVal = std::stod(value, &pos);
                        if (pos == value.length()) {
                            // Successfully parsed as double
                            dataObj[param.first] = doubleVal;
                            continue;
                        }
                    } catch (...) {
                        // Fall through to string assignment
                    }
                }
                // Use as string
                dataObj[param.first] = value;
            }

            // Convert to JSON string for event data
            if (!dataObj.empty()) {
                eventData = JsonUtils::toCompactString(dataObj);
                SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Form parameters as JSON: {}", instanceId_, eventData);
            }
        } else {
            // W3C SCXML C.2: Check if body is JSON or plain content
            bool isJsonContent = !req.body.empty() && (req.body.front() == '{' || req.body.front() == '[');

            // Use body as event data
            eventData = req.body;

            // Simple string-based event extraction for W3C compliance
            if (!req.body.empty() && isJsonContent) {
                // Look for "event" field in JSON using string parsing to avoid jsoncpp string_view issues
                size_t eventPos = req.body.find("\"event\"");
                if (eventPos != std::string::npos) {
                    size_t colonPos = req.body.find(":", eventPos);
                    if (colonPos != std::string::npos) {
                        size_t quoteStart = req.body.find("\"", colonPos);
                        if (quoteStart != std::string::npos) {
                            size_t quoteEnd = req.body.find("\"", quoteStart + 1);
                            if (quoteEnd != std::string::npos) {
                                eventName = req.body.substr(quoteStart + 1, quoteEnd - quoteStart - 1);
                                SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Extracted event name from JSON: {}", instanceId_,
                                              eventName);
                            }
                        }
                    }
                }
            }

            // W3C SCXML C.2: For non-JSON content, generate HTTP.POST event
            if (!isJsonContent && !req.body.empty() && !isFormData) {
                eventName = "HTTP.POST";
                SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Non-JSON content detected, using HTTP.POST event", instanceId_);
            }

            // Default event name if not specified (W3C test compatibility)
            if (eventName.empty() || eventName == "event1") {
                if (isJsonContent) {
                    eventName = "event1";  // Common W3C test default for JSON
                } else if (!isFormData) {
                    eventName = "HTTP.POST";  // W3C C.2: content without event name
                }
            }
        }

        // Generate unique sendId for W3C compliance
        std::string sendId = "w3c_test_" + std::to_string(std::chrono::system_clock::now().time_since_epoch().count());

        SCE_LOG_INFO("W3CHttpTestServer: [{}] Processing event '{}' with sendId '{}'", instanceId_, eventName, sendId);

        // Forward event to SCXML system via callback (thread-safe)
        // If callback not set yet, queue event for later delivery
        {
            std::lock_guard<std::mutex> lock(callbackMutex_);
            if (eventCallback_) {
                eventCallback_(eventName, eventData);
            } else {
                SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Callback not set yet, queuing event '{}' for later delivery",
                              instanceId_, eventName);
                pendingEvents_.emplace_back(eventName, eventData);
            }
        }

        // Send W3C compliant HTTP response using nlohmann/json
        json response = json::object();
        response["status"] = "success";
        response["event"] = eventName;
        response["sendId"] = sendId;
        response["serverId"] = instanceId_;
        response["timestamp"] =
            std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch())
                .count();

        std::string responseBody = JsonUtils::toCompactString(response);

        res.set_content(responseBody, "application/json");
        res.status = 200;
        res.set_header("Cache-Control", "no-cache");
        res.set_header("Access-Control-Allow-Origin", "*");  // W3C test compatibility
        res.set_header("Access-Control-Allow-Methods", "POST, OPTIONS");
        res.set_header("Access-Control-Allow-Headers", "Content-Type");

        SCE_LOG_DEBUG("W3CHttpTestServer: [{}] Sent response: {}", instanceId_, responseBody);

    } catch (const std::exception &e) {
        SCE_LOG_ERROR("W3CHttpTestServer: [{}] Exception handling POST request: {}", instanceId_, e.what());
        res.status = 500;
        res.set_content(R"({"status": "error", "message": "Internal server error"})", "application/json");
    }
}

}  // namespace W3C
}  // namespace SCE