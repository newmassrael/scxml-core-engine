// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "BasicHttpTestEndpoint.h"
#include "core/LogMacros.h"
#include <atomic>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace httplib {
class Server;
class Request;
class Response;
}  // namespace httplib

namespace SCE {
namespace W3C {

class W3CHttpTestServer {
public:
    using EventCallback = std::function<void(const std::string &eventName, const std::string &data)>;

    /**
     * @param port The port to bind, or 0 for one the kernel assigns (read it
     *        back with getPort() or accessUri() once start() has succeeded)
     */
    W3CHttpTestServer(int port, const std::string &path = "/test");
    ~W3CHttpTestServer();

    bool start();
    void stop();

    /**
     * @brief start(), and a std::runtime_error naming the address when it fails
     *
     * For a W3C run, where a listener that cannot start makes the test it
     * serves an ERROR (W3CTestRunner::errorReport) rather than a FAIL: the
     * machine was never given the chance to pass.
     */
    void startOrThrow();

    void setEventCallback(const EventCallback &callback) {
        std::lock_guard<std::mutex> lock(callbackMutex_);
        eventCallback_ = callback;

        // Deliver any pending events that arrived before callback was set
        if (eventCallback_ && !pendingEvents_.empty()) {
            SCE_LOG_DEBUG("W3CHttpTestServer: Delivering {} pending events", pendingEvents_.size());
            for (const auto &[eventName, eventData] : pendingEvents_) {
                eventCallback_(eventName, eventData);
            }
            pendingEvents_.clear();
        }
    }

    bool isRunning() const {
        return running_.load();
    }

    /// The port the server is bound to: the one it was given, or the one the
    /// kernel assigned; 0 before start() has bound one.
    int getPort() const {
        return boundPort_;
    }

    std::string getPath() const {
        return path_;
    }

    /**
     * @brief The BasicHTTP location of this listener, from the port it bound
     *
     * §scxml-C-2-3: what a machine this server serves is handed as the
     * processor's published location. Read from the bound socket rather than
     * recomputed from the environment, so it names the port the kernel
     * assigned when the server was given 0.
     *
     * @throws std::logic_error before start() has bound a port
     */
    std::string accessUri() const;

private:
    void handlePost(const httplib::Request &req, httplib::Response &res);

    int requestedPort_;  // What the constructor was given: 0 lets the kernel assign one
    int boundPort_{0};   // What start() bound; 0 until then
    std::string path_;
    std::string instanceId_;  // Unique ID to track which server instance responds
    std::unique_ptr<httplib::Server> server_;
    std::thread serverThread_;
    std::atomic<bool> running_{false};
    std::atomic<bool> shutdownRequested_{false};
    EventCallback eventCallback_;
    std::mutex callbackMutex_;  // Thread-safe access to eventCallback_ and pendingEvents_
    std::vector<std::pair<std::string, std::string>> pendingEvents_;  // Events that arrived before callback was set
};

}  // namespace W3C
}  // namespace SCE