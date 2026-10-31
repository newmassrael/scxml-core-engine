-- SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
-- SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
--
-- The DOM that XML in the data model is: a `<data>` element's XML content and an XML `_event.data`
-- (W3C SCXML B.2.1 and B.2.8.1: "the corresponding DOM structure"). The read surface is DOM Level 1
-- Core, as far as `tests/ecmascript/dom_read_surface.json` states it, and that table is what every
-- other binding is measured against, so this file is too: it is written in Lua, which is the one
-- spelling the lowered expression ever reaches (`d:getAttribute("x")`, `d.firstChild.nodeType`,
-- `#d.childNodes`), and a member it does not answer is a nil index.
--
-- The tree is the cpp reference backend's, which parses with pugixml's `parse_default`: a
-- whitespace-ONLY run of text, a comment and a processing instruction are not nodes; a run that holds
-- anything else keeps its whitespace; a CDATA section is a node of its own kind; line ends are
-- normalized and the whitespace of an attribute value is a space.
--
-- It does not need an XML library of the platform: a Lua state that can run the ECMAScript library
-- can read the markup, on every platform the engine runs on.

local ELEMENT, TEXT, CDATA, DOCUMENT = 1, 3, 4, 9

local entities = { amp = "&", lt = "<", gt = ">", quot = '"', apos = "'" }

--- The text with its character and entity references replaced; a reference this does not know is
--- left as it was written.
local function decode(text)
    return (text:gsub("&(#?[%w]+);", function(name)
        local named = entities[name]
        if named then
            return named
        end
        local code = name:match("^#[xX](%x+)$")
        code = code and tonumber(code, 16) or tonumber(name:match("^#(%d+)$"))
        if code then
            local ok, character = pcall(utf8.char, code)
            if ok then
                return character
            end
        end
        return nil
    end))
end

local methods = {}

--- The element a call is about: a document answers the Element vocabulary for its document element,
--- which `getAttribute` and `getTagName` did before this surface existed.
local function element_of(node)
    if node.nodeType == DOCUMENT then
        return node.documentElement
    end
    return node
end

function methods.hasChildNodes(self)
    return #self.childNodes > 0
end

function methods.getAttribute(self, name)
    local element = element_of(self)
    local value = element and element._attributes[name]
    if value == nil then
        return ""
    end
    return value
end

function methods.hasAttribute(self, name)
    local element = element_of(self)
    return element ~= nil and element._attributes[name] ~= nil
end

function methods.getTagName(self)
    local element = element_of(self)
    return element and element.tagName
end

