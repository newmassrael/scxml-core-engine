#include "parsing/SCXMLParser.h"
#include "GuardUtils.h"
#include "common/LogUtils.h"
#include "common/Logger.h"
#include "parsing/IXMLParser.h"
#include "parsing/ParsingCommon.h"

#ifndef __EMSCRIPTEN__
#include "parsing/LibXMLParser.h"
#else
#include "parsing/PugiXMLParser.h"
#endif

#include <algorithm>
#include <filesystem>

RSM::SCXMLParser::SCXMLParser(std::shared_ptr<RSM::NodeFactory> nodeFactory,
                              std::shared_ptr<RSM::IXIncludeProcessor> xincludeProcessor)
    : nodeFactory_(nodeFactory) {
    LOG_DEBUG("Creating SCXML parser");

    // Initialize specialized parsers
    stateNodeParser_ = std::make_shared<RSM::StateNodeParser>(nodeFactory_);
    transitionParser_ = std::make_shared<RSM::TransitionParser>(nodeFactory_);
    actionParser_ = std::make_shared<RSM::ActionParser>(nodeFactory_);
    guardParser_ = std::make_shared<RSM::GuardParser>(nodeFactory_);
    dataModelParser_ = std::make_shared<RSM::DataModelParser>(nodeFactory_);
    invokeParser_ = std::make_shared<RSM::InvokeParser>(nodeFactory_);
    doneDataParser_ = std::make_shared<RSM::DoneDataParser>(nodeFactory_);

    // Connect related parsers
    stateNodeParser_->setRelatedParsers(transitionParser_, actionParser_, dataModelParser_, invokeParser_,
                                        doneDataParser_);

    // Set ActionParser for TransitionParser
    transitionParser_->setActionParser(actionParser_);

    // Set up XInclude processor
    if (xincludeProcessor) {
        xincludeProcessor_ = xincludeProcessor;
    } else {
        xincludeProcessor_ = std::make_shared<RSM::XIncludeProcessor>();
    }
}

RSM::SCXMLParser::~SCXMLParser() {
    LOG_DEBUG("Destroying SCXML parser");
}

std::shared_ptr<RSM::SCXMLModel> RSM::SCXMLParser::parseFile(const std::string &filename) {
    try {
        // Initialize parsing state
        initParsing();

        // Check if file exists
        if (!std::filesystem::exists(filename)) {
            addError("File not found: " + filename);
            return nullptr;
        }

        LOG_INFO("Parsing SCXML file: {}", filename);

        // W3C SCXML 5.8: Set base path for external script resolution
        std::filesystem::path scxmlPath(filename);
        std::string basePath = scxmlPath.parent_path().string();
        actionParser_->setScxmlBasePath(basePath);
        LOG_DEBUG("Set SCXML base path for external script resolution: {}", basePath);

        // Parse file using platform-specific XML parser
        auto xmlParser = IXMLParser::create();
        auto doc = xmlParser->parseFile(filename);

        if (!doc || !doc->isValid()) {
            addError("Failed to parse XML file: " + xmlParser->getLastError());
            return nullptr;
        }

        // Process XIncludes
        LOG_DEBUG("Processing XIncludes");
        doc->processXInclude();

        // Parse document
        return parseAbstractDocument(doc);
    } catch (const std::exception &ex) {
        addError("Exception while parsing file: " + std::string(ex.what()));
        return nullptr;
    }
}

std::shared_ptr<RSM::SCXMLModel> RSM::SCXMLParser::parseContent(const std::string &content) {
    try {
        // Initialize parsing state
        initParsing();

        LOG_INFO("Parsing SCXML content");

        // Parse from string using platform-specific XML parser
        auto xmlParser = IXMLParser::create();
        auto doc = xmlParser->parseContent(content);

        if (!doc || !doc->isValid()) {
            addError("Failed to parse XML content: " + xmlParser->getLastError());
            return nullptr;
        }

        // Process XIncludes
        LOG_DEBUG("Processing XIncludes");
        doc->processXInclude();

        // Parse document
        return parseAbstractDocument(doc);
    } catch (const std::exception &ex) {
        addError("Exception while parsing content: " + std::string(ex.what()));
        return nullptr;
    }
}

