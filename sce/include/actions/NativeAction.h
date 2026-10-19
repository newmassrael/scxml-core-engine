// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "BaseAction.h"

#include <string>
#include <vector>

namespace SCE {

/**
 * @brief SCE `<sce:action>` — a host operation named by the document (§scxml-G-7)
 *
 * The statechart keeps the operation symbolic; the host performs it
 * (`INativeActionHost`). Each `<sce:arg expr>` is an expression of the
 * document's data model, evaluated when the action runs, in document order.
 *
 * Each action states its `effect` (docs/adr/0038): a `duty` is an operation the
 * machine's correctness depends on the host performing, a `notice` is one the
 * host may observe, record or drop. A duty an `<onentry>` takes may name the
 * action that gives it back, `released-by`, performed on the `<onexit>` of the
 * same state.
 *
 * Example:
 * <sce:action name="append_fragment_payload" effect="notice">
 *   <sce:arg expr="_event.data.payload"/>
 *   <sce:arg expr="_event.data.offset"/>
 * </sce:action>
 */
class NativeAction : public BaseAction {
public:
    /// One `<sce:arg>`: its expression, and the `name` it was given, if any.
    struct Argument {
        std::string name;
        std::string expr;
    };

    /// What the machine owes the host for this action (docs/adr/0038).
    /// `Unstated` is an action built in code that never said: the parser never
    /// makes one, because an `<sce:action>` with no `effect` is refused.
    enum class Effect {
        Unstated,
        Duty,
        Notice,
    };

    explicit NativeAction(const std::string &operation = "", const std::string &id = "");

    virtual ~NativeAction() = default;

    /// The operation the host is asked to perform.
    const std::string &getOperation() const;

    /// The document's word for an effect: `duty` or `notice`, empty when unstated.
    static const char *effectWord(Effect effect);

    Effect getEffect() const;

    void setEffect(Effect effect);

    /// The action that gives this duty back, as the document names it; empty
    /// when the duty is held for the machine's life or the action is a notice.
    const std::string &getReleasedBy() const;

    void setReleasedBy(const std::string &releasedBy);

    void addArgument(const std::string &expr, const std::string &name = "");

    const std::vector<Argument> &getArguments() const;

    // IActionNode implementation
    bool execute(IExecutionContext &context) override;
    std::string getActionType() const override;
    std::shared_ptr<IActionNode> clone() const override;

protected:
    // BaseAction implementation
    std::vector<std::string> validateSpecific() const override;
    std::string getSpecificDescription() const override;

private:
    std::string operation_;
    Effect effect_ = Effect::Unstated;
    std::string releasedBy_;
    std::vector<Argument> arguments_;
};

}  // namespace SCE
