// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IXMLElement.h"
#include "parsing/PositionMap.h"
#include <memory>
#include <string>

namespace SCE {

// Result of `IXMLDocument::processXInclude`. Carries the XInclude-
// expansion `PositionMap` alongside the success flag so downstream
// diagnostic emitters can remap in-memory coordinates back to the
// author's source file (or to a `xi:include`'d fragment file). See
// `claudedocs/rfc-sce-template-phase-x.md` §3 B2. The PositionMap
// is identity when `<xi:include>` is absent from the source
// document and otherwise captures every spliced fragment byte with
// File-origin attribution to the fragment's source file.
struct XIncludeResult {
    bool ok = false;
    SCE::parsing::PositionMap positions;
};

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
     * @return XIncludeResult { ok, positions }
     *
     * W3C XInclude: Replaces <xi:include> elements with external
     * content via the string-level expander
     * `SCE::parsing::expandStringX` (mirrors
     * `sce-build/src/xinclude.rs::expand`). Returns a `PositionMap`
     * tracking every emitted byte back to its source file so
     * post-expansion diagnostics can resolve to author-source
     * (file, row, col) — including bytes that originated in an
     * `xi:include`'d fragment. See
     * `claudedocs/rfc-sce-template-phase-x.md` §3 B2.
     */
    virtual XIncludeResult processXInclude() = 0;

    /**
     * @brief Process `<sce:use>` template expansion directives
     * @param upstream PositionMap describing every byte of the
     *        post-XInclude document text — typically the
     *        `XIncludeResult::positions` returned by
     *        `processXInclude`. The expander composes this map into
     *        its own output so a diagnostic byte position resolves
     *        through both preprocessor stages back to the author's
     *        source file (host or `xi:include`'d fragment), per
     *        Phase X RFC §1 Q2.
     * @return SceTemplateResult { ok, positions }
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
    virtual SceTemplateResult processSceTemplate(
        const SCE::parsing::PositionMap &upstream) = 0;

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
