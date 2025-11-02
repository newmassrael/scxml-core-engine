#pragma once

#include "IXMLElement.h"
#include <memory>
#include <string>

namespace RSM {

/**
 * @brief Abstract XML document interface
 *
 * Platform-agnostic XML document abstraction for multi-backend support.
 * Implementations: LibXMLDocument (native), PugiXMLDocument (WASM)
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
     * Native: Uses libxml2's native XInclude processor
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

}  // namespace RSM
