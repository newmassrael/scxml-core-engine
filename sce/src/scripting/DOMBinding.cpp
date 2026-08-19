// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/DOMBinding.h"
#include "core/LogMacros.h"

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
// Native builds: Define EMSCRIPTEN_KEEPALIVE as empty macro
#define EMSCRIPTEN_KEEPALIVE
#endif

namespace SCE {

// JavaScript class ID for DOM objects
static JSClassID js_dom_element_class_id = 0;

// Forward declaration of implementation functions
namespace {

/// What a DOM handle carries.
///
/// `document` is ownership and `isDocument` is identity, and they are two
/// fields because they were one: while "has a document" meant "is the
/// document", every node handed back by `getElementsByTagName` had to be
/// created without the owning tree — an XMLElement is a view into the
/// XMLDocument's pugixml arena, so such a handle read freed memory as
/// soon as the variable the tree arrived in was overwritten.
struct DOMObjectData {
    std::shared_ptr<XMLDocument> document;
    std::shared_ptr<XMLElement> element;
    bool isDocument = false;
};

/// The properties the prototype's one getter serves, as its `magic`.
///
/// The "corresponding DOM structure" the ECMAScript data model appendix
/// asks for is read through properties — `d.firstChild.nodeName`, not
/// `d.getFirstChild()` — so these are what the frontend's member reads
/// land on. One getter with a magic number rather than thirteen
/// functions: the switch is the whole difference between them.
enum DomProperty {
    PropNodeType,
    PropNodeName,
    PropNodeValue,
    PropData,
    PropTagName,
    PropTextContent,
    PropChildNodes,
    PropFirstChild,
    PropLastChild,
    PropNextSibling,
    PropPreviousSibling,
    PropParentNode,
    PropDocumentElement,
};

// Finalizer for DOM objects
void domObjectFinalizerImpl(JSRuntime * /*rt*/, JSValue val) {
    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(val, js_dom_element_class_id));
    if (data) {
        delete data;
    }
}

JSValue createNodeObjectImpl(JSContext *ctx, std::shared_ptr<XMLDocument> document,
                             std::shared_ptr<XMLElement> element);
void ensureClassAndPrototype(JSContext *ctx);

/// A document handle is one whose `document` is set; it answers the Node
/// interface as the document and the Element vocabulary for its document
/// element, which is the delegation `getAttribute` and `getTagName` have
/// always performed.
bool isDocumentHandle(const DOMObjectData *data) {
    return data != nullptr && data->isDocument;
}

// WASM-compatible C-linkage wrapper functions for QuickJS callbacks
#ifdef __EMSCRIPTEN__
extern "C" {
EMSCRIPTEN_KEEPALIVE
#endif

JSValue dom_getElementsByTagName_wrapper(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
    if (argc < 1) {
        return JS_ThrowTypeError(ctx, "getElementsByTagName requires 1 argument");
    }

    // Get tag name
    const char *tagName = JS_ToCString(ctx, argv[0]);
    if (!tagName) {
        return JS_EXCEPTION;
    }
    std::string tagNameStr(tagName);
    JS_FreeCString(ctx, tagName);

    // Get DOM object data
    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(this_val, js_dom_element_class_id));
    if (!data) {
        return JS_ThrowTypeError(ctx, "Invalid DOM object");
    }

    // A document matches its root inclusively, an element only descends
    // into its children — DOM Level 1 Core 1.2's split.
    std::vector<std::shared_ptr<XMLElement>> elements;
    if (isDocumentHandle(data) && data->document) {
        elements = data->document->getElementsByTagName(tagNameStr);
    } else if (data->element) {
        elements = data->element->getElementsByTagName(tagNameStr);
    }

    // Create JavaScript array
    JSValue jsArray = JS_NewArray(ctx);
    for (size_t i = 0; i < elements.size(); ++i) {
        JSValue elementObj = createNodeObjectImpl(ctx, data->document, elements[i]);
        JS_SetPropertyUint32(ctx, jsArray, static_cast<uint32_t>(i), elementObj);
    }

    return jsArray;
}

EMSCRIPTEN_KEEPALIVE
JSValue dom_getAttribute_wrapper(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
    if (argc < 1) {
        return JS_ThrowTypeError(ctx, "getAttribute requires 1 argument");
    }

    // Get attribute name
    const char *attrName = JS_ToCString(ctx, argv[0]);
    if (!attrName) {
        return JS_EXCEPTION;
    }
    std::string attrNameStr(attrName);
    JS_FreeCString(ctx, attrName);

    // Get DOM element data
    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(this_val, js_dom_element_class_id));
    if (!data || !data->element) {
        return JS_ThrowTypeError(ctx, "Invalid DOM element");
    }

