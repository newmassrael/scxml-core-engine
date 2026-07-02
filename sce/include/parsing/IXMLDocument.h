// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IXMLElement.h"
#include "parsing/PositionMap.h"
#include <memory>

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
     * @return PositionMap tracking every emitted byte back to its
     *         source file
     *
     * W3C XInclude: Replaces <xi:include> elements with external
     * content via the string-level expander
     * `SCE::parsing::expandStringX` (mirrors
     * `sce-build/src/xinclude.rs::expand`). The returned PositionMap
     * lets post-expansion diagnostics resolve to author-source
     * (file, row, col) — including bytes that originated in an
     * `xi:include`'d fragment. See
     * `sce/include/parsing/XIncludeExpander.h` for the contract.
     *
     * Throws on failure: `SCE::parsing::XIncludeExpansionError`
     * (typed subtypes from `sce/include/parsing/XIncludeError.h`)
     * for expansion failures, `SCE::parsing::ParseXmlFailed` for
     * reparse failures of the spliced text. §wire-W4.5 D2/D3.
     */
    virtual SCE::parsing::PositionMap processXInclude() = 0;

    /**
     * @brief Process `<sce:use>` template expansion directives
     * @param upstream PositionMap describing every byte of the
     *        post-XInclude document text — typically the value
     *        returned by `processXInclude`. The expander composes
     *        this map into its own output so a diagnostic byte
     *        position resolves through both preprocessor stages
     *        back to the author's source file (host or
     *        `xi:include`'d fragment).
     * @return PositionMap composing `upstream` with the
     *         template-expansion mapping
     *
     * Expands `<sce:use template="...">` against `<sce:template>` files
     * sibling to the current document. Mirrors the AOT expander
     * `sce-build/src/template.rs` so a document accepted by one path
     * yields a byte-equivalent post-preprocessor document on the
     * other; the parity harness under `tests/w3c_template_parity`
     * asserts the equivalence fixture-by-fixture.
     *
     * Throws on failure: `SCE::parsing::TemplateError` (typed
     * subtypes from `sce/include/parsing/TemplateError.h`) for
     * expansion failures, `SCE::parsing::ParseXmlFailed` for
     * reparse failures. §wire-W4.5 D2.
     */
    virtual SCE::parsing::PositionMap processSceTemplate(const SCE::parsing::PositionMap &upstream) = 0;

    /**
     * @brief Check if document is valid
     * @return true if document loaded successfully
     *
     * Used by `DataModelItem` for valid-doc gating before mutating
     * the wrapped DOM. Under W4 D1-C the underlying parsers throw
     * on failure, so under typed-throw the predicate is
     * monotonically true once a wrapper survives parser entry —
     * the method stays for any caller that needs the legacy
     * predicate shape.
     */
    virtual bool isValid() const = 0;
};

}  // namespace SCE
