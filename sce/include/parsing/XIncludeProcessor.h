#pragma once

#include "IXIncludeProcessor.h"
#include "parsing/IXMLDocument.h"
#include <string>
#include <unordered_map>
#include <vector>

/**
 * @brief Class responsible for XInclude processing
 *
 * @deprecated This class is legacy. XInclude processing is now handled
 * by IXMLDocument::processXInclude() method. This class is kept for
 * API compatibility.
 *
 * Stub implementation: Actual processing done by IXMLDocument::processXInclude()
 */

namespace SCE {

class XIncludeProcessor : public IXIncludeProcessor {
public:
    /**
     * @brief Constructor
     */
    XIncludeProcessor();

    /**
     * @brief Destructor
     */
    ~XIncludeProcessor() override;

    /**
     * @brief Execute XInclude processing
     * @param doc Platform-independent XML document
     * @return Success status
     *
     * @deprecated Use IXMLDocument::processXInclude() instead
     */
    bool process(std::shared_ptr<IXMLDocument> doc) override;

    /**
     * @brief Set base search path
     * @param basePath Base search path
     */
    void setBasePath(const std::string &basePath) override;

    /**
     * @brief Return error messages that occurred during processing
     * @return List of error messages
     */
    const std::vector<std::string> &getErrorMessages() const override;

private:
    // Stub implementation - actual processing in IXMLDocument::processXInclude()
    std::string basePath_;
    std::vector<std::string> errorMessages_;
};

}  // namespace SCE
