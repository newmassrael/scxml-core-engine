// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IXMLElement.h"
#include "parsing/PositionMap.h"
#include <memory>
#include <string>

namespace SCE {

// Result of `IXMLDocument::processSceTemplate`. Carries the
// template-expansion `PositionMap` alongside the success flag so
// downstream diagnostic emitters can remap in-memory coordinates
// back to the author's source file. See
// `claudedocs/rfc-sce-template-phase-c.md` §3 P2. The PositionMap
// is identity when `<sce:use>` is absent from the source document
// and otherwise captures every spliced template body byte with
// File/CallSite origin attribution (RFC §6.3 Q3 depth-1 rule).
struct SceTemplateResult {
    bool ok = false;
    SCE::parsing::PositionMap positions;
};


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
     * Implementations must throw `SCE::parsing::TemplateError`
     * (or one of its typed subtypes — see
     * `sce/include/parsing/TemplateError.h`) on any failure, never
     * silently succeed on unsupported shapes. The typed subtypes map
     * 1:1 to the Rust `xml/template-*` DiagnosticCode set so the
     * AOT and Interpreter paths surface the same class of failure.
     */
    virtual SceTemplateResult processSceTemplate() = 0;

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
