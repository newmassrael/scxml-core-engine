#pragma once

#include "SCXMLContext.h"
#include "actions/IActionNode.h"
#include "factory/NodeFactory.h"
#include "model/IStateNode.h"
#include "parsing/IXMLElement.h"

#ifndef __EMSCRIPTEN__
#include <libxml++/libxml++.h>
#endif

#include <memory>
#include <string>

namespace RSM {

class TransitionParser;
class ActionParser;
class DataModelParser;
class InvokeParser;
class DoneDataParser;

class StateNodeParser {
public:
    explicit StateNodeParser(std::shared_ptr<NodeFactory> nodeFactory);
    ~StateNodeParser();

    // Parse state node
    std::shared_ptr<IStateNode> parseStateNode(const std::shared_ptr<IXMLElement> &stateElement,
                                               std::shared_ptr<IStateNode> parentState = nullptr,
                                               const SCXMLContext &context = SCXMLContext());

    // Set related parsers
    void setRelatedParsers(std::shared_ptr<TransitionParser> transitionParser,
                           std::shared_ptr<ActionParser> actionParser, std::shared_ptr<DataModelParser> dataModelParser,
                           std::shared_ptr<InvokeParser> invokeParser, std::shared_ptr<DoneDataParser> doneDataParser);

private:
#ifndef __EMSCRIPTEN__
    // Determine state type (libxml++ version)
    Type determineStateType(const xmlpp::Element *stateElement);

    // Parse child states (libxml++ version)
    void parseChildStates(const xmlpp::Element *stateElement, std::shared_ptr<IStateNode> parentState,
                          const SCXMLContext &context = SCXMLContext());

    // Parse transition elements (libxml++ version)
    void parseTransitions(const xmlpp::Element *parentElement, std::shared_ptr<IStateNode> state);

    // W3C SCXML 3.8/3.9: Parse onentry/onexit elements as IActionNode block-based (libxml++ version)
    void parseEntryExitActionNodes(const xmlpp::Element *parentElement, std::shared_ptr<IStateNode> state);

    // W3C SCXML 3.8/3.9: Block-based executable content parsing (libxml++ version)
    void parseExecutableContentBlock(const xmlpp::Element *parentElement,
                                     std::vector<std::shared_ptr<RSM::IActionNode>> &actionBlock);

    // Parse invoke elements (libxml++ version)
    void parseInvokeElements(const xmlpp::Element *parentElement, std::shared_ptr<IStateNode> state);

    // Parse history state type (shallow/deep) (libxml++ version)
    void parseHistoryType(const xmlpp::Element *historyElement, std::shared_ptr<IStateNode> state);

    // Parse reactive guards (libxml++ version)
    void parseReactiveGuards(const xmlpp::Element *parentElement, std::shared_ptr<IStateNode> state);

    // Parse initial element (libxml++ version)
    void parseInitialElement(const xmlpp::Element *initialElement, std::shared_ptr<IStateNode> state);
#endif

    // Determine state type (IXMLElement version)
    Type determineStateType(const std::shared_ptr<IXMLElement> &stateElement);

    // Parse child states (IXMLElement version)
    void parseChildStates(const std::shared_ptr<IXMLElement> &stateElement, std::shared_ptr<IStateNode> parentState,
                          const SCXMLContext &context = SCXMLContext());

    // Parse transition elements (IXMLElement version)
    void parseTransitions(const std::shared_ptr<IXMLElement> &parentElement, std::shared_ptr<IStateNode> state);

    // W3C SCXML 3.8/3.9: Parse onentry/onexit elements as IActionNode block-based (IXMLElement version)
    void parseEntryExitActionNodes(const std::shared_ptr<IXMLElement> &parentElement,
                                   std::shared_ptr<IStateNode> state);

    // W3C SCXML 3.8/3.9: Block-based executable content parsing (IXMLElement version)
    void parseExecutableContentBlock(const std::shared_ptr<IXMLElement> &parentElement,
                                     std::vector<std::shared_ptr<RSM::IActionNode>> &actionBlock);

    // Parse invoke elements (IXMLElement version)
    void parseInvokeElements(const std::shared_ptr<IXMLElement> &parentElement, std::shared_ptr<IStateNode> state);

    // Parse history state type (shallow/deep) (IXMLElement version)
    void parseHistoryType(const std::shared_ptr<IXMLElement> &historyElement, std::shared_ptr<IStateNode> state);

    // Parse reactive guards (IXMLElement version)
    void parseReactiveGuards(const std::shared_ptr<IXMLElement> &parentElement, std::shared_ptr<IStateNode> state);

    // Parse initial element (IXMLElement version)
    void parseInitialElement(const std::shared_ptr<IXMLElement> &initialElement, std::shared_ptr<IStateNode> state);

    std::shared_ptr<NodeFactory> nodeFactory_;
    std::shared_ptr<TransitionParser> transitionParser_;
    std::shared_ptr<ActionParser> actionParser_;
    std::shared_ptr<DataModelParser> dataModelParser_;
    std::shared_ptr<InvokeParser> invokeParser_;
    std::shared_ptr<DoneDataParser> doneDataParser_;
};

}  // namespace RSM