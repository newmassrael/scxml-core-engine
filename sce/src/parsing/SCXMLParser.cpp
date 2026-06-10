// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/SCXMLParser.h"
#include "GuardUtils.h"
#include "backends/LogUtils.h"
#include "core/LogMacros.h"
#include "parsing/Diagnostic.h"
#include "parsing/IXMLParser.h"
#include "parsing/ParseError.h"
#include "parsing/ParsingCommon.h"
#include "parsing/SemanticError.h"
#include "parsing/TemplateError.h"
#include "parsing/XIncludeError.h"

#include "parsing/IXMLElement.h"

#include <algorithm>
#include <filesystem>
#include <utility>

SCE::SCXMLParser::SCXMLParser(std::shared_ptr<SCE::NodeFactory> nodeFactory,
                              std::shared_ptr<SCE::IXIncludeProcessor> xincludeProcessor)
    : nodeFactory_(nodeFactory) {
    SCE_LOG_DEBUG("Creating SCXML parser");

    // Initialize specialized parsers
    stateNodeParser_ = std::make_shared<SCE::StateNodeParser>(nodeFactory_);
    transitionParser_ = std::make_shared<SCE::TransitionParser>(nodeFactory_);
    actionParser_ = std::make_shared<SCE::ActionParser>(nodeFactory_);
    guardParser_ = std::make_shared<SCE::GuardParser>(nodeFactory_);
    dataModelParser_ = std::make_shared<SCE::DataModelParser>(nodeFactory_);
    invokeParser_ = std::make_shared<SCE::InvokeParser>(nodeFactory_);
    doneDataParser_ = std::make_shared<SCE::DoneDataParser>(nodeFactory_);

    // Connect related parsers
    stateNodeParser_->setRelatedParsers(transitionParser_, actionParser_, dataModelParser_, invokeParser_,
                                        doneDataParser_);

    // Set ActionParser for TransitionParser
    transitionParser_->setActionParser(actionParser_);

    // Set up XInclude processor
    if (xincludeProcessor) {
        xincludeProcessor_ = xincludeProcessor;
    } else {
        xincludeProcessor_ = std::make_shared<SCE::XIncludeProcessor>();
    }
}

SCE::SCXMLParser::~SCXMLParser() {
    SCE_LOG_DEBUG("Destroying SCXML parser");
}