--- The elements below this node named `name`, in document order, as a 1-based sequence; `*` names
--- them all.
function methods.getElementsByTagName(self, name)
    local found = {}
    local function walk(node)
        for _, child in ipairs(node.childNodes) do
            if child.nodeType == ELEMENT then
                if name == "*" or child.tagName == name then
                    found[#found + 1] = child
                end
                walk(child)
            end
        end
    end
    walk(self)
    return found
end

--- The text of a node and of everything below it, markup removed.
local function text_of(node)
    if node.nodeType == TEXT or node.nodeType == CDATA then
        return node.nodeValue
    end
    local parts = {}
    for _, child in ipairs(node.childNodes) do
        parts[#parts + 1] = text_of(child)
    end
    return table.concat(parts)
end

local node_mt = {
    __index = function(self, key)
        if key == "textContent" then
            return text_of(self)
        end
        return methods[key]
    end,
}

local function new_node(fields)
    return setmetatable(fields, node_mt)
end

local function append(parent, node)
    local siblings = parent.childNodes
    node.parentNode = parent
    local last = siblings[#siblings]
    if last then
        last.nextSibling = node
        node.previousSibling = last
    end
    siblings[#siblings + 1] = node
    parent.firstChild = siblings[1]
    parent.lastChild = node
end

--- The position just past a `<!...>` declaration that starts at `at`, which may hold an internal
--- subset in brackets.
local function end_of_declaration(text, at)
    local depth = 0
    for i = at, #text do
        local c = text:sub(i, i)
        if c == "[" then
            depth = depth + 1
        elseif c == "]" then
            depth = depth - 1
        elseif c == ">" and depth <= 0 then
            return i + 1
        end
    end
    error("unterminated declaration at byte " .. at)
end

--- The document the XML `text` is. Raises on markup that is not well formed, which is the caller's
--- `error.execution`.
function _scxml_parse_xml(text)
    text = text:gsub("^\239\187\191", ""):gsub("\r\n?", "\n")
    local document = new_node({ nodeType = DOCUMENT, nodeName = "#document", childNodes = {} })
    local current = document
    local position = 1
    local length = #text

    local function add_text(raw, kind)
        if kind == TEXT and raw:match("^[ \t\n]*$") then
            return
        end
        local data = kind == CDATA and raw or decode(raw)
        append(current, new_node({
            nodeType = kind,
            nodeName = kind == CDATA and "#cdata-section" or "#text",
            nodeValue = data,
            data = data,
            childNodes = {},
        }))
    end

    while position <= length do
        local opens = text:find("<", position, true)
        if not opens then
            add_text(text:sub(position), TEXT)
            break
        end
        if opens > position then
            add_text(text:sub(position, opens - 1), TEXT)
        end
        if text:sub(opens, opens + 3) == "<!--" then
            local closes = text:find("-->", opens + 4, true)
            if not closes then
                error("unterminated comment at byte " .. opens)
            end
            position = closes + 3
        elseif text:sub(opens, opens + 8) == "<![CDATA[" then
            local closes = text:find("]]>", opens + 9, true)
            if not closes then
                error("unterminated CDATA section at byte " .. opens)
            end
            add_text(text:sub(opens + 9, closes - 1), CDATA)
            position = closes + 3
        elseif text:sub(opens, opens + 1) == "<?" then
            local closes = text:find("?>", opens + 2, true)
            if not closes then
                error("unterminated processing instruction at byte " .. opens)
            end
            position = closes + 2
        elseif text:sub(opens, opens + 1) == "<!" then
            position = end_of_declaration(text, opens)
        elseif text:sub(opens, opens + 1) == "</" then
            local name, after = text:match("^</([^%s>]+)%s*>()", opens)
            if not name or current.nodeType ~= ELEMENT or current.tagName ~= name then
                error("end tag does not close the open element at byte " .. opens)
            end
            current = current.parentNode
            position = after
        else
            local name, after = text:match("^<([^%s/>]+)()", opens)
            if not name then
                error("malformed start tag at byte " .. opens)
            end
            local attributes = {}
            local cursor = after
            local empty
            while true do
                cursor = text:find("[^ \t\n]", cursor)
                if not cursor then
                    error("unterminated start tag at byte " .. opens)
                end
                if text:sub(cursor, cursor) == ">" then
                    position, empty = cursor + 1, false
                    break
                elseif text:sub(cursor, cursor + 1) == "/>" then
                    position, empty = cursor + 2, true
                    break
                end
                local attribute, _, value, following = text:match("^([^%s=/>]+)%s*=%s*([\"'])(.-)%2()", cursor)
                if not attribute then
                    error("malformed attribute at byte " .. cursor)
                end
                attributes[attribute] = decode((value:gsub("[\t\n]", " ")))
                cursor = following
            end
            local element = new_node({
                nodeType = ELEMENT,
                nodeName = name,
                tagName = name,
                childNodes = {},
                _attributes = attributes,
            })
            append(current, element)
            if current == document then
                document.documentElement = element
                document.tagName = name
            end
            if not empty then
                current = element
            end
        end
    end

    if current ~= document then
        error("the element '" .. current.tagName .. "' is never closed")
    end
    if not document.documentElement then
        error("the document has no element")
    end
    return document
end
