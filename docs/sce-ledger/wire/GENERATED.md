# GENERATED.md — atomic store derived view

this file `mnemosyne-cli generate-docs` output — direct no edit. atomic store (`docs/.atomic/workspace.atomic.json`) in mutate primitive (`set-section-*` / `append-changelog-entry`) pass and then re-generate.

Source: `docs/sce-ledger/wire/.atomic/workspace.atomic.json`

---

## Sections

### §wire-W0. contract (this RFC; commit-series contract; prerequisite for W1)














### §wire-W1. contract (this RFC; commit-series contract)











**Bindings**:
- [implements] sce/include/parsing/Diagnostic.h:clone
- [implements] sce/include/parsing/SCXMLParser.h:getDiagnostics





### §wire-W2. scope sketch — library API only (no CLI)











**Bindings**:
- [implements] sce/include/parsing/Diagnostic.h:to_canonical_json_string
- [implements] sce/include/parsing/DiagnosticBatchFormatter.h
- [implements] sce/src/parsing/Diagnostic.cpp:Diagnostic::to_canonical_json_string





### §wire-W3. scope sketch











**Bindings**:
- [implements] sce/include/parsing/XIncludeError.h:SCE::parsing
- [implements] sce/include/parsing/XIncludeError.h:setLocation
- [implements] sce/include/parsing/XIncludeExpander.h





### §wire-W4. LANDED 2026-04-26 (α-strict, D1-C typed-throw)











**Bindings**:
- [implements] sce/include/parsing/IXMLParser.h:parseFile
- [implements] sce/include/parsing/ParseError.h:ParseException
- [implements] sce/include/parsing/ParseError.h:ParseFileNotFound
- [implements] sce/include/parsing/ParseError.h:SCE::parsing
- [implements] sce/src/parsing/PugiXMLParser.cpp:PugiXMLParser::parseFile
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseAbstractDocument
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseContent
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseFile
- [implements] sce/src/runtime/StateMachine.cpp:StateMachine::processEvent





### §wire-W4.5. LANDED 2026-04-26 (debt repayment, polling surface removed)











**Bindings**:
- [implements] sce/include/parsing/IXMLDocument.h:processSceTemplate
- [implements] sce/include/parsing/IXMLDocument.h:processXInclude
- [implements] sce/src/parsing/PugiXMLParser.cpp:PugiXMLDocument::processSceTemplate
- [implements] sce/src/parsing/PugiXMLParser.cpp:PugiXMLDocument::processXInclude
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseContent
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseFile
- [implements] sce/src/parsing/XIncludeProcessor.cpp:XIncludeProcessor::process





### §wire-W5. LANDED 2026-04-26 (semantic family typed-throw, test-as-consumer + dead-code cleanup)











**Bindings**:
- [implements] sce/include/parsing/SemanticError.h:SCE::parsing
- [implements] sce/include/parsing/SemanticError.h:SemanticTopLevelScriptUnloaded
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseContent
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseFile
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseScxmlNode
- [implements] sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::validateModel
- [implements] sce/src/parsing/SemanticError.cpp:SCE::parsing





## Changelog (atomic ledger)

(empty — first atomic entry will populate this section.)