std::shared_ptr<SCE::SCXMLModel> SCE::SCXMLParser::parseFile(const std::string &filename) {
    try {
        // Initialize parsing state
        initParsing();

        SCE_LOG_INFO("Parsing SCXML file: {}", filename);

        // §scxml-5.8: Set base path for external script resolution
        std::filesystem::path scxmlPath(filename);
        std::string basePath = scxmlPath.parent_path().string();
        actionParser_->setScxmlBasePath(basePath);
        SCE_LOG_DEBUG("Set SCXML base path for external script resolution: {}", basePath);

        // §wire-W4 D1-C: PugiXMLParser throws `ParseFileNotFound` /
        // `ParseXmlFailed` on parse-entry failures; the caller no
        // longer polls a nullable result + `getLastError()`. The
        // existing `if (!doc || !doc->isValid())` branch became dead
        // code under typed-throw and is removed.
        auto xmlParser = IXMLParser::create();
        auto doc = xmlParser->parseFile(filename);

        // Process XIncludes; the produced PositionMap threads into
        // `processSceTemplate` so map composition unifies
        // post-XInclude and post-template diagnostic
        // coordinates through a single map. §wire-W4.5 D1: typed
        // throws bubble to the catch arms below
        // (XIncludeExpansionError / ParseError) — there is no
        // longer a polling result.
        SCE_LOG_DEBUG("Processing XIncludes");
        auto xincludePositions = doc->processXInclude();

        // Process `<sce:use>` template expansion. Each failure
        // mode raises a typed `SCE::parsing::TemplateError`
        // subtype (see `sce/include/parsing/TemplateError.h` for
        // the 8-variant set) caught below; reparse failures of the
        // spliced text raise `ParseXmlFailed` per §wire-W4.5 D2.
        SCE_LOG_DEBUG("Processing sce:template");
        documentPositions_ = doc->processSceTemplate(xincludePositions);

        // Parse document
        return parseAbstractDocument(doc);
    } catch (const SCE::parsing::ParseError &pe) {
        // §wire-W4 D1-C: PugiXMLParser + parseAbstractDocument throw
        // typed `ParseError` subtypes for parser-entry failures
        // (file-not-found, malformed XML, no/wrong root element).
        // Q4-B coexistence: `addError` populates the legacy string
        // surface; `recordDiagnostic` populates the typed
        // `getDiagnostics()` surface that consumers dispatch on
        // (§wire-W4 D8 ParseErrorConsumer.TypedCodeDistinguishesFailureClass*).
        addError(pe.what());
        recordDiagnostic(pe.clone());
        return nullptr;
    } catch (const SCE::parsing::TemplateError &tpl) {
        std::string msg = tpl.what();
        if (tpl.location().has_value()) {
            const auto &loc = *tpl.location();
            msg += " at " + loc.file.string() + ":" + std::to_string(loc.row) +
                   ":" + std::to_string(loc.col);
        }
        addError(msg);
        recordDiagnostic(tpl.clone());
        return nullptr;
    } catch (const SCE::parsing::XIncludeExpansionError &xie) {
        // §wire-W3: re-thrown by `PugiXMLDocument::processXInclude`
        // so the typed leaf reaches `getDiagnostics()` with its
        // `xml/xinclude-*` `code()` intact. `addError` populates the
        // legacy string surface for Q4-B coexistence, paralleling
        // the §wire-W2 TemplateError arm above.
        std::string msg = "XInclude processing failed: " + std::string(xie.what());
        if (xie.location().has_value()) {
            const auto &loc = *xie.location();
            msg += " at " + loc.file.string() + ":" + std::to_string(loc.row) +
                   ":" + std::to_string(loc.col);
        }
        addError(msg);
        recordDiagnostic(xie.clone());
        return nullptr;
    } catch (const SCE::parsing::SemanticError &se) {
        // §wire-W5 D5: SCXML semantic-validation throws (parseScxmlNode
        // top-level-script + no-states; validateModel initial-state +
        // transition-target + compound-state-initial) surface here.
        // Q4-B coexistence: legacy `addError` populates
        // `getErrorMessages()` while `recordDiagnostic` populates the
        // typed `getDiagnostics()` surface consumers dispatch on.
        addError(se.what());
        recordDiagnostic(se.clone());
        return nullptr;
    } catch (const std::exception &ex) {
        // §wire-W4 D1-C: wrap unexpected std::exception as typed
        // `ParseException` so the typed surface stays populated even
        // for non-typed throws (bad_alloc, third-party throws). Per
        // D4 α-strict, `typeid(ex).name()` is NOT included — the wire
        // detail field would emit different strings on libstdc++ /
        // libc++ / MSVC, breaking portability.
        SCE::parsing::ParseException pe(
            "Exception while parsing file: " + std::string(ex.what()));
        addError(pe.what());
        recordDiagnostic(pe.clone());
        return nullptr;
    }
}