    // Get attribute value
    std::string attrValue = data->element->getAttribute(attrNameStr);
    return JS_NewString(ctx, attrValue.c_str());
}

EMSCRIPTEN_KEEPALIVE
JSValue dom_hasAttribute_wrapper(JSContext *ctx, JSValueConst this_val, int argc, JSValueConst *argv) {
    if (argc < 1) {
        return JS_ThrowTypeError(ctx, "hasAttribute requires 1 argument");
    }
    const char *attrName = JS_ToCString(ctx, argv[0]);
    if (!attrName) {
        return JS_EXCEPTION;
    }
    std::string attrNameStr(attrName);
    JS_FreeCString(ctx, attrName);

    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(this_val, js_dom_element_class_id));
    if (!data || !data->element) {
        return JS_ThrowTypeError(ctx, "Invalid DOM element");
    }
    return JS_NewBool(ctx, data->element->hasAttribute(attrNameStr) ? 1 : 0);
}

EMSCRIPTEN_KEEPALIVE
JSValue dom_getTagName_wrapper(JSContext *ctx, JSValueConst this_val, int /*argc*/, JSValueConst * /*argv*/) {
    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(this_val, js_dom_element_class_id));
    if (!data || !data->element) {
        return JS_ThrowTypeError(ctx, "Invalid DOM element");
    }
    return JS_NewString(ctx, data->element->getTagName().c_str());
}

EMSCRIPTEN_KEEPALIVE
JSValue dom_hasChildNodes_wrapper(JSContext *ctx, JSValueConst this_val, int /*argc*/, JSValueConst * /*argv*/) {
    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(this_val, js_dom_element_class_id));
    if (!data || !data->element) {
        return JS_ThrowTypeError(ctx, "Invalid DOM element");
    }
    // A document always has one child: its document element.
    return JS_NewBool(ctx, (isDocumentHandle(data) || data->element->hasChildNodes()) ? 1 : 0);
}

