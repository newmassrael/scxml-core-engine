// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Execution Controller for SCXML Interactive Visualizer
 *
 * Manages step-by-step execution control and UI updates for:
 * - State diagram highlighting
 * - Event queue visualization
 * - Data model inspection
 * - Execution log
 */

// UI Timing Constants
const FOCUS_HIGHLIGHT_DURATION = 2000; // ms - Duration for state focus animation
const PANEL_HIGHLIGHT_DURATION = 3000; // ms - Duration for panel highlight animation


class ExecutionController {
    constructor(wasmRunner, visualizer, availableEvents = [], visualizerManager = null) {
        this.runner = wasmRunner;
        this.visualizer = visualizer;
        this.currentStep = 0;
        this.maxSteps = 0;
        this.availableEvents = availableEvents;
        this.visualizerManager = visualizerManager;

        // Transition history for time-travel debugging
        this.transitionHistory = []; // transitionHistory[step] = transition executed at that step

        // Scheduled events real-time update
        this.scheduledEventsTimer = null;

        // Navigation state for single-window sub-SCXML browsing (W3C SCXML 6.3)
        this.navigationStack = []; // Stack of {machineId, label, structure, visualizer, subSCXMLs}
        this.currentMachine = {
            id: 'root',
            label: 'Parent',
            structure: this.runner.getSCXMLStructure(),  // Get root SCXML structure from runner
            visualizer: this.visualizer,
            subSCXMLs: []  // [{stateId, childStructure, invokeSrc}, ...]
        };

        // Cache DOM elements for performance
        this.elements = {
            stepCounter: document.getElementById('step-counter'),
            eventQueuePanel: document.getElementById('event-queue-panel'),
            dataModelPanel: document.getElementById('data-model-panel'),
            stateActionsPanel: document.getElementById('state-actions-panel'),
            transitionInfoPanel: document.getElementById('transition-detail-panel'),
            logPanel: document.getElementById('log-panel'),
            singleViewContainer: document.getElementById('single-view-container'),
            btnStepBack: document.getElementById('btn-step-back'),
            btnStepForward: document.getElementById('btn-step-forward'),
            btnReset: document.getElementById('btn-reset'),
            eventButtonsContainer: document.getElementById('event-buttons-container')
        };

        // Initialize helper modules (must be before using them)
        this.formatters = new Formatters(this);
        this.uiUpdater = new UIUpdater(this);
        this.controlHandler = new ControlHandler(this);
        this.breadcrumbManager = new BreadcrumbManager(this);

        this.setupControls();
        this.setupEventButtons();
        this.loadTestMetadata();

        // Initialize: Update state to reflect actual machine state (may already be in final state due to eventless transitions)
        this.initializeState();
    }

    destroy() {
        // Stop scheduled events timer
        this.stopScheduledEventsTimer();

        // Remove keyboard event listener
        if (this.keyboardHandler) {
            document.removeEventListener('keydown', this.keyboardHandler);
            this.keyboardHandler = null;
        }

        // Remove transition click event listener
        if (this.transitionClickHandler) {
            document.removeEventListener('transition-click', this.transitionClickHandler);
            this.transitionClickHandler = null;
        }

        // Remove state navigate event listener
        if (this.stateNavigateHandler) {
            document.removeEventListener('state-navigate', this.stateNavigateHandler);
            this.stateNavigateHandler = null;
        }

        // Clear references
        this.runner = null;
        this.visualizer = null;
        this.visualizerManager = null;
        this.elements = null;
    }

    async loadTestMetadata() {
        try {
            const params = new URLSearchParams(window.location.hash.substring(1));
            const testId = params.get('test');
            if (!testId) {
                console.log('No test ID in URL');
                return;
            }
            // DRY Principle: Use shared path resolution from utils.js
            const resourcesPrefix = getResourcesPath();

            const metadataUrl = `${resourcesPrefix}/${testId}/metadata.txt`;
            const response = await fetch(metadataUrl);
            if (!response.ok) {
                console.warn(`metadata.txt not found for test ${testId}`);
                return;
            }
            const text = await response.text();
            const lines = text.split('\n');
            let description = '';
            let specnum = '';
            for (const line of lines) {
                if (line.startsWith('description:')) {
                    description = line.substring('description:'.length).trim();
                } else if (line.startsWith('specnum:')) {
                    specnum = line.substring('specnum:'.length).trim();
                }
            }
            const descElement = document.getElementById('test-description');
            if (descElement && description) {
                const specLabel = specnum ? ` (W3C SCXML ${specnum})` : '';
                descElement.textContent = description + specLabel;
            }
            console.log(`Loaded metadata for test ${testId}: ${description}`);
        } catch (error) {
            console.error('Error loading test metadata:', error);
        }
    }

