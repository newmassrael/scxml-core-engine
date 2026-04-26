// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/Diagnostic.h"
#include "parsing/PositionMap.h"

#include <nlohmann/json.hpp>

#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>

namespace SCE::parsing {

// C++ exception hierarchy for top-level SCXML parser-entry failures
// thrown by `SCE::SCXMLParser::parseFile` / `parseContent` /
// `parseAbstractDocument`, plus the underlying `IXMLParser`
// implementation (`PugiXMLParser`) under RFC §W4 D1-C (typed-throw,
// no nullptr-return + getLastError poll).
//
// Two of the leaves (`ParseFileNotFound`, `ParseWrongRootElement`)
// emit distinct `xml/*` wire codes that have full Rust producers in
// `sce-build/src/parser.rs::SCXMLParser` (parse_file
// ErrorKind::NotFound branch + parse_impl root-tag check). The other
// three (`ParseXmlFailed`, `ParseException`, `ParseNoRootElement`)
// reuse `xml/parse` because the Rust error model has no producer for
// those scenarios (Result-based, no exceptions, roxmltree always-
// has-root). Wire-level consumers cannot distinguish the three
// reused-code leaves from each other — only in-process C++ consumers
// dispatch among them via `dynamic_cast`. RFC §W4 α-strict per
// `claudedocs/rfc-sce-diagnostic-wire-unification.md`.
//
// `ParseNullDocument` (sketched in the original starter inventory)
// is dropped under D1-C because PugiXMLParser throws on internal
// failure instead of returning nullptr — callers therefore never
// observe a null document and the leaf would be unreachable.

class ParseError : public std::runtime_error, public Diagnostic {
public:
    using std::runtime_error::runtime_error;

    // Reserved for future location stamping. No throw site populates
    // this today (parser-entry failures fire before sub-parsers
    // resolve a node-precise position; pugi's error offset is
    // already embedded in the message text for `ParseXmlFailed`).
    // Mirrors `XIncludeExpansionError::setLocation` per W3 RFC's
    // pinned design point.
    void setLocation(SourcePos pos) {
        location_ = std::move(pos);
    }

    // `Diagnostic` interface. Subtypes override `code()` with their
    // wire string; `location()` and `to_json()` are shared on the
    // base because every parser-entry diagnostic carries the same
    // envelope (stage = "xml", id derived through the canonical key
    // shape, message read from `runtime_error::what()`).
    std::string_view code() const noexcept override = 0;
    const std::optional<SourcePos> &location() const noexcept override {
        return location_;
    }
    nlohmann::ordered_json to_json() const override;

private:
    std::optional<SourcePos> location_;
};

// SCXML source file does not exist at the resolved path. Distinct
// from generic I/O failure — this leaf maps to a parser-entry retry
// strategy ("PATH_RETRY") on the consumer side. Mirrors
// `sce-build/src/forge/error.rs::XmlError::FileNotFound { path }`
// and maps 1:1 to the Rust `xml/file-not-found` `DiagnosticCode`.
// Thrown by `PugiXMLParser::parseFile` when `std::filesystem::exists`
// returns false (D1-C typed-throw refit per RFC §W4 Stage C).
class ParseFileNotFound : public ParseError {
public:
    using ParseError::ParseError;
    std::string_view code() const noexcept override {
        return "xml/file-not-found";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<ParseFileNotFound>(*this);
    }
};

// Pugi `load_buffer` reported a parse failure — mismatched tags,
// malformed entity, encoding error, premature EOF, etc. Reuses the
// existing Rust `xml/parse` wire code (mirrors
// `XmlError::Parse(String)`). The pugi error description is
// embedded in the message text including row/col coordinates per
// pugi's own `xml_parse_result::description()` convention. Thrown by
// `PugiXMLParser::parseFile` / `parseContent` (D1-C) when
// `xml_parse_result` evaluates to false.
class ParseXmlFailed : public ParseError {
public:
    using ParseError::ParseError;
    std::string_view code() const noexcept override {
        return "xml/parse";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<ParseXmlFailed>(*this);
    }
};

// Wraps a non-typed `std::exception` caught at the `SCXMLParser`
// entry boundary — `bad_alloc`, third-party throws, etc. Reuses
// `xml/parse` because the Rust error model has no exception analog;
// the C++-side typed dispatch (via `dynamic_cast`) is the only
// distinguishing surface. Thrown by `SCXMLParser::parseFile` /
// `parseContent` from the catch-all `std::exception&` arm.
//
// Per RFC §W4 D4 (α-strict): does NOT carry `typeid(ex).name()` —
// type-name is implementation-defined per `[lib.type.info]` and
// would emit different strings on libstdc++ / libc++ / MSVC. The
// `what()` text is the only payload.
class ParseException : public ParseError {
public:
    using ParseError::ParseError;
    std::string_view code() const noexcept override {
        return "xml/parse";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<ParseException>(*this);
    }
};

// Pugi parsed successfully but `document_element()` returned an
// invalid handle — the document parsed as well-formed XML yet
// contained no top-level element (e.g. comment-only or whitespace-
// only document). Reuses `xml/parse` because roxmltree (Rust's
// parser) rejects such input at parse time, so there is no Rust
// producer for this specific scenario. Thrown by
// `SCXMLParser::parseAbstractDocument` when `getRootElement()`
// returns null.
class ParseNoRootElement : public ParseError {
public:
    using ParseError::ParseError;
    std::string_view code() const noexcept override {
        return "xml/parse";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<ParseNoRootElement>(*this);
    }
};

// Document parsed and has a root element, but the root tag is not
// `<scxml>`. Catches the previously-silent failure mode where a
// non-SCXML document (e.g. an HTML fragment, an empty `<root/>`)
// would walk through `parseScxmlNode` and return an empty model.
// Mirrors `sce-build/src/forge/error.rs::XmlError::WrongRootElement
// { found }` and maps 1:1 to the Rust `xml/wrong-root-element`
// `DiagnosticCode`. Thrown by `SCXMLParser::parseAbstractDocument`
// when `ParsingCommon::matchNodeName(rootElement->getName(),
// "scxml")` returns false.
class ParseWrongRootElement : public ParseError {
public:
    using ParseError::ParseError;
    std::string_view code() const noexcept override {
        return "xml/wrong-root-element";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<ParseWrongRootElement>(*this);
    }
};

}  // namespace SCE::parsing
