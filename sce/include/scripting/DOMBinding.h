// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "scripting/XMLDOMWrapper.h"
#include <memory>
#include <quickjs.h>
#include <string>

namespace SCE {

/**
 * §scxml-B-2: QuickJS bindings for XML DOM API
 *
 * Creates JavaScript-accessible DOM nodes carrying DOM Level 1 Core's
 * read surface — `getElementsByTagName` / `getAttribute` /
 * `hasAttribute` / `getTagName` / `hasChildNodes`, and the Node
 * interface as properties (`nodeType`, `nodeName`, `nodeValue`,
 * `childNodes`, `firstChild`, `lastChild`, `nextSibling`,
 * `previousSibling`, `parentNode`, `textContent`, `tagName`, `data`,
 * `documentElement`) — installed once on a per-context prototype.
 *
 * The class holds no members and declares no callbacks: the opaque
 * payload, the finalizer and every callback live in DOMBinding.cpp's
 * anonymous namespace, because a second declaration of the payload was a
 * second *type* with the same layout, and the finalizer that deleted
 * through it only worked while the layouts happened to agree.
 */
class DOMBinding {
public:
    /**
     * Reset DOM class ID (must be called when JSEngine is reset/shutdown)
     * §scxml-B-2: Ensures DOM class ID is reinitialized for new QuickJS runtime
     */
    static void resetClassId();

    /**
     * Create a JavaScript DOM object from XML content
     *
     * Returns the document handle: it owns the parsed tree, answers the
     * Node interface as a document, and answers the Element vocabulary
     * for its document element. Every node reached from it carries the
     * same owning document, so an element assigned to one variable stays
     * readable after the variable the tree arrived in is overwritten.
     */
    static JSValue createDOMObject(JSContext *ctx, const std::string &xmlContent);
};

}  // namespace SCE
