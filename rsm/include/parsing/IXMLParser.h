#pragma once

#include "IXMLDocument.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Abstract XML parser interface
 *
 * Platform-agnostic XML parser abstraction for multi-backend support.
 * Implementation: PugiXMLParser (unified for all platforms)
 *
 * ARCHITECTURE.md: Zero Duplication - Single interface for multiple backends
 * Implementation: PugiXMLParser (unified for all platforms)
 * Backend: pugixml (lightweight, W3C compliant, WASM-compatible)
 */
class IXMLParser {
public:
    virtual ~IXMLParser() = default;

    /**
     * @brief Parse XML from file
     * @param filename Absolute path to XML file
     * @return Parsed document, nullptr on error
     */
    virtual std::shared_ptr<IXMLDocument> parseFile(const std::string &filename) = 0;

    /**
     * @brief Parse XML from string content
     * @param content XML string content
     * @return Parsed document, nullptr on error
     */
    virtual std::shared_ptr<IXMLDocument> parseContent(const std::string &content) = 0;

    /**
     * @brief Get last error message
     * @return Error message, empty if no error
     */
    virtual std::string getLastError() const = 0;

    /**
     * @brief Factory method to create platform-specific parser
     * @return Parser instance (PugiXMLParser on all platforms)
     */
    static std::shared_ptr<IXMLParser> create();
};

}  // namespace SCE