std::shared_ptr<SCE::SCXMLModel> SCE::SCXMLParser::parseContent(const std::string &content) {
    try {
        // Initialize parsing state
        initParsing();

        SCE_LOG_INFO("Parsing SCXML content");

        // §wire-W4 D1-C: PugiXMLParser throws `ParseXmlFailed` on
        // malformed input; the caller no longer polls a nullable
        // result + `getLastError()`.
        auto xmlParser = IXMLParser::create();
        auto doc = xmlParser->parseContent(content);

        // Process XIncludes; the produced PositionMap composes into
        // the template stage. Typed throws
        // bubble to the catch arms below (§wire-W4.5 D1).
        SCE_LOG_DEBUG("Processing XIncludes");
        auto xincludePositions = doc->processXInclude();

        // Process `<sce:use>` template expansion. Each failure
        // mode raises a typed `SCE::parsing::TemplateError`
        // subtype caught below; reparse failures raise
        // `ParseXmlFailed` per §wire-W4.5 D2.
        SCE_LOG_DEBUG("Processing sce:template");
        documentPositions_ = doc->processSceTemplate(xincludePositions);

        // Parse document
        return parseAbstractDocument(doc);
    } catch (const SCE::parsing::ParseError &pe) {
        // §wire-W4 D1-C: parser-entry typed surface (mirror of
        // `parseFile`'s arm above).
        addError(pe.what());
        recordDiagnostic(pe.clone());
        return nullptr;
    } catch (const SCE::parsing::TemplateError &tpl) {
        std::string msg = tpl.what();
        if (tpl.location().has_value()) {
            const auto &loc = *tpl.location();
            msg += " at " + loc.file.string() + ":" + std::to_string(loc.row) +
                   ":" + std::to_string(loc.col);
        }
        addError(msg);
        recordDiagnostic(tpl.clone());
        return nullptr;
    } catch (const SCE::parsing::XIncludeExpansionError &xie) {
        // §wire-W3: re-thrown by `PugiXMLDocument::processXInclude`
        // so the typed leaf reaches `getDiagnostics()` with its
        // `xml/xinclude-*` `code()` intact. `addError` populates the
        // legacy string surface for Q4-B coexistence.
        std::string msg = "XInclude processing failed: " + std::string(xie.what());
        if (xie.location().has_value()) {
            const auto &loc = *xie.location();
            msg += " at " + loc.file.string() + ":" + std::to_string(loc.row) +
                   ":" + std::to_string(loc.col);
        }
        addError(msg);
        recordDiagnostic(xie.clone());
        return nullptr;
    } catch (const SCE::parsing::SemanticError &se) {
        // §wire-W5 D5: SCXML semantic-validation throws (parseScxmlNode
        // top-level-script + no-states; validateModel initial-state +
        // transition-target + compound-state-initial) surface here.
        // Q4-B coexistence: legacy `addError` populates
        // `getErrorMessages()` while `recordDiagnostic` populates the
        // typed `getDiagnostics()` surface consumers dispatch on.
        // Mirror of the parseFile arm above.
        addError(se.what());
        recordDiagnostic(se.clone());
        return nullptr;
    } catch (const std::exception &ex) {
        // §wire-W4 D1-C: wrap unexpected std::exception as typed
        // `ParseException`.
        SCE::parsing::ParseException pe(
            "Exception while parsing content: " + std::string(ex.what()));
        addError(pe.what());
        recordDiagnostic(pe.clone());
        return nullptr;
    }
}

std::shared_ptr<SCE::SCXMLModel> SCE::SCXMLParser::parseAbstractDocument(std::shared_ptr<IXMLDocument> doc) {
    // §wire-W4 D1-C: PugiXMLParser::parseFile / parseContent throw on
    // failure rather than returning nullptr, so this function's
    // callers (parseFile / parseContent above) only invoke it with a
    // valid document. The historical `if (!doc) { addError("Null
    // document"); return nullptr; }` branch became dead under
    // typed-throw and is removed (the dropped `ParseNullDocument`
    // leaf in §wire-W4 α-strict).

    // Get root element. roxmltree's Rust-side analog cannot reach
    // this case (parse rejects root-less input), so the C++ leaf
    // reuses `xml/parse` rather than introducing a new wire code.
    auto rootElement = doc->getRootElement();
    if (!rootElement) {
        throw SCE::parsing::ParseNoRootElement("No root element found");
    }

    // Check if root element is 'scxml' AND in the W3C SCXML namespace
    // (or unnamespaced — lenient on legacy fixtures per `isScxmlNamespace`).
    // Mirrors the Rust-side `XmlError::WrongRootElement` producer in
    // `sce-build/src/parser.rs::SCXMLParser::parse_impl` — both engines
    // surface the same `xml/wrong-root-element` wire code so consumers
    // dispatch identically across pipelines. The namespace gate
    // additionally rejects `<framework:scxml>` roots that would
    // otherwise be matched by local-name-only dispatch.
    if (!ParsingCommon::matchNodeName(rootElement->getName(), "scxml") ||
        !ParsingCommon::isScxmlNamespace(rootElement)) {
        throw SCE::parsing::ParseWrongRootElement(
            "Root element is not 'scxml', found: " + rootElement->getName());
    }

    SCE_LOG_INFO("Valid SCXML document found, parsing structure");

    // Create SCXML model
    auto model = std::make_shared<SCXMLModel>();

    // Parse SCXML node using IXMLElement interface
    bool result = parseScxmlNode(rootElement, model);
    if (result) {
        SCE_LOG_INFO("SCXML document parsed successfully");

        // Validate model
        if (validateModel(model)) {
            return model;
        } else {
            SCE_LOG_ERROR("SCXML model validation failed");
            return nullptr;
        }
    } else {
        SCE_LOG_ERROR("Failed to parse SCXML document");
        return nullptr;
    }
}