std::shared_ptr<RSM::SCXMLModel> RSM::SCXMLParser::parseAbstractDocument(std::shared_ptr<IXMLDocument> doc) {
    if (!doc) {
        addError("Null document");
        return nullptr;
    }

    // Get root element
    auto rootElement = doc->getRootElement();
    if (!rootElement) {
        addError("No root element found");
        return nullptr;
    }

    // Check if root element is 'scxml'
    if (!ParsingCommon::matchNodeName(rootElement->getName(), "scxml")) {
        addError("Root element is not 'scxml', found: " + rootElement->getName());
        return nullptr;
    }

    LOG_INFO("Valid SCXML document found, parsing structure");

    // Create SCXML model
    auto model = std::make_shared<SCXMLModel>();

    // Parse SCXML node using IXMLElement interface
    bool result = parseScxmlNode(rootElement, model);
    if (result) {
        LOG_INFO("SCXML document parsed successfully");

        // Validate model
        if (validateModel(model)) {
            return model;
        } else {
            LOG_ERROR("SCXML model validation failed");
            return nullptr;
        }
    } else {
        LOG_ERROR("Failed to parse SCXML document");
        return nullptr;
    }
}

bool RSM::SCXMLParser::parseScxmlNode(const std::shared_ptr<IXMLElement> &scxmlNode,
                                      std::shared_ptr<SCXMLModel> model) {
    if (!scxmlNode || !model) {
        addError("Null scxml node or model");
        return false;
    }

    LOG_DEBUG("Parsing SCXML root node");

    // Create and initialize SCXMLContext
    SCXMLContext context;

    // Parse basic attributes
    if (scxmlNode->hasAttribute("name")) {
        std::string name = scxmlNode->getAttribute("name");
        model->setName(name);
        LOG_DEBUG("Name: {}", name);
    }

    if (scxmlNode->hasAttribute("initial")) {
        std::string initial = scxmlNode->getAttribute("initial");
        model->setInitialState(initial);
        LOG_DEBUG("Initial state: {}", initial);
    }

    if (scxmlNode->hasAttribute("datamodel")) {
        std::string datamodelType = scxmlNode->getAttribute("datamodel");
        model->setDatamodel(datamodelType);
        context.setDatamodelType(datamodelType);
        LOG_DEBUG("Datamodel: {}", datamodelType);
    }

    if (scxmlNode->hasAttribute("binding")) {
        std::string binding = scxmlNode->getAttribute("binding");
        model->setBinding(binding);
        context.setBinding(binding);
        LOG_DEBUG("Binding mode: {}", binding);
    }

    // Parse context properties
    parseContextProperties(scxmlNode, model);

    // Parse dependency injection points
    parseInjectPoints(scxmlNode, model);

    // Parse guard conditions
    LOG_DEBUG("Parsing guards");
    auto guards = guardParser_->parseAllGuards(scxmlNode);
    for (const auto &guard : guards) {
        model->addGuard(guard);

        if (!guard->getCondition().empty() && !guard->getTargetState().empty()) {
            LOG_DEBUG("Added guard: {} with condition: {} targeting state: {}", guard->getId(), guard->getCondition(),
                      guard->getTargetState());
        } else if (!guard->getCondition().empty()) {
            LOG_DEBUG("Added guard: {} with condition: {}", guard->getId(), guard->getCondition());
        } else if (!guard->getTargetState().empty()) {
            LOG_DEBUG("Added guard: {} targeting state: {}", guard->getId(), guard->getTargetState());
        } else {
            LOG_DEBUG("Added guard: {}", guard->getId());
        }
    }

    // Parse top-level datamodel
    LOG_DEBUG("Parsing root datamodel");
    auto datamodelNode = RSM::ParsingCommon::findFirstChildElement(scxmlNode, "datamodel");
    if (datamodelNode) {
        auto dataItems = dataModelParser_->parseDataModelNode(datamodelNode, context);
        for (const auto &item : dataItems) {
            model->addDataModelItem(item);
            LOG_DEBUG("Added data model item: {}", item->getId());
        }
    }

    addSystemVariables(model);

    // W3C SCXML 5.8: Parse top-level <script> elements
    auto scriptElements = RSM::ParsingCommon::findChildElements(scxmlNode, "script");
    if (!scriptElements.empty()) {
        LOG_DEBUG("Parsing {} root script element(s) (W3C SCXML 5.8)", scriptElements.size());
        size_t parsedCount = 0;

        for (size_t i = 0; i < scriptElements.size(); ++i) {
            auto scriptAction = actionParser_->parseActionNode(scriptElements[i]);
            if (scriptAction) {
                model->addTopLevelScript(scriptAction);
                parsedCount++;
                LOG_DEBUG("Added top-level script #{} for document load time execution (W3C SCXML 5.8)", i + 1);
            } else {
                std::string errorDetail = "Top-level script element #" + std::to_string(i + 1) + " cannot be loaded";

                if (scriptElements[i]->hasAttribute("src")) {
                    std::string srcValue = scriptElements[i]->getAttribute("src");
                    errorDetail += " (src: \"" + Log::sanitize(srcValue) + "\")";
                }
                errorDetail += " - document rejected per W3C SCXML 5.8";

                LOG_ERROR("Failed to parse top-level script element #{} (W3C SCXML 5.8)", i + 1);
                addError(errorDetail);
                return false;
            }
        }

        LOG_DEBUG("Successfully parsed {}/{} top-level script(s) (W3C SCXML 5.8)", parsedCount, scriptElements.size());
    }

    // Parse states
    LOG_DEBUG("Looking for root state nodes");

    std::vector<std::shared_ptr<IXMLElement>> rootStateElements;
    auto stateElements = RSM::ParsingCommon::findChildElements(scxmlNode, "state");
    rootStateElements.insert(rootStateElements.end(), stateElements.begin(), stateElements.end());

    auto parallelElements = RSM::ParsingCommon::findChildElements(scxmlNode, "parallel");
    rootStateElements.insert(rootStateElements.end(), parallelElements.begin(), parallelElements.end());

    auto finalElements = RSM::ParsingCommon::findChildElements(scxmlNode, "final");
    rootStateElements.insert(rootStateElements.end(), finalElements.begin(), finalElements.end());

    if (rootStateElements.empty()) {
        addError("No state nodes found in SCXML document");
        return false;
    }

    LOG_INFO("Found {} root state nodes", rootStateElements.size());

    for (const auto &stateElement : rootStateElements) {
        LOG_INFO("Parsing root state");
        auto state = stateNodeParser_->parseStateNode(stateElement, nullptr, context);
        if (state) {
            model->addState(state);

            if (!model->getRootState()) {
                model->setRootState(state);
            }

            LOG_INFO("Root state parsed: {}", state->getId());
        } else {
            addError("Failed to parse a root state");
            return false;
        }
    }

    return true;
}

