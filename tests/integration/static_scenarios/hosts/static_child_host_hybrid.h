// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host `static_child_host_hybrid` is replayed with: a hybrid `<invoke>` is given the
// host answered for the candidate it starts, and for no other. Each candidate's host
// writes what it was asked into the book the replay reads, under the candidate's stem.

#pragma once

#include <deque>

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_child_host_hybrid_sm.h"

namespace SCE::Tests::StaticScenario {

template <> struct ScenarioHosts<::SCE::Generated::static_child_host_hybrid::static_child_host_hybrid> {
    using Machine = ::SCE::Generated::static_child_host_hybrid::static_child_host_hybrid;
    using FirstActions = ::SCE::Generated::static_hosted_first::StaticHostedFirstActions;
    using SecondActions = ::SCE::Generated::static_hosted_second::StaticHostedSecondActions;

    struct FirstHost : FirstActions {
        explicit FirstHost(ChildHostBook::Child &child) : child(child) {}

        void firstRan() override {
            child.call("first_ran");
        }

        ChildHostBook::Child &child;
    };

    struct SecondHost : SecondActions {
        explicit SecondHost(ChildHostBook::Child &child) : child(child) {}

        void secondRan() override {
            child.call("second_ran");
        }

        ChildHostBook::Child &child;
    };

    struct Host : ::SCE::Generated::static_child_host_hybrid::StaticChildHostHybridActions {
        ChildHostBook book;
        std::deque<FirstHost> firsts;
        std::deque<SecondHost> seconds;

        FirstActions &actionsForWork0jStaticHostedFirst() override {
            return firsts.emplace_back(book.open("work", "static_hosted_first"));
        }

        SecondActions &actionsForWork0jStaticHostedSecond() override {
            return seconds.emplace_back(book.open("work", "static_hosted_second"));
        }
    };

    std::unique_ptr<Machine> make() {
        return std::make_unique<Machine>(host);
    }

    std::optional<json> hostCalls() {
        return std::nullopt;
    }

    std::optional<json> childHosts() {
        return host.book.take();
    }

    Host host;
};

}  // namespace SCE::Tests::StaticScenario