bool SCE::SCXMLParser::parseScxmlNode(const std::shared_ptr<IXMLElement> &scxmlNode,
                                      std::shared_ptr<SCXMLModel> model) {
    // §wire-W5 Stage E: removed `if (!scxmlNode || !model)` precondition
    // guard. Callers (parseAbstractDocument) construct `model` with
    // `std::make_shared` (never null) and reach this point only after
    // `getRootElement()` returned a non-null handle and the
    // `ParseWrongRootElement` check passed (W4 D1-C). The guard was
    // dead post-W4 — revealed by W5 site inventory.

    SCE_LOG_DEBUG("Parsing SCXML root node");

    // Create and initialize SCXMLContext
    SCXMLContext context;

    // Parse basic attributes
    if (scxmlNode->hasAttribute("name")) {
        std::string name = scxmlNode->getAttribute("name");
        model->setName(name);
        SCE_LOG_DEBUG("Name: {}", name);
    }

    if (scxmlNode->hasAttribute("initial")) {
        std::string initial = scxmlNode->getAttribute("initial");
        model->setInitialState(initial);
        SCE_LOG_DEBUG("Initial state: {}", initial);
    }

    if (scxmlNode->hasAttribute("datamodel")) {
        std::string datamodelType = scxmlNode->getAttribute("datamodel");
        model->setDatamodel(datamodelType);
        context.setDatamodelType(datamodelType);
        // §scxml-5.5 + Appendix B.2.2: `<donedata><content>text</content>`
        // semantics are datamodel-dependent. Propagate the root attribute
        // so `DoneDataParser` can pick Expression vs Literal per document.
        if (doneDataParser_) {
            doneDataParser_->setDatamodelType(datamodelType);
        }
        SCE_LOG_DEBUG("Datamodel: {}", datamodelType);
    }

    if (scxmlNode->hasAttribute("binding")) {
        std::string binding = scxmlNode->getAttribute("binding");
        model->setBinding(binding);
        context.setBinding(binding);
        SCE_LOG_DEBUG("Binding mode: {}", binding);
    }

    // Parse context properties
    parseContextProperties(scxmlNode, model);

    // Parse dependency injection points
    parseInjectPoints(scxmlNode, model);

    // Parse guard conditions
    SCE_LOG_DEBUG("Parsing guards");
    auto guards = guardParser_->parseAllGuards(scxmlNode);
    for (const auto &guard : guards) {
        model->addGuard(guard);

        if (!guard->getCondition().empty() && !guard->getTargetState().empty()) {
            SCE_LOG_DEBUG("Added guard: {} with condition: {} targeting state: {}", guard->getId(), guard->getCondition(),
                      guard->getTargetState());
        } else if (!guard->getCondition().empty()) {
            SCE_LOG_DEBUG("Added guard: {} with condition: {}", guard->getId(), guard->getCondition());
        } else if (!guard->getTargetState().empty()) {
            SCE_LOG_DEBUG("Added guard: {} targeting state: {}", guard->getId(), guard->getTargetState());
        } else {
            SCE_LOG_DEBUG("Added guard: {}", guard->getId());
        }
    }

    // Parse top-level datamodel
    SCE_LOG_DEBUG("Parsing root datamodel");
    auto datamodelNode = SCE::ParsingCommon::findFirstChildElement(scxmlNode, "datamodel");
    if (datamodelNode) {
        auto dataItems = dataModelParser_->parseDataModelNode(datamodelNode, context);
        for (const auto &item : dataItems) {
            model->addDataModelItem(item);
            SCE_LOG_DEBUG("Added data model item: {}", item->getId());
        }
    }

    addSystemVariables(model);

    // §scxml-5.8: Parse top-level <script> elements
    auto scriptElements = SCE::ParsingCommon::findChildElements(scxmlNode, "script");
    if (!scriptElements.empty()) {
        SCE_LOG_DEBUG("Parsing {} root script element(s) (W3C SCXML 5.8)", scriptElements.size());
        size_t parsedCount = 0;

        for (size_t i = 0; i < scriptElements.size(); ++i) {
            auto scriptAction = actionParser_->parseActionNode(scriptElements[i]);
            if (scriptAction) {
                model->addTopLevelScript(scriptAction);
                parsedCount++;
                SCE_LOG_DEBUG("Added top-level script #{} for document load time execution (W3C SCXML 5.8)", i + 1);
            } else {
                std::string errorDetail = "Top-level script element #" + std::to_string(i + 1) + " cannot be loaded";

                std::optional<std::string> srcOpt;
                if (scriptElements[i]->hasAttribute("src")) {
                    std::string srcValue = scriptElements[i]->getAttribute("src");
                    errorDetail += " (src: \"" + Log::sanitize(srcValue) + "\")";
                    srcOpt = std::move(srcValue);
                }
                errorDetail += " - document rejected per W3C SCXML 5.8";

                SCE_LOG_ERROR("Failed to parse top-level script element #{} (W3C SCXML 5.8)", i + 1);
                // §wire-W5 D5: typed-throw replaces addError + return-false
                // so the parser-entry catch arm can record both the legacy
                // string and the typed Diagnostic in one site.
                throw SCE::parsing::SemanticTopLevelScriptUnloaded(
                    std::move(errorDetail),
                    /*index=*/std::optional<std::size_t>{i + 1},
                    /*src=*/std::move(srcOpt));
            }
        }

        SCE_LOG_DEBUG("Successfully parsed {}/{} top-level script(s) (W3C SCXML 5.8)", parsedCount, scriptElements.size());
    }

    // Parse states
    SCE_LOG_DEBUG("Looking for root state nodes");

    std::vector<std::shared_ptr<IXMLElement>> rootStateElements;
    auto stateElements = SCE::ParsingCommon::findChildElements(scxmlNode, "state");
    rootStateElements.insert(rootStateElements.end(), stateElements.begin(), stateElements.end());

    auto parallelElements = SCE::ParsingCommon::findChildElements(scxmlNode, "parallel");
    rootStateElements.insert(rootStateElements.end(), parallelElements.begin(), parallelElements.end());

    auto finalElements = SCE::ParsingCommon::findChildElements(scxmlNode, "final");
    rootStateElements.insert(rootStateElements.end(), finalElements.begin(), finalElements.end());

    if (rootStateElements.empty()) {
        // §wire-W5 D5: typed-throw — folded onto `validation/empty-collection`
        // per W4 D4 fold (concept identity with forge "kind requires at
        // least one X").
        throw SCE::parsing::SemanticNoStates(
            "No state nodes found in SCXML document");
    }

    SCE_LOG_INFO("Found {} root state nodes", rootStateElements.size());

    for (const auto &stateElement : rootStateElements) {
        SCE_LOG_INFO("Parsing root state");
        auto state = stateNodeParser_->parseStateNode(stateElement, nullptr, context);
        if (state) {
            model->addState(state);

            if (!model->getRootState()) {
                model->setRootState(state);
            }

            SCE_LOG_INFO("Root state parsed: {}", state->getId());
        } else {
            addError("Failed to parse a root state");
            return false;
        }
    }

    return true;
}

