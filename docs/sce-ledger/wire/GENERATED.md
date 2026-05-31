# GENERATED.md — atomic store derived view

this file `mnemosyne-cli generate-docs` output — direct no edit. atomic store (`docs/.atomic/workspace.atomic.json`) in mutate primitive (`set-section-*` / `append-changelog-entry`) pass and then re-generate.

Source: `docs/sce-ledger/wire/.atomic/workspace.atomic.json`

---

## Sections

### §wire-W0. contract (this RFC; commit-series contract; prerequisite for W1)













### §wire-W1. contract (this RFC; commit-series contract)











**Implementations**:
- sce/include/parsing/Diagnostic.h:clone
- sce/include/parsing/SCXMLParser.h:getDiagnostics




### §wire-W2. scope sketch — library API only (no CLI)











**Implementations**:
- sce/include/parsing/Diagnostic.h:to_canonical_json_string
- sce/include/parsing/DiagnosticBatchFormatter.h
- sce/src/parsing/Diagnostic.cpp:Diagnostic::to_canonical_json_string




### §wire-W3. scope sketch











**Implementations**:
- sce/include/parsing/XIncludeError.h:SCE::parsing
- sce/include/parsing/XIncludeError.h:setLocation
- sce/include/parsing/XIncludeExpander.h




### §wire-W4. LANDED 2026-04-26 (α-strict, D1-C typed-throw)











**Implementations**:
- sce/include/parsing/IXMLParser.h:parseFile
- sce/include/parsing/ParseError.h:ParseException
- sce/include/parsing/ParseError.h:ParseFileNotFound
- sce/include/parsing/ParseError.h:SCE::parsing
- sce/src/parsing/PugiXMLParser.cpp:PugiXMLParser::parseFile
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseAbstractDocument
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseContent
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseFile
- sce/src/runtime/StateMachine.cpp:StateMachine::processEvent




### §wire-W4.5. LANDED 2026-04-26 (debt repayment, polling surface removed)











**Implementations**:
- sce/include/parsing/IXMLDocument.h:processSceTemplate
- sce/include/parsing/IXMLDocument.h:processXInclude
- sce/src/parsing/PugiXMLParser.cpp:PugiXMLDocument::processSceTemplate
- sce/src/parsing/PugiXMLParser.cpp:PugiXMLDocument::processXInclude
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseContent
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseFile
- sce/src/parsing/XIncludeProcessor.cpp:XIncludeProcessor::process




### §wire-W5. LANDED 2026-04-26 (semantic family typed-throw, test-as-consumer + dead-code cleanup)











**Implementations**:
- sce/include/parsing/SemanticError.h:SCE::parsing
- sce/include/parsing/SemanticError.h:SemanticTopLevelScriptUnloaded
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseContent
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseFile
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::parseScxmlNode
- sce/src/parsing/SCXMLParser.cpp:SCE::SCXMLParser::validateModel
- sce/src/parsing/SemanticError.cpp:SCE::parsing




## Changelog (atomic ledger)

(empty — first atomic entry will populate this section.)

