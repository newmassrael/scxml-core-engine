#pragma once

#include "IXMLElement.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Abstract XML document interface
 *
 * Platform-agnostic XML document abstraction for multi-backend support.
 * Implementation: PugiXMLDocument (all platforms)
 */
class IXMLDocument {
public:
    virtual ~IXMLDocument() = default;

    /**
     * @brief Get root element
     * @return Root element, nullptr if document is empty
     */
    virtual std::shared_ptr<IXMLElement> getRootElement() = 0;

    /**
     * @brief Process XInclude directives
     * @return true on success
     *
     * W3C XInclude: Replaces <xi:include> elements with external content
     * Uses pugixml's manual XInclude implementation
     * WASM: Manual implementation using pugixml
     */
    virtual bool processXInclude() = 0;

    /**
     * @brief Get error message if parsing failed
     * @return Error message, empty if no error
     */
    virtual std::string getErrorMessage() const = 0;

    /**
     * @brief Check if document is valid
     * @return true if document loaded successfully
     */
    virtual bool isValid() const = 0;
};

}  // namespace SCE