void SCE::SCXMLParser::parseContextProperties(const std::shared_ptr<IXMLElement> &scxmlNode,
                                              std::shared_ptr<SCXMLModel> model) {
    if (!scxmlNode || !model) {
        return;
    }

    SCE_LOG_DEBUG("Parsing context properties");

    auto ctxProps = SCE::ParsingCommon::findChildElements(scxmlNode, "property");

    for (const auto &propElement : ctxProps) {
        if (propElement->hasAttribute("name") && propElement->hasAttribute("type")) {
            std::string name = propElement->getAttribute("name");
            std::string type = propElement->getAttribute("type");
            model->addContextProperty(name, type);
            SCE_LOG_DEBUG("Added property: {} ({})", name, type);
        } else {
            addWarning("Property node missing required attributes");
        }
    }

    SCE_LOG_DEBUG("Found {} context properties", model->getContextProperties().size());
}

void SCE::SCXMLParser::parseInjectPoints(const std::shared_ptr<IXMLElement> &scxmlNode,
                                         std::shared_ptr<SCXMLModel> model) {
    if (!scxmlNode || !model) {
        return;
    }

    SCE_LOG_DEBUG("Parsing injection points");

    std::vector<std::string> injectNodeNames = {"inject-point", "inject_point", "injectpoint", "inject", "dependency"};

    bool foundInjectPoints = false;
    for (const auto &nodeName : injectNodeNames) {
        auto injectElements = SCE::ParsingCommon::findChildElements(scxmlNode, nodeName);

        for (const auto &injectElement : injectElements) {
            std::string name, type;

            if (injectElement->hasAttribute("name")) {
                name = injectElement->getAttribute("name");
            } else if (injectElement->hasAttribute("id")) {
                name = injectElement->getAttribute("id");
            }

            if (injectElement->hasAttribute("type")) {
                type = injectElement->getAttribute("type");
            } else if (injectElement->hasAttribute("class")) {
                type = injectElement->getAttribute("class");
            }

            if (!name.empty() && !type.empty()) {
                model->addInjectPoint(name, type);
                SCE_LOG_DEBUG("Added inject point: {} ({})", name, type);
                foundInjectPoints = true;
            } else {
                addWarning("Inject point node missing required attributes");
            }
        }

        if (foundInjectPoints) {
            break;
        }
    }

    SCE_LOG_DEBUG("Found {} injection points", model->getInjectPoints().size());
}

