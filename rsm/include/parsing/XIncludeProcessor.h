#pragma once

#include "IXIncludeProcessor.h"
#include "parsing/IXMLDocument.h"
#include <string>
#include <unordered_map>
#include <vector>

#ifndef __EMSCRIPTEN__
#include <libxml++/libxml++.h>
#endif

/**
 * @brief Class responsible for XInclude processing
 *
 * @deprecated This class is legacy. XInclude processing is now handled
 * by IXMLDocument::processXInclude() method. This class is kept for
 * API compatibility.
 *
 * Platform support:
 * - Native builds: Uses libxml++ for XInclude processing
 * - WASM builds: Stub implementation (actual processing done by PugiXMLDocument)
 */

namespace RSM {

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

#ifndef __EMSCRIPTEN__
    /**
     * @brief Add search path (Native builds only)
     * @param searchPath Search path to add
     */
    void addSearchPath(const std::string &searchPath);

    /**
     * @brief Return warning messages (Native builds only)
     * @return List of warning messages
     */
    const std::vector<std::string> &getWarningMessages() const;

    /**
     * @brief Return list of processed files (Native builds only)
     * @return List of processed files (path -> node count)
     */
    const std::unordered_map<std::string, int> &getProcessedFiles() const;

    /**
     * @brief Legacy method for libxml++ Document (Native builds only)
     * @param doc libxml++ document object
     * @return Success status
     * @deprecated Use process(std::shared_ptr<IXMLDocument>) instead
     */
    bool processLegacy(xmlpp::Document *doc);
#endif

private:
#ifndef __EMSCRIPTEN__
    // Native implementation details
    std::string basePath_;
    std::vector<std::string> searchPaths_;
    std::vector<std::string> errorMessages_;
    std::vector<std::string> warningMessages_;
    std::unordered_map<std::string, int> processedFiles_;
    bool isProcessing_;
    int maxRecursionDepth_;
    int currentRecursionDepth_;

    int findAndProcessXIncludes(xmlpp::Element *element, const std::string &baseDir);
    bool processXIncludeElement(xmlpp::Element *xincludeElement, const std::string &baseDir);
    bool loadAndMergeFile(const std::string &href, xmlpp::Element *xincludeElement, const std::string &baseDir);
    std::string resolveFilePath(const std::string &href, const std::string &baseDir);
    void addError(const std::string &message);
    void addWarning(const std::string &message);
#else
    // WASM stub implementation
    std::string basePath_;
    std::vector<std::string> errorMessages_;
#endif
};

}  // namespace RSM
