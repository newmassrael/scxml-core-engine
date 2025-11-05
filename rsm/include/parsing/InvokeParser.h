// InvokeParser.h
#pragma once

#include "factory/NodeFactory.h"
#include "model/IInvokeNode.h"
#include "parsing/IXMLElement.h"

#include <memory>
#include <vector>

namespace RSM {

class InvokeParser {
public:
    InvokeParser(std::shared_ptr<NodeFactory> nodeFactory);
    ~InvokeParser();

    // Parse invoke element
    std::shared_ptr<IInvokeNode> parseInvokeNode(const std::shared_ptr<IXMLElement> &invokeElement);

    // Parse all invoke elements within a specific state
    std::vector<std::shared_ptr<IInvokeNode>> parseInvokesInState(const std::shared_ptr<IXMLElement> &stateElement);

    // Parse param elements and return created DataModelItems
    std::vector<std::shared_ptr<IDataModelItem>>
    parseParamElementsAndCreateDataItems(const std::shared_ptr<IXMLElement> &invokeElement,
                                         std::shared_ptr<IInvokeNode> invokeNode);

private:
    std::shared_ptr<NodeFactory> nodeFactory_;

    void parseFinalizeElement(const std::shared_ptr<IXMLElement> &finalizeElement,
                              std::shared_ptr<IInvokeNode> invokeNode);
    void parseParamElements(const std::shared_ptr<IXMLElement> &invokeElement, std::shared_ptr<IInvokeNode> invokeNode);
    void parseContentElement(const std::shared_ptr<IXMLElement> &invokeElement,
                             std::shared_ptr<IInvokeNode> invokeNode);
};

}  // namespace RSM