EMSCRIPTEN_KEEPALIVE
JSValue dom_property_getter(JSContext *ctx, JSValueConst this_val, int magic) {
    DOMObjectData *data = static_cast<DOMObjectData *>(JS_GetOpaque(this_val, js_dom_element_class_id));
    if (!data || !data->element) {
        return JS_ThrowTypeError(ctx, "Invalid DOM object");
    }
    const bool isDocument = isDocumentHandle(data);
    XMLElement &node = *data->element;

    switch (magic) {
    case PropNodeType:
        return JS_NewInt32(ctx, isDocument ? DomNodeType::Document : node.getNodeType());
    case PropNodeName:
        return JS_NewString(ctx, isDocument ? "#document" : node.getNodeName().c_str());
    // DOM Level 1 Core gives an element and a document a null nodeValue;
    // `data` is CharacterData's own name for the same value.
    case PropNodeValue:
    case PropData:
        if (isDocument || !node.hasNodeValue()) {
            return JS_NULL;
        }
        return JS_NewString(ctx, node.getNodeValue().c_str());
    case PropTagName:
        if (!isDocument && node.hasNodeValue()) {
            return JS_NULL;  // character data has no tag name
        }
        return JS_NewString(ctx, node.getTagName().c_str());
    case PropTextContent:
        return JS_NewString(ctx, node.getTextContent().c_str());
    case PropChildNodes: {
        JSValue jsArray = JS_NewArray(ctx);
        if (isDocument) {
            JS_SetPropertyUint32(ctx, jsArray, 0, createNodeObjectImpl(ctx, data->document, data->element));
        } else {
            std::vector<std::shared_ptr<XMLElement>> children = node.getChildNodes();
            for (size_t i = 0; i < children.size(); ++i) {
                JS_SetPropertyUint32(ctx, jsArray, static_cast<uint32_t>(i),
                                     createNodeObjectImpl(ctx, data->document, children[i]));
            }
        }
        return jsArray;
    }
    case PropFirstChild:
        if (isDocument) {
            return createNodeObjectImpl(ctx, data->document, data->element);
        }
        return createNodeObjectImpl(ctx, data->document, node.getFirstChild());
    case PropLastChild:
        if (isDocument) {
            return createNodeObjectImpl(ctx, data->document, data->element);
        }
        return createNodeObjectImpl(ctx, data->document, node.getLastChild());
    case PropNextSibling:
        if (isDocument) {
            return JS_NULL;
        }
        return createNodeObjectImpl(ctx, data->document, node.getNextSibling());
    case PropPreviousSibling:
        if (isDocument) {
            return JS_NULL;
        }
        return createNodeObjectImpl(ctx, data->document, node.getPreviousSibling());
    case PropParentNode:
        if (isDocument) {
            return JS_NULL;
        }
        return createNodeObjectImpl(ctx, data->document, node.getParentNode());
    // Only the document handle carries documentElement, which is how a
    // document can tell the two kinds apart without reading nodeType.
    case PropDocumentElement:
        if (!isDocument) {
            return JS_NULL;
        }
        return createNodeObjectImpl(ctx, data->document, data->element);
    default:
        return JS_UNDEFINED;
    }
}

#ifdef __EMSCRIPTEN__
}  // extern "C"
#endif

void defineProperty(JSContext *ctx, JSValue proto, const char *name, DomProperty property) {
    JSCFunctionType fn;
    fn.getter_magic = dom_property_getter;
    JSValue getter = JS_NewCFunction2(ctx, fn.generic, name, 0, JS_CFUNC_getter_magic, property);
    JSAtom atom = JS_NewAtom(ctx, name);
    JS_DefinePropertyGetSet(ctx, proto, atom, getter, JS_UNDEFINED, JS_PROP_CONFIGURABLE | JS_PROP_ENUMERABLE);
    JS_FreeAtom(ctx, atom);
}

/// Install the class once per runtime and its prototype once per context.
///
/// The prototype is why this is one call rather than a per-object member
/// install: `childNodes` on a deep tree would otherwise define thirteen
/// getters and five methods on every node it hands back.
void ensureClassAndPrototype(JSContext *ctx) {
    if (js_dom_element_class_id == 0) {
        JS_NewClassID(JS_GetRuntime(ctx), &js_dom_element_class_id);
        JSClassDef classDef = {
            .class_name = "DOMNode",
            .finalizer = domObjectFinalizerImpl,
            .gc_mark = nullptr,
            .call = nullptr,
            .exotic = nullptr,
        };
        JS_NewClass(JS_GetRuntime(ctx), js_dom_element_class_id, &classDef);
    }

    // Class IDs live in the runtime, prototypes in the context, so a
    // second context in the same runtime needs its own.
    JSValue existing = JS_GetClassProto(ctx, js_dom_element_class_id);
    if (JS_IsObject(existing)) {
        JS_FreeValue(ctx, existing);
        return;
    }
    JS_FreeValue(ctx, existing);

    JSValue proto = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, proto, "getElementsByTagName",
                      JS_NewCFunction(ctx, dom_getElementsByTagName_wrapper, "getElementsByTagName", 1));
    JS_SetPropertyStr(ctx, proto, "getAttribute", JS_NewCFunction(ctx, dom_getAttribute_wrapper, "getAttribute", 1));
    JS_SetPropertyStr(ctx, proto, "hasAttribute", JS_NewCFunction(ctx, dom_hasAttribute_wrapper, "hasAttribute", 1));
    JS_SetPropertyStr(ctx, proto, "getTagName", JS_NewCFunction(ctx, dom_getTagName_wrapper, "getTagName", 0));
    JS_SetPropertyStr(ctx, proto, "hasChildNodes", JS_NewCFunction(ctx, dom_hasChildNodes_wrapper, "hasChildNodes", 0));

    defineProperty(ctx, proto, "nodeType", PropNodeType);
    defineProperty(ctx, proto, "nodeName", PropNodeName);
    defineProperty(ctx, proto, "nodeValue", PropNodeValue);
    defineProperty(ctx, proto, "data", PropData);
    defineProperty(ctx, proto, "tagName", PropTagName);
    defineProperty(ctx, proto, "textContent", PropTextContent);
    defineProperty(ctx, proto, "childNodes", PropChildNodes);
    defineProperty(ctx, proto, "firstChild", PropFirstChild);
    defineProperty(ctx, proto, "lastChild", PropLastChild);
    defineProperty(ctx, proto, "nextSibling", PropNextSibling);
    defineProperty(ctx, proto, "previousSibling", PropPreviousSibling);
    defineProperty(ctx, proto, "parentNode", PropParentNode);
    defineProperty(ctx, proto, "documentElement", PropDocumentElement);

    JS_SetClassProto(ctx, js_dom_element_class_id, proto);
}

