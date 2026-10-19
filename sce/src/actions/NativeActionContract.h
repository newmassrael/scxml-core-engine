// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <vector>

namespace SCE {

class IStateNode;
class NativeAction;
class SCXMLModel;

/**
 * @brief What a document says about its `<sce:action>`s as a whole (docs/adr/0038)
 *
 * A single `<sce:action>` states its own `effect` and `released-by`; the rules
 * between actions need the whole document. They are the ones the build tool
 * applies (`sce-build/src/forge/native_action.rs`, `check_releases` and
 * `register_signature`), applied here to the parsed model so the Interpreter
 * refuses the documents the generated engines refuse, with the same codes,
 * messages and key fragments:
 *
 *  - one name carries one effect at every call site, because the effect is part
 *    of the one host operation that serves them;
 *  - a `released-by` sits on a duty of an `<onentry>`, names an action of the
 *    document that is a duty performed on the `<onexit>` of the same state.
 *
 * `StateMachine::start()` reads `statesDuty` for the other half of the ADR: a
 * machine that states a duty refuses a host that only simulates it.
 */
namespace NativeActionContract {

/// Where in a state an action sits. The order is the one the build tool walks.
enum class Site {
    Entry,
    Exit,
    Initial,
    HistoryDefault,
    Transition,
};

/// One `<sce:action>` of the document and the state that holds it.
struct Occurrence {
    const NativeAction *action;
    const IStateNode *state;
    Site site;
};

/// Every `<sce:action>` the model holds, in the order of its states sorted by
/// id and, within a state, `<onentry>`, `<onexit>`, `<initial>`, a history's
/// default, then its transitions. An action nested in an `<if>` or `<foreach>`
/// is found and counts as the block that holds it: the build tool refuses such
/// a placement, the Interpreter runs it, and a duty there is still a duty.
std::vector<Occurrence> occurrences(const SCXMLModel &model);

/// Whether the model states any duty, wherever it sits.
bool statesDuty(const SCXMLModel &model);

/// Apply the rules between actions. Throws the first refusal as a typed
/// `SemanticError` leaf (`validation/native-action-signature-conflict` or
/// `validation/native-action-release`); returns when the document holds.
void require(const SCXMLModel &model);

}  // namespace NativeActionContract

}  // namespace SCE
