#pragma once

#include "parsing/IXMLElement.h"
#include <memory>
#include <string>

#ifndef __EMSCRIPTEN__
#include <libxml++/libxml++.h>
#endif

namespace RSM {

/**
 * @brief Helper for XML content serialization across platforms
 *
 * W3C SCXML B.2: Provides platform-agnostic XML content extraction
 * for assign elements, data elements, and send content elements.
 *
 * ARCHITECTURE.md Zero Duplication: Single Source of Truth for XML serialization
 */
class XmlSerializationHelper {
public:
    /**
     * @brief Serialize XML element content to string
     * @param element XML element (IXMLElement interface)
     * @return Serialized content string
     *
     * W3C SCXML: For WASM builds, uses text content extraction
     * Note: Cannot fully serialize XML structure on WASM (pugixml limitation)
     */
    static std::string serializeContent(const std::shared_ptr<IXMLElement> &element);

    /**
     * @brief Escape string for JavaScript literal
     * @param content Raw content string
     * @return JavaScript-escaped string with quotes
     *
     * W3C SCXML test 530: Serialize XML content as string literal for assign elements
     */
    static std::string escapeForJavaScript(const std::string &content);
};

}  // namespace RSM
