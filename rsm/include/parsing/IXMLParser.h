#pragma once

#include "IXMLDocument.h"
#include <memory>
#include <string>

namespace RSM {

/**
 * @brief Abstract XML parser interface
 *
 * Platform-agnostic XML parser abstraction for multi-backend support.
 * Implementations: LibXMLParser (native), PugiXMLParser (WASM)
 *
 * ARCHITECTURE.md: Zero Duplication - Single interface for multiple backends
 * Native builds: Use libxml++ (mature, W3C compliant)
 * WASM builds: Use pugixml (lightweight, WASM-compatible)
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
     * @return Parser instance (LibXMLParser on native, PugiXMLParser on WASM)
     */
    static std::shared_ptr<IXMLParser> create();
};

}  // namespace RSM
