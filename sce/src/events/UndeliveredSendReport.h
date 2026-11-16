// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "events/EventDescriptor.h"

namespace SCE {

/**
 * @brief The error event a `<send>` that failed is reported with, by why it
 *        failed, or nullptr for a failure that is not the document's to hear
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
 *   error.communication.
 * - A target nothing the host wired serves (a Mesh peer with no router) is a
 *   fault of how the host was set up: error.execution, as a host-served send
 *   with no handler is (SCE_MESH.md, Errors).
 * - A send cancelled before it was delivered, a refused argument and an
 *   internal fault raise nothing here: the first is the document's own
 *   `<cancel>`, and the others are reported where they arise.
 */
inline const char *errorEventForFailedSend(SendResult::ErrorType type) {
    // §scxml-6.2.4: "If it is unable to dispatch the message, the Processor
    // MUST place the error error.communication on the internal event queue."
    switch (type) {
    case SendResult::ErrorType::TARGET_NOT_FOUND:
    case SendResult::ErrorType::NETWORK_ERROR:
    case SendResult::ErrorType::TIMEOUT:
        return "error.communication";
    case SendResult::ErrorType::NOT_CONFIGURED:
        return "error.execution";
    case SendResult::ErrorType::NONE:
    case SendResult::ErrorType::VALIDATION_ERROR:
    case SendResult::ErrorType::CANCELLED:
    case SendResult::ErrorType::INTERNAL_ERROR:
        return nullptr;
    }
    return nullptr;
}

}  // namespace SCE