void RSM::SCXMLParser::parseContextProperties(const std::shared_ptr<IXMLElement> &scxmlNode,
                                              std::shared_ptr<SCXMLModel> model) {
    if (!scxmlNode || !model) {
        return;
    }

    LOG_DEBUG("Parsing context properties");

    auto ctxProps = RSM::ParsingCommon::findChildElements(scxmlNode, "property");

    for (const auto &propElement : ctxProps) {
        if (propElement->hasAttribute("name") && propElement->hasAttribute("type")) {
            std::string name = propElement->getAttribute("name");
            std::string type = propElement->getAttribute("type");
            model->addContextProperty(name, type);
            LOG_DEBUG("Added property: {} ({})", name, type);
        } else {
            addWarning("Property node missing required attributes");
        }
    }

    LOG_DEBUG("Found {} context properties", model->getContextProperties().size());
}

void RSM::SCXMLParser::parseInjectPoints(const std::shared_ptr<IXMLElement> &scxmlNode,
                                         std::shared_ptr<SCXMLModel> model) {
    if (!scxmlNode || !model) {
        return;
    }

    LOG_DEBUG("Parsing injection points");

    std::vector<std::string> injectNodeNames = {"inject-point", "inject_point", "injectpoint", "inject", "dependency"};

    bool foundInjectPoints = false;
    for (const auto &nodeName : injectNodeNames) {
        auto injectElements = RSM::ParsingCommon::findChildElements(scxmlNode, nodeName);

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
                LOG_DEBUG("Added inject point: {} ({})", name, type);
                foundInjectPoints = true;
            } else {
                addWarning("Inject point node missing required attributes");
            }
        }

        if (foundInjectPoints) {
            break;
        }
    }

    LOG_DEBUG("Found {} injection points", model->getInjectPoints().size());
}

bool RSM::SCXMLParser::hasErrors() const {
    return !errorMessages_.empty();
}

const std::vector<std::string> &RSM::SCXMLParser::getErrorMessages() const {
    return errorMessages_;
}

const std::vector<std::string> &RSM::SCXMLParser::getWarningMessages() const {
    return warningMessages_;
}

void RSM::SCXMLParser::initParsing() {
    errorMessages_.clear();
    warningMessages_.clear();
}

void RSM::SCXMLParser::addError(const std::string &message) {
    LOG_ERROR("SCXMLParser - {}", message);
    errorMessages_.push_back(message);
}

