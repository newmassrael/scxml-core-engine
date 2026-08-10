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
#include <vector>

namespace SCE::parsing {

// C++ exception hierarchy for top-level SCXML parser-entry failures
// thrown by `SCE::SCXMLParser::parseFile` / `parseContent` /
// `parseAbstractDocument`, plus the underlying `IXMLParser`
// implementation (`PugiXMLParser`) under §wire-W4 D1-C (typed-throw,
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
// dispatch among them via `dynamic_cast`. NEW wire codes exist only
// where a matching Rust producer exists (§wire-W4).
//
// `ParseNullDocument` (sketched in the original starter inventory)
// is dropped under D1-C because PugiXMLParser throws on internal
// failure instead of returning nullptr — callers therefore never
// observe a null document and the leaf would be unreachable.
//
// Location stamping follows the §wire-W3 design pin, which this
// family now inherits from the `Diagnostic` base rather than
// declaring for itself: the parser boundary names the document, and
// a leaf that resolved coordinates attaches them.

class ParseError : public std::runtime_error, public Diagnostic {
public:
    // `Diagnostic` interface. Subtypes override `code()` with their
    // wire string; `stage()`, `location()` and `to_json()` are shared
    // on the base because every parser-entry diagnostic carries the
    // same envelope.
    std::string_view code() const noexcept override = 0;

    // Every parser-entry `xml/*` `DiagnosticCode` reports the `xml`
    // stage in the Rust authority — see `DiagnosticCode::stage()` in
    // `sce-build/src/forge/diagnostic.rs`.
    std::string_view stage() const noexcept override {
        return "xml";
    }

    nlohmann::ordered_json to_json() const override;

protected:
    ParseError(std::string message, std::vector<std::string> keyFragments, std::optional<std::string> actual)
        : std::runtime_error(std::move(message)), Diagnostic(std::move(keyFragments)), actual_(std::move(actual)) {}

    // `expected` — the closed set of values that would have been
    // accepted. Only the wrong-root leaf has one; the default emits
    // nothing.
    virtual void appendExpected(nlohmann::ordered_json &out) const;

private:
    std::optional<std::string> actual_;
};

// SCXML source file does not exist at the resolved path. Distinct
// from generic I/O failure — this leaf maps to a parser-entry retry
// strategy ("PATH_RETRY") on the consumer side. Mirrors
// `sce-build/src/forge/error.rs::XmlError::FileNotFound { path }`
// and maps 1:1 to the Rust `xml/file-not-found` `DiagnosticCode`.
// Thrown by `PugiXMLParser::parseFile` when `std::filesystem::exists`
// returns false (D1-C typed-throw refit per §wire-W4 Stage C).
class ParseFileNotFound : public ParseError {
public:
    // `detail` is empty for the not-found case and carries the open
    // failure for the exists-but-unreadable one, which routes here
    // because no Rust producer distinguishes it either. It renders
    // into the message and stays out of the key fragments — the path
    // is what identifies the failure, and the platform\'s errno text
    // would make the id platform-specific.
    explicit ParseFileNotFound(std::string path, std::string detail = {})
        : ParseError(detail.empty() ? "SCXML file not found: " + path
                                    : "SCXML file not found: " + path + " (cannot open: " + detail + ")",
                     {path}, path) {}

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
    explicit ParseXmlFailed(std::string detail) : ParseError("XML parse error: " + detail, {}, std::nullopt) {}

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
// Per §wire-W4 D4 (α-strict): does NOT carry `typeid(ex).name()` —
// type-name is implementation-defined per `[lib.type.info]` and
// would emit different strings on libstdc++ / libc++ / MSVC. The
// `what()` text is the only payload.
class ParseException : public ParseError {
public:
    explicit ParseException(std::string detail) : ParseError("XML parse error: " + detail, {}, std::nullopt) {}

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
    ParseNoRootElement() : ParseError("XML parse error: no root element found", {}, std::nullopt) {}

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
    explicit ParseWrongRootElement(std::string found)
        : ParseError("Root element is not <scxml>, found: <" + found + ">", {found}, found) {}

    std::string_view code() const noexcept override {
        return "xml/wrong-root-element";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<ParseWrongRootElement>(*this);
    }

protected:
    void appendExpected(nlohmann::ordered_json &out) const override;
};

}  // namespace SCE::parsing
