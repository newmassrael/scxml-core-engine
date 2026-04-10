// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <functional>
#include <map>
#include <string>
#include <vector>

namespace SCE::Forge {

/// Service request dispatched by procedure <send sce:service> actions.
/// SCE_FORGE.md Section 4.5: sce:service and sce:subfunc are codegen hints
/// mapped to method calls on the runtime's service interface.
struct ProcedureServiceRequest {
    /// Service name (sce:service attribute value).
    std::string service;
    /// Sub-function code (sce:subfunc attribute value). Empty if not specified.
    std::string subfunc;
    /// Key-value parameters (sce:addr, sce:payload, etc.).
    std::vector<std::pair<std::string, std::string>> params;
};

/// Response from a procedure service handler.
/// Determines the event raised after the send action: "ok" or "fail".
struct ProcedureServiceResponse {
    /// Whether the service call succeeded (raises "ok" event if true, "fail" if false).
    bool success = false;
    /// Response data string. Available as _event.data in subsequent <assign> expressions.
    std::string data;
};

/// Callback type for procedure service dispatch.
/// Set on the generated procedure engine before calling runToCompletion().
///
/// Example usage:
/// @code
/// securityAccess sm;
/// sm.setServiceHandler([&client](const ProcedureServiceRequest& req) {
///     auto resp = client.send(req.service, req.subfunc);
///     return ProcedureServiceResponse{resp.ok, resp.payload};
/// });
/// auto result = sm.runToCompletion();
/// @endcode
using ProcedureServiceHandler =
    std::function<ProcedureServiceResponse(const ProcedureServiceRequest&)>;

/// Result of a Level 2 procedure execution via runToCompletion().
/// Contains the final state name and done data parameters.
struct ProcedureRunResult {
    /// Whether the procedure reached a final state.
    bool completed = false;
    /// Name of the final state reached (empty if not completed).
    std::string final_state;
    /// Done data parameters from <donedata> on the reached <final> state.
    std::map<std::string, std::string> done_data;
};

}  // namespace SCE::Forge
