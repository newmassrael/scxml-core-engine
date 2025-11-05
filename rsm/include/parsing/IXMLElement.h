#pragma once

#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

namespace RSM {

/**
 * @brief Abstract XML element interface
 *
 * Platform-agnostic XML element abstraction for multi-backend support.
 * Implementation: PugiXMLElement (all platforms)
 */
class IXMLElement {
public:
    virtual ~IXMLElement() = default;

    /**
     * @brief Get element tag name
     * @return Tag name (e.g., "state", "transition")
     */
    virtual std::string getName() const = 0;

    /**
     * @brief Get attribute value
     * @param name Attribute name
     * @return Attribute value, empty string if not found
     */
    virtual std::string getAttribute(const std::string &name) const = 0;

    /**
     * @brief Check if attribute exists
     * @param name Attribute name
     * @return true if attribute exists
     */
    virtual bool hasAttribute(const std::string &name) const = 0;

    /**
     * @brief Get all attributes as key-value pairs
     * @return Map of attribute names to values
     */
    virtual std::unordered_map<std::string, std::string> getAttributes() const = 0;

    /**
     * @brief Get namespace URI
     * @return Namespace URI, empty string if no namespace
     */
    virtual std::string getNamespace() const = 0;

    /**
     * @brief Get all child elements
     * @return Vector of child elements
     */
    virtual std::vector<std::shared_ptr<IXMLElement>> getChildren() const = 0;

    /**
     * @brief Get child elements by tag name
     * @param tagName Tag name to filter
     * @return Vector of matching child elements
     */
    virtual std::vector<std::shared_ptr<IXMLElement>> getChildrenByTagName(const std::string &tagName) const = 0;

    /**
     * @brief Get text content
     * @return Text content of element
     */
    virtual std::string getTextContent() const = 0;

    /**
     * @brief Import node from another document
     * @param source Source element to import
     * @return true on success
     */
    virtual bool importNode(const std::shared_ptr<IXMLElement> &source) = 0;

    /**
     * @brief Remove this element from parent
     * @return true on success
     */
    virtual bool remove() = 0;

    /**
     * @brief Get parent element
     * @return Parent element, nullptr if root
     */
    virtual std::shared_ptr<IXMLElement> getParent() const = 0;

    /**
     * @brief Serialize child content to XML string
     * @return Serialized XML content (empty if no children)
     *
     * pugixml: Full XML structure preservation with pugi::xml_node::print()

     */
    virtual std::string serializeChildContent() const = 0;
};

}  // namespace RSM
