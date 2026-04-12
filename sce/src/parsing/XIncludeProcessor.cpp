// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/XIncludeProcessor.h"
#include "core/LogMacros.h"

// ============================================================================
// Unified stub implementation: XInclude processing delegated to IXMLDocument
// ============================================================================

namespace SCE {

XIncludeProcessor::XIncludeProcessor() {
    SCE_LOG_DEBUG("Creating XInclude processor (stub)");
}

XIncludeProcessor::~XIncludeProcessor() {
    SCE_LOG_DEBUG("Destroying XInclude processor");
}

bool XIncludeProcessor::process(std::shared_ptr<IXMLDocument> doc) {
    SCE_LOG_WARN("XIncludeProcessor::process() is deprecated. Use IXMLDocument::processXInclude() instead");
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

}  // namespace SCE
