// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "common/ErrorData.h"
#include "events/EventDescriptor.h"

#include <optional>

namespace SCE {

/**
 * @brief The code a `<send>` that failed is reported with, by why it failed,
 *        or none for a failure that is not the document's to hear
 *
 * The code names its event (`Common::errorEventName`, docs/adr/0106 #111):
 * a code of the communication range is error.communication, any other
 * error.execution, so the reason and the event cannot disagree.
 *
 * The one reading of a SendResult's error type for the two places the
 * Interpreter learns a send failed: the send itself, for one made at once
 * (ActionExecutorImpl), and the delivery at the end of the delay, for a
 * delayed one (EventDispatcherImpl's reporting target). Two readings had
 * drifted: both reported a missing target, and neither a target that was
 * found and could not be reached, so a BasicHTTP POST that never reached its
 * server was dropped with nothing raised.
 *
 * - A target that names no session, one that cannot be reached, and one that
 *   did not answer in time are sends the processor was unable to dispatch:
 *   `targetUnreachable`, raised as error.communication.
 * - A target nothing the host wired serves (a Mesh peer with no router) is a
 *   fault of how the host was set up: `processorUnregistered`, raised as
 *   error.execution, as a host-served send with no handler is (SCE_MESH.md,
 *   Errors).
 * - A send cancelled before it was delivered, a refused argument and an
 *   internal fault raise nothing here: the first is the document's own
 *   `<cancel>`, and the others are reported where they arise.
 */
inline std::optional<Common::ErrorReason> errorReasonForFailedSend(SendResult::ErrorType type) {
    // §scxml-6.2.4: "If it is unable to dispatch the message, the Processor
    // MUST place the error error.communication on the internal event queue."
    switch (type) {
    case SendResult::ErrorType::TARGET_NOT_FOUND:
    case SendResult::ErrorType::NETWORK_ERROR:
    case SendResult::ErrorType::TIMEOUT:
        return Common::ErrorReason::TargetUnreachable;
    case SendResult::ErrorType::NOT_CONFIGURED:
        return Common::ErrorReason::ProcessorUnregistered;
    case SendResult::ErrorType::NONE:
    case SendResult::ErrorType::VALIDATION_ERROR:
    case SendResult::ErrorType::CANCELLED:
    case SendResult::ErrorType::INTERNAL_ERROR:
        return std::nullopt;
    }
    return std::nullopt;
}

}  // namespace SCE