    /**
     * Initialize controller state after runner.initialize()
     * W3C SCXML 3.13: Eventless transitions execute automatically during initialize(),
     * so we must update UI to reflect actual state (may already be in final state)
     */
    async initializeState() {
        try {
            // Update all UI panels to reflect actual machine state
            await this.updateState();

            // Check if already in final state and disable step forward if needed
            this.checkAndHandleFinalState();

            console.log('ExecutionController: Initial state updated');
        } catch (error) {
            console.error('Error initializing controller state:', error);
        }
    }

    // Delegate methods to helper modules
    escapeHtml(unsafe) { return this.formatters.escapeHtml(unsafe); }
    appendOptionalAttributes(text, action, attrNames) { return this.formatters.appendOptionalAttributes(text, action, attrNames); }
    formatAction(action) { return this.formatters.formatAction(action); }
    getW3CReference(transition) { return this.formatters.getW3CReference(transition); }
    formatValue(value) { return this.formatters.formatValue(value); }
    getElementKeyFromId(id) { return this.formatters.getElementKeyFromId(id); }

    updateStepCounter() { return this.uiUpdater.updateStepCounter(); }
    getCurrentActiveStates() { return this.uiUpdater.getCurrentActiveStates(); }
    updateEventQueue() { return this.uiUpdater.updateEventQueue(); }
    updateScheduledEvents() { return this.uiUpdater.updateScheduledEvents(); }
    startScheduledEventsTimer() { return this.uiUpdater.startScheduledEventsTimer(); }
    stopScheduledEventsTimer() { return this.uiUpdater.stopScheduledEventsTimer(); }
    updateDataModel() { return this.uiUpdater.updateDataModel(); }
    updateStateActions() { return this.uiUpdater.updateStateActions(); }
    updateTransitionInfo(detail) { return this.uiUpdater.updateTransitionInfo(detail); }
    updateLog() { return this.uiUpdater.updateLog(); }
    updateState() { return this.uiUpdater.updateState(); }
    updateStateDiagram() { return this.uiUpdater.updateStateDiagram(); }
    updateTransitionAnimation() { return this.uiUpdater.updateTransitionAnimation(); }

    setupControls() { return this.controlHandler.setupControls(); }
    setupEventButtons() { return this.controlHandler.setupEventButtons(); }
    checkAndHandleFinalState() { return this.controlHandler.checkAndHandleFinalState(); }
    focusState(stateId) { return this.controlHandler.focusState(stateId); }
    highlightStateInPanel(stateId) { return this.controlHandler.highlightStateInPanel(stateId); }
    showMessage(message, type) { return this.controlHandler.showMessage(message, type); }
    disableButton(buttonId) { return this.controlHandler.disableButton(buttonId); }
    enableButton(buttonId) { return this.controlHandler.enableButton(buttonId); }

    extractSubSCXMLInfo(structure) { return this.breadcrumbManager.extractSubSCXMLInfo(structure); }
    updateBreadcrumb() { return this.breadcrumbManager.updateBreadcrumb(); }

    // Execution control methods
    stepForward() { return this.controlHandler.stepForward(); }
    stepBackward() { return this.controlHandler.stepBackward(); }
    reset() { return this.controlHandler.reset(); }
    raiseEvent(eventName, eventData) { return this.controlHandler.raiseEvent(eventName, eventData); }
    removeInternalEvent(index) { return this.controlHandler.removeInternalEvent(index); }
    removeExternalEvent(index) { return this.controlHandler.removeExternalEvent(index); }

    // Navigation methods
    handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId) { return this.breadcrumbManager.handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId); }
    navigateToChild(stateId, childStructure, childInfo) { return this.breadcrumbManager.navigateToChild(stateId, childStructure, childInfo); }
    navigateBack() { return this.breadcrumbManager.navigateBack(); }
    navigateToDepth(depth) { return this.breadcrumbManager.navigateToDepth(depth); }
}
