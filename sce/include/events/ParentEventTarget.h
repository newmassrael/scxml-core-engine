// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IEventTarget.h"
#include <memory>
#include <string>

namespace SCE {

class IEventRaiser;

/**
 * @brief Event target for routing events to parent sessions (#_parent)
 *
 * §scxml-6.2: This target handles the special "#_parent" target used
 * in invoke scenarios where child sessions need to send events to their
 * parent session.
 *
 * It delivers; it does not wait. A `delay` is the dispatcher's: it schedules
 * the event with this target and calls `send` when the delay has elapsed,
 * and it drops what is still pending when the child session ends (W3C SCXML
 * 6.2, test 187). This target once scheduled a delayed event again itself,
 * and since the dispatcher hands it the event with its delay unchanged, a
 * delayed send to the parent was rescheduled every time it fired and never
 * arrived.
 */
class ParentEventTarget : public IEventTarget {
public:
    /**
     * @brief Construct parent event target
     * @param childSessionId The child session ID that wants to send to parent
     * @param eventRaiser Event raiser for delivering events to parent session
     */
    ParentEventTarget(const std::string &childSessionId, std::shared_ptr<IEventRaiser> eventRaiser);

    virtual ~ParentEventTarget() = default;

    // IEventTarget implementation
    std::future<SendResult> send(const EventDescriptor &event) override;
    std::vector<std::string> validate() const override;
    std::string getTargetType() const override;
    bool canHandle(const std::string &targetUri) const override;
    std::string getDebugInfo() const override;

private:
    std::string childSessionId_;
    std::shared_ptr<IEventRaiser> eventRaiser_;

    /**
     * @brief Find parent session ID for the given child session
     * @param childSessionId Child session ID
     * @return Parent session ID or empty string if not found
     */
    std::string findParentSessionId(const std::string &childSessionId) const;
};

}  // namespace SCE