/// Wrap one node. `document` is set only for the handle that stands for
/// the document itself; every other handle is a node of that same tree,
/// and pugixml's document node — what a climb from the root element
/// reaches — is pushed as the document handle so `parentNode` answers
/// with the value the variable holds rather than a third shape.
JSValue createNodeObjectImpl(JSContext *ctx, std::shared_ptr<XMLDocument> owner, std::shared_ptr<XMLElement> element) {
    if (!element) {
        return JS_NULL;
    }
    ensureClassAndPrototype(ctx);

    JSValue obj = JS_NewObjectClass(ctx, js_dom_element_class_id);
    if (JS_IsException(obj)) {
        return obj;
    }

    DOMObjectData *data = new DOMObjectData();
    data->document = std::move(owner);
    // pugixml's document node is the parent of a document element, so a
    // climb from the root lands on it. It becomes the document handle —
    // the same value the variable holds — rather than a third shape.
    if (element->getNodeType() == DomNodeType::Document && data->document) {
        data->isDocument = true;
        data->element = data->document->getDocumentElement();
    } else {
        data->element = std::move(element);
    }
    JS_SetOpaque(obj, data);

    return obj;
}

}  // anonymous namespace

void DOMBinding::resetClassId() {
    // §scxml-B-2: Reset DOM class ID when JSEngine is reset/shutdown
    // QuickJS class IDs are runtime-specific and must be reinitialized for new runtimes
    js_dom_element_class_id = 0;
}

JSValue DOMBinding::createDOMObject(JSContext *ctx, const std::string &xmlContent) {
    // Parse XML
    auto document = std::make_shared<XMLDocument>(xmlContent);
    if (!document->isValid()) {
        // Debug rather than error, and the caller decides what the refusal
        // means: `parseEventData` abandons this reading for the next one the
        // clause names, which is the ordinary path for every `error.*` message
        // the engine raises.
        SCE_LOG_DEBUG("DOMBinding: content is not a valid XML document - {}", document->getErrorMessage());
        return JS_ThrowSyntaxError(ctx, "Failed to parse XML content");
    }

    ensureClassAndPrototype(ctx);

    JSValue obj = JS_NewObjectClass(ctx, js_dom_element_class_id);
    if (JS_IsException(obj)) {
        return obj;
    }

    // The document handle: the tree's owner, answering the Element
    // vocabulary for its document element.
    DOMObjectData *data = new DOMObjectData();
    data->document = document;
    data->element = document->getDocumentElement();
    data->isDocument = true;
    JS_SetOpaque(obj, data);

    return obj;
}

}  // namespace SCE