void RSM::SCXMLParser::addWarning(const std::string &message) {
    LOG_WARN("SCXMLParser - {}", message);
    warningMessages_.push_back(message);
}

bool RSM::SCXMLParser::validateModel(std::shared_ptr<SCXMLModel> model) {
    if (!model) {
        addError("Null model in validation");
        return false;
    }

    LOG_INFO("Validating SCXML model");

    bool isValid = true;

    // 1. Verify root state
    if (!model->getRootState()) {
        addError("Model has no root state");
        return false;
    }

    // 2. Validate initial states
    const auto &initialStates = model->getInitialStates();
    if (!initialStates.empty()) {
        for (const auto &initialStateId : initialStates) {
            if (!model->findStateById(initialStateId)) {
                addError("Initial state '" + initialStateId + "' not found");
                isValid = false;
            }
        }
    }

    // 3. Validate state relationships
    for (const auto &state : model->getAllStates()) {
        // Validate parent state
        auto parent = state->getParent();
        if (parent) {
            bool isChild = false;
            for (const auto &child : parent->getChildren()) {
                if (child.get() == state.get()) {
                    isChild = true;
                    break;
                }
            }

            if (!isChild) {
                addError("State '" + state->getId() + "' has parent '" + parent->getId() +
                         "' but is not in parent's children list");
                isValid = false;
            }
        }

        // Validate transition target states
        for (const auto &transition : state->getTransitions()) {
            const auto &targets = transition->getTargets();
            for (const auto &target : targets) {
                if (!target.empty() && !model->findStateById(target)) {
                    addError("Transition in state '" + state->getId() + "' references non-existent target state '" +
                             target + "'");
                    isValid = false;
                }
            }
        }

        // W3C SCXML 3.3: Validate initial state(s)
        if (!state->getInitialState().empty() && state->getChildren().size() > 0) {
            std::istringstream iss(state->getInitialState());
            std::string initialStateId;
            bool allInitialStatesFound = true;

            while (iss >> initialStateId) {
                if (!model->findStateById(initialStateId)) {
                    addError("State '" + state->getId() + "' references non-existent initial state '" + initialStateId +
                             "'");
                    allInitialStatesFound = false;
                }
            }

            if (!allInitialStatesFound) {
                isValid = false;
            }
        }
    }

    // 4. Validate guards
    for (const auto &guard : model->getGuards()) {
        if (!GuardUtils::isConditionExpression(guard->getTargetState()) &&
            !model->findStateById(guard->getTargetState())) {
            addWarning("Guard '" + guard->getId() + "' references non-existent target state '" +
                       guard->getTargetState() + "'");
        }
    }

    if (isValid) {
        LOG_INFO("Model validation successful");
    } else {
        LOG_INFO("Model validation completed with errors");
    }

    return isValid;
}

void RSM::SCXMLParser::addSystemVariables(std::shared_ptr<SCXMLModel> model) {
    if (!model) {
        LOG_WARN("Null model");
        return;
    }

    LOG_DEBUG("Adding system variables to data model");

    std::string datamodelType = model->getDatamodel();
    if (datamodelType.empty() || datamodelType == "null") {
        LOG_DEBUG("Skipping system variables for null datamodel");
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
    LOG_DEBUG("Added system variable: _name");

    // Add _sessionid system variable
    auto sessionIdItem = nodeFactory_->createDataModelItem("_sessionid", datamodelType);
    sessionIdItem->setType(datamodelType);
    if (datamodelType == "ecmascript") {
        sessionIdItem->setExpr("''");
    } else if (datamodelType == "xpath") {
        sessionIdItem->setContent("''");
    }
    model->addSystemVariable(sessionIdItem);
    LOG_DEBUG("Added system variable: _sessionid");

    // Add _ioprocessors system variable
    auto ioProcessorsItem = nodeFactory_->createDataModelItem("_ioprocessors", datamodelType);
    ioProcessorsItem->setType(datamodelType);
    if (datamodelType == "ecmascript") {
        ioProcessorsItem->setExpr("{}");
    } else if (datamodelType == "xpath") {
        ioProcessorsItem->setContent("<ioprocessors/>");
    }
    model->addSystemVariable(ioProcessorsItem);
    LOG_DEBUG("Added system variable: _ioprocessors");

    // W3C SCXML 5.10: _event is bound lazily on first event
    LOG_DEBUG("Skipping _event initialization per W3C SCXML 5.10 (bound only after first event)");
}
