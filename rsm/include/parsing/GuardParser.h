#pragma once

#include "factory/NodeFactory.h"
#include "model/IGuardNode.h"
#include "parsing/IXMLElement.h"

#include <memory>
#include <string>
#include <vector>

/**
 * @brief Class responsible for parsing guard conditions
 *
 * This class provides functionality to parse guard condition-related elements
 * in SCXML documents. It handles code:guard elements within code:guards elements
 * and guard attributes of transition elements.
 */

namespace RSM {

class GuardParser {
public:
    /**
     * @brief Constructor
     * @param nodeFactory Factory instance for node creation
     */
    explicit GuardParser(std::shared_ptr<NodeFactory> nodeFactory);

    /**
     * @brief Destructor
     */
    ~GuardParser();

    /**
     * @brief Parse guard node
     * @param guardNode XML guard node
     * @return Created guard node
     */
    std::shared_ptr<IGuardNode> parseGuardNode(const std::shared_ptr<IXMLElement> &guardNode);

    /**
     * @brief Parse guard attribute from transition
     * @param transitionNode XML transition node
     * @param targetState Transition target state
     * @return Created guard node, nullptr if no guard attribute
     */
    std::shared_ptr<IGuardNode> parseGuardFromTransition(const std::shared_ptr<IXMLElement> &transitionNode,
                                                         const std::string &targetState);

    /**
     * @brief Parse reactive guard
     * @param reactiveGuardNode XML reactive guard node
     * @return Created guard node
     */
    std::shared_ptr<IGuardNode> parseReactiveGuard(const std::shared_ptr<IXMLElement> &reactiveGuardNode);

    /**
     * @brief Parse all guards within guards element
     * @param guardsNode code:guards element
     * @return List of parsed guard nodes
     */
    std::vector<std::shared_ptr<IGuardNode>> parseGuardsElement(const std::shared_ptr<IXMLElement> &guardsNode);

    /**
     * @brief Parse all guards in SCXML document
     * @param scxmlNode SCXML root node
     * @return List of parsed guard nodes
     */
    std::vector<std::shared_ptr<IGuardNode>> parseAllGuards(const std::shared_ptr<IXMLElement> &scxmlNode);

    /**
     * @brief Check if element is a guard node
     * @param element XML element
     * @return Whether it is a guard node
     */
    bool isGuardNode(const std::shared_ptr<IXMLElement> &element) const;

    /**
     * @brief Check if element is a reactive guard node
     * @param element XML element
     * @return Whether it is a reactive guard node
     */
    bool isReactiveGuardNode(const std::shared_ptr<IXMLElement> &element) const;

private:
    /**
     * @brief Parse dependency list (IXMLElement version)
     * @param guardNode Guard node
     * @param guardObject Guard object
     */
    void parseDependencies(const std::shared_ptr<IXMLElement> &guardNode, std::shared_ptr<IGuardNode> guardObject);

    /**
     * @brief Parse external implementation element (IXMLElement version)
     * @param guardNode Guard node
     * @param guardObject Guard object
     */
    void parseExternalImplementation(const std::shared_ptr<IXMLElement> &guardNode,
                                     std::shared_ptr<IGuardNode> guardObject);

    /**
     * @brief Handle namespace matching
     * @param nodeName Node name
     * @param searchName Name to search for
     * @return Whether node name matches search name (considering namespace)
     */
    bool matchNodeName(const std::string &nodeName, const std::string &searchName) const;

    std::shared_ptr<NodeFactory> nodeFactory_;
};

}  // namespace RSM