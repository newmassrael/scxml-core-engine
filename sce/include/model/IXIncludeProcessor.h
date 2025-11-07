#pragma once

#include <memory>
#include <string>
#include <vector>

namespace SCE {

// Forward declaration - platform-independent
class IXMLDocument;

/**
 * @brief Interface for XInclude processing
 *
 * @deprecated This interface is legacy. XInclude processing is now handled
 * by IXMLDocument::processXInclude() method. This interface is kept for
 * API compatibility but is no longer used internally.
 *
 * Platform support:
 * - All builds: XIncludeProcessor stub (delegates to PugiXMLDocument)
 * - WASM builds: PugiXIncludeProcessor (pugixml) - stub implementation
 */
class IXIncludeProcessor {
public:
    /**
     * @brief Virtual destructor
     */
    virtual ~IXIncludeProcessor() = default;

    /**
     * @brief Execute XInclude processing
     * @param doc Platform-independent XML document
     * @return Success status
     *
     * @deprecated Use IXMLDocument::processXInclude() instead
     */
    virtual bool process(std::shared_ptr<IXMLDocument> doc) = 0;

    /**
     * @brief Set base search path
     * @param basePath Base search path
     */
    virtual void setBasePath(const std::string &basePath) = 0;

    /**
     * @brief Return error messages that occurred during processing
     * @return List of error messages
     */
    virtual const std::vector<std::string> &getErrorMessages() const = 0;
};

}  // namespace SCE
