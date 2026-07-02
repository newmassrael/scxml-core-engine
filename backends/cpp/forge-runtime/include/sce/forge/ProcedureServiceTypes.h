// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Forge {

/// Service request dispatched by procedure `<send sce:service>` actions.
///
/// Each field maps 1:1 to a `<send>` attribute in the SCXML source:
///
/// ```xml
/// <send sce:service="Diag" sce:subfunc="0x02"
///       sce:addr="ecuAddr" sce:payload="frame.encode_to_vec()"/>
/// ```
///
/// `service` is always present; the other three are `std::optional` so
/// absent attributes are distinguishable from empty values. `payload` is
/// typed as raw bytes (`std::vector<uint8_t>`) because its semantic role
/// is a wire-format data blob originating from codec `encode_to_vec()` calls.
/// `subfunc` and `addr` remain textual since the user may reference
/// datamodel variables of any SCE type (uint8..uint64, std::string).
struct ProcedureServiceRequest {
    /// Dispatch identifier (sce:service, always present).
    std::string service;
    /// Sub-function identifier (sce:subfunc). `std::nullopt` if absent.
    std::optional<std::string> subfunc;
    /// Address identifier (sce:addr). `std::nullopt` if absent.
    std::optional<std::string> addr;
    /// Payload bytes (sce:payload). `std::nullopt` if absent.
    std::optional<std::vector<std::uint8_t>> payload;
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
using ProcedureServiceHandler = std::function<ProcedureServiceResponse(const ProcedureServiceRequest &)>;

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