bool SCE::SCXMLParser::hasErrors() const {
    return !errorMessages_.empty();
}

const std::vector<std::string> &SCE::SCXMLParser::getErrorMessages() const {
    return errorMessages_;
}

const std::vector<std::string> &SCE::SCXMLParser::getWarningMessages() const {
    return warningMessages_;
}

const std::vector<std::unique_ptr<SCE::parsing::Diagnostic>> &
SCE::SCXMLParser::getDiagnostics() const noexcept {
    return diagnostics_;
}

void SCE::SCXMLParser::initParsing() {
    errorMessages_.clear();
    warningMessages_.clear();
    diagnostics_.clear();
}

void SCE::SCXMLParser::addError(const std::string &message) {
    SCE_LOG_ERROR("SCXMLParser - {}", message);
    errorMessages_.push_back(message);
}

void SCE::SCXMLParser::addWarning(const std::string &message) {
    SCE_LOG_WARN("SCXMLParser - {}", message);
    warningMessages_.push_back(message);
}

void SCE::SCXMLParser::recordDiagnostic(
    std::unique_ptr<SCE::parsing::Diagnostic> diag) {
    if (diag) {
        diagnostics_.push_back(std::move(diag));
    }
}

bool SCE::SCXMLParser::validateModel(std::shared_ptr<SCXMLModel> model) {
    // §wire-W5 Stage E: removed `if (!model)` and `if
    // (!model->getRootState())` guards. The first is unreachable
    // because callers construct `model` with `std::make_shared` and
    // only invoke `validateModel` from the success path of
    // `parseScxmlNode`. The second is unreachable because
    // `parseScxmlNode`'s "No state nodes found" throw fires before
    // any successful state parse — a model that reached
    // `validateModel` always has a root state. Both guards were dead
    // post-W4 typed-throw — revealed by W5 site inventory.

    SCE_LOG_INFO("Validating SCXML model");

    // Snapshot all declared state ids once for the typed-throw
    // payload's `available` list. Used by `SemanticInitialStateUnknown`
    // and `SemanticTransitionTargetUnknown` so consumers receive a
    // structured `fix.candidates` list (§wire-W5 D2 fold of forge
    // `validation/invalid-reference`).
    std::vector<std::string> availableStateIds;
    availableStateIds.reserve(model->getAllStates().size());
    for (const auto &s : model->getAllStates()) {
        availableStateIds.push_back(s->getId());
    }

    // 2. Validate initial states (§scxml-3.3 — root-level initial)
    const auto &initialStates = model->getInitialStates();
    if (!initialStates.empty()) {
        for (const auto &initialStateId : initialStates) {
            if (!model->findStateById(initialStateId)) {
                // §wire-W5 D5 typed-throw — fail-fast on the first bad
                // id (W4 D1-C invariant: a single semantic error
                // terminates the parse, paralleling the parser-entry
                // ParseError catch arm).
                throw SCE::parsing::SemanticInitialStateUnknown(
                    "Initial state '" + initialStateId + "' not found",
                    initialStateId,
                    SCE::parsing::SemanticInitialStateUnknown::Scope::DocumentRoot,
                    /*parent_id=*/std::string{},
                    availableStateIds);
            }
        }
    }

    // 3. Validate state relationships
    //
    // §wire-W5 Stage E removed the parent/children consistency guard
    // (previously: "State '<X>' has parent '<Y>' but is not in
    // parent's children list"). The check was a defensive guard for
    // an internal model-construction invariant — `StateNode::addChild`
    // always pairs `setParent` with `getChildren().push_back`, so a
    // child whose `getParent()` points to a state that doesn't list
    // it in `getChildren()` could only arise from a model-mutation
    // bug AFTER parse-time. Such a bug is a programming error, not a
    // semantic-validation failure; an assertion or invariant test
    // would catch it more honestly. The guard had no Rust producer
    // (Rust constructs the model atomically — `parser.rs::parse_states`
    // attaches state to parent in the same operation) and no
    // wire-code mapping. Removed per `feedback_silently_broken_hooks.md`.
    for (const auto &state : model->getAllStates()) {
        // Validate transition target states (§scxml-3.5)
        for (const auto &transition : state->getTransitions()) {
            const auto &targets = transition->getTargets();
            for (const auto &target : targets) {
                if (!target.empty() && !model->findStateById(target)) {
                    // §wire-W5 D5 typed-throw — folded onto
                    // `validation/invalid-reference` (concept identity
                    // with forge `ValidationError::InvalidReference`).
                    throw SCE::parsing::SemanticTransitionTargetUnknown(
                        "Transition in state '" + state->getId() +
                            "' references non-existent target state '" +
                            target + "'",
                        state->getId(),
                        target,
                        availableStateIds);
                }
            }
        }

        // §scxml-3.3: Validate compound-state initial state(s)
        if (!state->getInitialState().empty() && state->getChildren().size() > 0) {
            std::istringstream iss(state->getInitialState());
            std::string initialStateId;
            while (iss >> initialStateId) {
                if (!model->findStateById(initialStateId)) {
                    // Same wire code as root-level (§wire-W5 D2 — one
                    // C++ leaf covers both scopes), payload `Scope`
                    // discriminates root vs compound for in-process
                    // typed dispatch.
                    throw SCE::parsing::SemanticInitialStateUnknown(
                        "State '" + state->getId() +
                            "' references non-existent initial state '" +
                            initialStateId + "'",
                        initialStateId,
                        SCE::parsing::SemanticInitialStateUnknown::Scope::CompoundState,
                        /*parent_id=*/state->getId(),
                        availableStateIds);
                }
            }
        }
    }

    // 4. Validate guards (warning-only — does not throw)
    for (const auto &guard : model->getGuards()) {
        if (!GuardUtils::isConditionExpression(guard->getTargetState()) &&
            !model->findStateById(guard->getTargetState())) {
            addWarning("Guard '" + guard->getId() + "' references non-existent target state '" +
                       guard->getTargetState() + "'");
        }
    }

    SCE_LOG_INFO("Model validation successful");
    return true;
}

