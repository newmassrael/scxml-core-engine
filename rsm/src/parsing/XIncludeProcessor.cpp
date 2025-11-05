#include "parsing/XIncludeProcessor.h"
#include "common/Logger.h"

// ============================================================================
// Unified stub implementation: XInclude processing delegated to IXMLDocument
// ============================================================================

namespace RSM {

XIncludeProcessor::XIncludeProcessor() {
    LOG_DEBUG("Creating XInclude processor (stub)");
}

XIncludeProcessor::~XIncludeProcessor() {
    LOG_DEBUG("Destroying XInclude processor");
}

bool XIncludeProcessor::process(std::shared_ptr<IXMLDocument> doc) {
    LOG_WARN("XIncludeProcessor::process() is deprecated. Use IXMLDocument::processXInclude() instead");
    if (doc) {
        return doc->processXInclude();
    }
    return false;
}

void XIncludeProcessor::setBasePath(const std::string &basePath) {
    basePath_ = basePath;
}

const std::vector<std::string> &XIncludeProcessor::getErrorMessages() const {
    return errorMessages_;
}

}  // namespace RSM
