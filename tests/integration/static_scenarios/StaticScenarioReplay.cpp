// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The parts of the scenario replay that no machine's type reaches, compiled once for every
// machine's unit (StaticScenarioReplay.h says what the replay is).

#include "integration/static_scenarios/StaticScenarioReplay.h"

#include <filesystem>
#include <fstream>
#include <map>
#include <vector>

#include "common/ForwardedEvent.h"

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT names the repository root; the build defines it"
#endif

namespace SCE::Tests::StaticScenario {

json readScenario(const std::string &scenario) {
    const auto path = std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build/tests/fixtures/static_datamodel/scenarios" /
                      (scenario + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    return in.is_open() ? json::parse(in) : json::object();
}

bool holds(const json &saved, const json &expected) {
    if (saved.is_string() && expected.is_number_integer()) {
        return saved.get<std::string>() == expected.dump();
    }
    return saved == expected;
}

ChildHostBook::Child::Child(std::string invoke, int start, std::string candidate)
    : invoke_(std::move(invoke)), start_(start), candidate_(std::move(candidate)) {}

void ChildHostBook::Child::call(const std::string &action, json args) {
    calls_.push_back({{"action", action}, {"args", std::move(args)}});
}

ChildHostBook::Child &ChildHostBook::open(const std::string &invoke, const std::string &candidate) {
    int start = 1;
    for (const auto &child : children_) {
        if (child.invoke_ == invoke) {
            ++start;
        }
    }
    return children_.emplace_back(invoke, start, candidate);
}

json ChildHostBook::take() {
    json fresh = json::array();
    for (auto &child : children_) {
        json calls = json::array();
        for (; child.taken_ < child.calls_.size(); ++child.taken_) {
            calls.push_back(child.calls_[child.taken_]);
        }
        if (child.announced_ && calls.empty()) {
            continue;
        }
        child.announced_ = true;
        json entry = {{"invoke", child.invoke_}, {"start", child.start_}};
        if (!child.candidate_.empty()) {
            entry["candidate"] = child.candidate_;
        }
        entry["host_calls"] = std::move(calls);
        fresh.push_back(std::move(entry));
    }
    return fresh;
}

/// What a child's router holds for it until its parent applies it.
using Inbox = std::shared_ptr<std::vector<::SCE::Common::ForwardedEvent>>;

/// The list every send is written into, the starts of each path, and the inbox of the
/// session each path is running.
struct ChildRouters::Book {
    std::shared_ptr<json> sent;
    std::map<std::string, int> starts;
    std::map<std::string, Inbox> running;
};

/// The router of one start of one child.
class ChildRouters::Recording : public ::SCE::Static::IChildMeshRouter {
public:
    Recording(std::shared_ptr<Book> book, std::string path, int number, Inbox inbox)
        : book_(std::move(book)), path_(std::move(path)), number_(number), inbox_(std::move(inbox)) {}

    /// Written down as the machine's own sends are, with the child's path and start; the
    /// peer is not there, so nothing answers.
    bool send(const std::string &target, const std::string &eventName, const std::string &data, const std::string &,
              const std::string &) override {
        json entry = {{"target", target}, {"event", eventName}};
        if (!data.empty()) {
            entry["data"] = json::parse(data);
        }
        entry["invoke"] = path_;
        entry["start"] = number_;
        book_->sent->push_back(std::move(entry));
        return true;
    }

    std::vector<::SCE::Common::ForwardedEvent> takeCalls() override {
        return std::exchange(*inbox_, {});
    }

    /// The path is no longer running this session: a step that names it fails until the
    /// path starts a child again.
    void detach() override {
        const auto running = book_->running.find(path_);
        if (running != book_->running.end() && running->second == inbox_) {
            book_->running.erase(running);
        }
    }

    std::shared_ptr<::SCE::Static::IChildMeshRouter> childRouter(const std::string &invokeId,
                                                                 const std::string &) override {
        return ChildRouters::start(book_, path_ + "/" + invokeId);
    }

private:
    std::shared_ptr<Book> book_;
    std::string path_;
    int number_;
    Inbox inbox_;
};

ChildRouters::ChildRouters(std::shared_ptr<json> sent) : book_(std::make_shared<Book>()) {
    book_->sent = std::move(sent);
}

::SCE::Static::ChildMeshRouters ChildRouters::factory() const {
    return [book = book_](const std::string &invokeId, const std::string &) { return start(book, invokeId); };
}

bool ChildRouters::deliver(const std::string &path, const std::string &event, const std::string &data) const {
    const auto running = book_->running.find(path);
    if (running == book_->running.end()) {
        return false;
    }
    running->second->push_back(::SCE::Common::ForwardedEvent{event, data, "", "", "external", "", ""});
    return true;
}

/// A router for a new start of `path`, which is now the session the path is running.
std::shared_ptr<::SCE::Static::IChildMeshRouter> ChildRouters::start(const std::shared_ptr<Book> &book,
                                                                     const std::string &path) {
    auto inbox = std::make_shared<std::vector<::SCE::Common::ForwardedEvent>>();
    const int number = ++book->starts[path];
    book->running[path] = inbox;
    return std::make_shared<Recording>(book, path, number, std::move(inbox));
}

}  // namespace SCE::Tests::StaticScenario
