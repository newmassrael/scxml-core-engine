#pragma once

#include "factory/NodeFactory.h"
#include "model/IStateNode.h"
#include "parsing/IXMLElement.h"

#include <memory>
#include <string>

/**
 * @brief Class for parsing <donedata> elements
 *
 * This class parses SCXML <donedata> elements and their child elements (<content>, <param>).
 * <donedata> defines the data to be returned when a <final> state is entered.
 */

namespace SCE {

class DoneDataParser {
public:
    /**
     * @brief Constructor
     * @param factory Factory instance for node creation
     */
    explicit DoneDataParser(std::shared_ptr<NodeFactory> factory);

    /**
     * @brief Destructor
     */
    ~DoneDataParser() = default;

    /**
     * @brief Parse <donedata> element
     * @param doneDataElement <donedata> XML element
     * @param stateNode Target state node
     * @return Parsing success status
     */
    bool parseDoneData(const std::shared_ptr<IXMLElement> &doneDataElement, IStateNode *stateNode);

private:
    /**
     * @brief Parse <content> element (IXMLElement version)
     * @param contentElement <content> XML element
     * @param stateNode Target state node
     * @return Parsing success status
     */
    bool parseContent(const std::shared_ptr<IXMLElement> &contentElement, IStateNode *stateNode);

    /**
     * @brief Parse <param> element (IXMLElement version)
     * @param paramElement <param> XML element
     * @param stateNode Target state node
     * @return Parsing success status
     */
    bool parseParam(const std::shared_ptr<IXMLElement> &paramElement, IStateNode *stateNode);

    std::shared_ptr<NodeFactory> factory_;  // Node creation factory
};

}  // namespace SCE