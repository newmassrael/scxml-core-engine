// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

    W3CHttpTestServer(int port, const std::string &path = "/test");
    ~W3CHttpTestServer();

    bool start();
    void stop();

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

    int getPort() const {
        return port_;
    }

    std::string getPath() const {
        return path_;
    }

private:
    void handlePost(const httplib::Request &req, httplib::Response &res);

    int port_;
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