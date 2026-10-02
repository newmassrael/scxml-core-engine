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
 * Example:
 * <sce:action name="append_fragment_payload">
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

    explicit NativeAction(const std::string &operation = "", const std::string &id = "");

    virtual ~NativeAction() = default;

    /// The operation the host is asked to perform.
    const std::string &getOperation() const;

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
    std::vector<Argument> arguments_;
};

}  // namespace SCE