void SCE::SCXMLParser::addSystemVariables(std::shared_ptr<SCXMLModel> model) {
    if (!model) {
        SCE_LOG_WARN("Null model");
        return;
    }

    SCE_LOG_DEBUG("Adding system variables to data model");

    std::string datamodelType = model->getDatamodel();
    if (datamodelType.empty() || datamodelType == "null") {
        SCE_LOG_DEBUG("Skipping system variables for null datamodel");
        return;
    }

    // Add _name system variable
    auto nameItem = nodeFactory_->createDataModelItem("_name", datamodelType);
    nameItem->setType(datamodelType);
    if (datamodelType == "ecmascript") {
        nameItem->setExpr("''");
    } else if (datamodelType == "xpath") {
        nameItem->setContent("''");
    }
    model->addSystemVariable(nameItem);
    SCE_LOG_DEBUG("Added system variable: _name");

    // Add _sessionid system variable
    auto sessionIdItem = nodeFactory_->createDataModelItem("_sessionid", datamodelType);
    sessionIdItem->setType(datamodelType);
    if (datamodelType == "ecmascript") {
        sessionIdItem->setExpr("''");
    } else if (datamodelType == "xpath") {
        sessionIdItem->setContent("''");
    }
    model->addSystemVariable(sessionIdItem);
    SCE_LOG_DEBUG("Added system variable: _sessionid");

    // Add _ioprocessors system variable
    auto ioProcessorsItem = nodeFactory_->createDataModelItem("_ioprocessors", datamodelType);
    ioProcessorsItem->setType(datamodelType);
    if (datamodelType == "ecmascript") {
        ioProcessorsItem->setExpr("{}");
    } else if (datamodelType == "xpath") {
        ioProcessorsItem->setContent("<ioprocessors/>");
    }
    model->addSystemVariable(ioProcessorsItem);
    SCE_LOG_DEBUG("Added system variable: _ioprocessors");

    // §scxml-5.10: _event is bound lazily on first event
    SCE_LOG_DEBUG("Skipping _event initialization per W3C SCXML 5.10 (bound only after first event)");
}
