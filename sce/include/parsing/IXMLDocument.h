// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

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
     * @brief Process `<sce:use>` template expansion directives
     * @return true on success
     *
     * Expands `<sce:use template="...">` against `<sce:template>` files
     * sibling to the current document. Mirrors the AOT expander
     * `sce-build/src/template.rs` so a document accepted by one path
     * yields a byte-equivalent post-preprocessor document on the other
     * (claudedocs/rfc-sce-template-phase-b.md §1 Q1).
     *
     * Phase B M1 implements the no-op passthrough path only; any
     * `<sce:use>` element triggers a `SCE::parsing::TemplateNotImplemented`
     * exception naming the M2-M5 milestone that will support the
     * encountered shape. Implementations must throw
     * `SCE::parsing::TemplateError` (or a subtype) — never silently
     * succeed on unsupported shapes, since that is the failure mode
     * Phase B exists to close.
     */
    virtual bool processSceTemplate() = 0;

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
