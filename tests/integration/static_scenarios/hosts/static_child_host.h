// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host `static_child_host` is replayed with. A child that declares `<sce:action>`s is
// built with the host its parent's host answers for it, each time the invocation starts,
// and what the child asked of that host is what the scenario's `child_hosts` holds. The
// parent declares no act of its own: its host answers a host for each child and writes it
// into the book the replay reads. The parent's host owns what it answers, as a machine's
// own host is held by reference, so the children's hosts are kept in a deque.

#pragma once

#include <deque>

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_child_host_sm.h"

namespace SCE::Tests::StaticScenario {

template <> struct ScenarioHosts<::SCE::Generated::static_child_host::static_child_host> {
    using Machine = ::SCE::Generated::static_child_host::static_child_host;
    using WorkerActions =
        ::SCE::Generated::static_child_host__sce_synth_invoke__worker::StaticChildHostSceSynthInvokeWorkerActions;

    struct WorkerHost : WorkerActions {
        explicit WorkerHost(ChildHostBook::Child &child) : child(child) {}

        void started() override {
            child.call("started");
        }

        void finished(uint32_t steps) override {
            child.call("finished", json::array({steps}));
        }

        ChildHostBook::Child &child;
    };

    struct Host : ::SCE::Generated::static_child_host::StaticChildHostActions {
        ChildHostBook book;
        std::deque<WorkerHost> workers;

        WorkerActions &actionsForWorker() override {
            return workers.emplace_back(book.open("worker"));
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
