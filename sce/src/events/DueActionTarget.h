// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "events/IEventTarget.h"

#include <functional>
#include <future>
#include <string>
#include <utility>
#include <vector>

namespace SCE {

/**
 * @brief A scheduler entry whose delivery is one act of the engine that armed it,
 *        rather than an event handed to a target the document named
 *
 * The Interpreter's scheduler delivers what it holds to an `IEventTarget` when
 * the entry comes due. Two things the host processor door arms are not events
 * for a target (docs/adr/0084, D4 and D5): a host-served `<send delay>`, which
 * is performed by the host's processor when its delay elapses, and the deadline
 * of a host-run invocation, which the engine's ledger judges when it comes due.
 * Each waits in the same queue as every delayed send, so `<cancel>` and the
 * scheduler's mode reach it as they reach any other, and does @p act when due.
 */
class DueActionTarget : public IEventTarget {
public:
    /**
     * @param kind What the entry is, for the scheduler's debugging views
     * @param act What to do when the entry comes due
     */
    DueActionTarget(std::string kind, std::function<void()> act) : kind_(std::move(kind)), act_(std::move(act)) {}

    std::future<SendResult> send(const EventDescriptor &event) override {
        if (act_) {
            act_();
        }
        std::promise<SendResult> done;
        done.set_value(SendResult::success(event.sendId));
        return done.get_future();
    }

    std::string getTargetType() const override {
        return kind_;
    }

    /// Never chosen for a target URI: it is armed directly, not created by a factory.
    bool canHandle(const std::string & /*targetUri*/) const override {
        return false;
    }

    std::vector<std::string> validate() const override {
        return {};
    }

    std::string getDebugInfo() const override {
        return "DueActionTarget(" + kind_ + ")";
    }

private:
    std::string kind_;
    std::function<void()> act_;
};

}  // namespace SCE
