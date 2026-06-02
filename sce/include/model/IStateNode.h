// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "DoneData.h"  // Added header
#include "actions/IActionNode.h"
#include "types.h"
#include <memory>
#include <string>
#include <vector>

namespace SCE {

class ITransitionNode;
class IInvokeNode;
class IDataModelItem;

class IStateNode {
public:
    virtual ~IStateNode() = default;

    virtual const std::string &getId() const = 0;
    virtual Type getType() const = 0;

    virtual void setParent(IStateNode *parent) = 0;
    virtual IStateNode *getParent() const = 0;

    virtual void addChild(std::shared_ptr<IStateNode> child) = 0;
    virtual const std::vector<std::shared_ptr<IStateNode>> &getChildren() const = 0;

    virtual void addTransition(std::shared_ptr<ITransitionNode> transition) = 0;
    virtual const std::vector<std::shared_ptr<ITransitionNode>> &getTransitions() const = 0;

    virtual void addDataItem(std::shared_ptr<IDataModelItem> dataItem) = 0;
    virtual const std::vector<std::shared_ptr<IDataModelItem>> &getDataItems() const = 0;

    virtual void setOnEntry(const std::string &callback) = 0;
    virtual const std::string &getOnEntry() const = 0;

    virtual void setOnExit(const std::string &callback) = 0;
    virtual const std::string &getOnExit() const = 0;

    virtual void setInitialState(const std::string &state) = 0;
    virtual const std::string &getInitialState() const = 0;

    virtual void addInvoke(std::shared_ptr<IInvokeNode> invoke) = 0;
    virtual const std::vector<std::shared_ptr<IInvokeNode>> &getInvoke() const = 0;

    virtual void setHistoryType(bool isDeep) = 0;

    /**
     * @brief Return history state type
     * @return History type (NONE, SHALLOW, DEEP)
     */
    virtual HistoryType getHistoryType() const = 0;

    /**
     * @brief Check if shallow history
     * @return Whether it is shallow history
     */
    virtual bool isShallowHistory() const = 0;

    /**
     * @brief Check if deep history
     * @return Whether it is deep history
     */
    virtual bool isDeepHistory() const = 0;

    // §scxml-3.8: Block-based onentry action methods for proper handler isolation
    virtual void addEntryActionBlock(std::vector<std::shared_ptr<SCE::IActionNode>> block) = 0;
    virtual const std::vector<std::vector<std::shared_ptr<SCE::IActionNode>>> &getEntryActionBlocks() const = 0;
    // §scxml-3.9: Block-based onexit action methods for proper handler isolation
    virtual void addExitActionBlock(std::vector<std::shared_ptr<SCE::IActionNode>> block) = 0;
    virtual const std::vector<std::vector<std::shared_ptr<SCE::IActionNode>>> &getExitActionBlocks() const = 0;

    virtual bool isFinalState() const = 0;

    /**
     * @brief Return DoneData object reference
     * @return DoneData object reference
     */
    virtual const DoneData &getDoneData() const = 0;

    /**
     * @brief Return DoneData object reference (mutable)
     * @return DoneData object reference
     */
    virtual DoneData &getDoneData() = 0;

    /**
     * @brief Set `<donedata><content expr="X"/>` (§scxml-5.5 evaluated path)
     *
     * X will be evaluated against the active datamodel at runtime. Pass
     * an empty string to clear any previously-set content (used by the
     * XOR-with-params resolution in `DoneDataParser`).
     */
    virtual void setDoneDataContentExpression(const std::string &expr) = 0;

    /**
     * @brief Set `<donedata><content>inline text</content>` (§scxml-5.5 literal path)
     *
     * Per §scxml-5.5 the children of `<content>` are used **as the content
     * value** — no evaluation, no script engine required.
     */
    virtual void setDoneDataContentLiteral(const std::string &literal) = 0;

    /**
     * @brief Add <param> element to <donedata>
     * @param name Parameter name
     * @param location Data model location path
     */
    virtual void addDoneDataParam(const std::string &name, const std::string &location) = 0;

    /**
     * @brief Remove all <param> elements from <donedata>
     */
    virtual void clearDoneDataParams() = 0;

    /**
     * @brief Return transition object of initial element
     * @return Pointer to initial transition object, nullptr if no initial element
     */
    virtual std::shared_ptr<ITransitionNode> getInitialTransition() const = 0;

    /**
     * @brief Set transition object of initial element
     * @param transition Initial transition object
     */
    virtual void setInitialTransition(std::shared_ptr<ITransitionNode> transition) = 0;
};

}  // namespace SCE
