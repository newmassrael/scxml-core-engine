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

        // Cache DOM elements for performance
        this.elements = {
            stepCounter: document.getElementById('step-counter'),
            eventQueuePanel: document.getElementById('event-queue-panel'),
            dataModelPanel: document.getElementById('data-model-panel'),
            stateActionsPanel: document.getElementById('state-actions-panel'),
            transitionInfoPanel: document.getElementById('transition-info-panel'),
            logPanel: document.getElementById('log-panel'),
            singleViewContainer: document.getElementById('single-view-container'),
            splitViewContainer: document.getElementById('split-view-container'),
            childDiagramsContainer: document.getElementById('child-diagrams-container'),
            communicationLog: document.getElementById('communication-log'),
            btnStepBack: document.getElementById('btn-step-back'),
            btnStepForward: document.getElementById('btn-step-forward'),
            btnReset: document.getElementById('btn-reset'),
            eventButtonsContainer: document.getElementById('event-buttons-container')
        };

        this.setupControls();
        this.setupEventButtons();
        this.loadTestMetadata();

        // Initialize: Update state to reflect actual machine state (may already be in final state due to eventless transitions)
        this.initializeState();
    }

    /**
     * Setup control buttons and keyboard shortcuts
     */
    setupControls() {
        // Step buttons
        if (this.elements.btnStepBack) {
            this.elements.btnStepBack.onclick = () => this.stepBackward();
        }

        if (this.elements.btnStepForward) {
            this.elements.btnStepForward.onclick = () => this.stepForward();
        }

        if (this.elements.btnReset) {
            this.elements.btnReset.onclick = () => this.reset();
        }

        // Keyboard shortcuts
        this.keyboardHandler = (e) => {
            // Don't capture if typing in input fields
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
                return;
            }

            if (e.key === 'ArrowLeft') {
                e.preventDefault();
                this.stepBackward();
            } else if (e.key === 'ArrowRight') {
                e.preventDefault();
                this.stepForward();
            } else if (e.key === 'r' || e.key === 'R') {
                e.preventDefault();
                this.reset();
            }
        };
        
        document.addEventListener('keydown', this.keyboardHandler);

        // W3C SCXML 3.3: Transition click event handler
        this.transitionClickHandler = (event) => {
            this.updateTransitionInfo(event.detail);
        };
        document.addEventListener('transition-click', this.transitionClickHandler);

        console.log('Execution controls initialized (Arrow keys: step, R: reset)');
    }

    /**
     * Setup event buttons from available events
     */
    setupEventButtons() {
        if (!this.elements.eventButtonsContainer) return;

        // Clear existing content
        this.elements.eventButtonsContainer.innerHTML = '';

        if (this.availableEvents.length === 0) {
            this.elements.eventButtonsContainer.innerHTML = '<div class="text-small text-gray">No events available</div>';
            return;
        }

        // Create button for each event
        this.availableEvents.forEach(eventName => {
            const button = document.createElement('button');
            button.className = 'btn btn-sm event-button';
            button.textContent = eventName;
            button.onclick = () => this.raiseEvent(eventName);
            this.elements.eventButtonsContainer.appendChild(button);
        });

        console.log(`Created ${this.availableEvents.length} event buttons`);
    }

    /**
     * Load test metadata from metadata.txt and update description
     */
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

    /**
     * Check if state machine is in final state and update UI accordingly
     * W3C SCXML 3.13: Final state means no more transitions possible
     */
    checkAndHandleFinalState() {
        if (this.runner.isInFinalState()) {
            this.disableButton('btn-step-forward');
            const activeStates = this.runner.getActiveStates();
            const stateList = activeStates.length > 0 ? activeStates.join(', ') : 'unknown';
            console.log(`State machine in final state: ${stateList}`);
        } else {
            this.enableButton('btn-step-forward');
        }
    }

    /**
     * Execute one step forward
     */
    async stepForward() {
        try {
            const success = this.runner.stepForward();

            if (!success) {
                this.showMessage('Send an event first (use event buttons below)', 'info');
                this.disableButton('btn-step-forward');
                return;
            }

            this.currentStep = this.runner.getCurrentStep();
            await this.updateState();

            // Update event queue after processing
            this.updateEventQueue();

            // Check if reached final state and disable button if needed
            this.checkAndHandleFinalState();

            console.log(`Step ${this.currentStep} executed`);
        } catch (error) {
            console.error('Error during stepForward:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    /**
     * Execute one step backward (time-travel debugging)
     */
    async stepBackward() {
        try {
            const success = this.runner.stepBackward();

            if (!success) {
                this.showMessage('Already at initial state', 'info');
                return;
            }

            this.currentStep = this.runner.getCurrentStep();

            // C++ StateSnapshot automatically restores transition info
            // Get restored transition from C++ and highlight it
            const restoredTransition = this.runner.getLastTransition();
            console.log(`[STEP BACK] Restored to step ${this.currentStep}, transition:`, restoredTransition);

            if (restoredTransition && restoredTransition.source && restoredTransition.target) {
                console.log(`[STEP BACK] Highlighting restored transition: ${restoredTransition.source} → ${restoredTransition.target}`);
                this.visualizer.highlightTransition(restoredTransition);
                // Update Transition Info panel to show restored transition
                this.visualizer.showTransitionInfo(restoredTransition);
            } else {
                console.log(`[STEP BACK] No transition at step ${this.currentStep}, clearing highlights`);
                this.visualizer.clearTransitionHighlights();
            }

            // Set flag to skip transition animation during backward step
            this.isSteppingBackward = true;
            await this.updateState();
            this.isSteppingBackward = false;

            // Re-enable forward button
            this.enableButton('btn-step-forward');

            console.log(`Restored to step ${this.currentStep}`);
        } catch (error) {
            console.error('Error during stepBackward:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    /**
     * Reset to initial state
     */
    async reset() {
        try {
            this.runner.reset();
            this.currentStep = 0;

            // Clear all transition highlights
            this.visualizer.clearTransitionHighlights();

            await this.updateState();

            // Check if in final state and update button accordingly
            this.checkAndHandleFinalState();

            // Clear log
            if (this.elements.logPanel) {
                this.elements.logPanel.innerHTML = '<div class="log-entry">Reset to initial configuration</div>';
            }

            console.log('Reset to initial state');
        } catch (error) {
            console.error('Error during reset:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    /**
     * Raise external event (for custom event injection)
     */
    async raiseEvent(eventName, eventData = '') {
        try {
            this.runner.raiseEvent(eventName, eventData);

            // W3C SCXML 3.13: Event queuing is NOT a step
            // Step counter remains unchanged until stepForward() processes the event
            console.log(`📨 Event queued: ${eventName} (press Step Forward to process)`);

            // Update UI to show pending event in queue
            this.updateEventQueue();

            // Enable Step Forward button
            this.enableButton('btn-step-forward');

            this.showMessage(`Event "${eventName}" added to queue`, 'info');
        } catch (error) {
            console.error('Error raising event:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    /**
     * Remove event from internal queue by index
     * W3C SCXML 3.13: Manual event queue manipulation for debugging
     */
    async removeInternalEvent(index) {
        try {
            const success = this.runner.removeInternalEvent(index);

            if (success) {
                console.log(`🗑️ Internal event at index ${index} removed`);

                // Update UI to reflect queue modification
                this.updateEventQueue();
                this.updateState();

                this.showMessage(`Internal event removed`, 'info');
            } else {
                console.warn(`⚠️ Failed to remove internal event at index ${index}`);
                this.showMessage(`Invalid event index`, 'warning');
            }
        } catch (error) {
            console.error('Error removing internal event:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    /**
     * Remove event from external queue by index
     * W3C SCXML 3.13: Manual event queue manipulation for debugging
     */
    async removeExternalEvent(index) {
        try {
            const success = this.runner.removeExternalEvent(index);

            if (success) {
                console.log(`🗑️ External event at index ${index} removed`);

                // Update UI to reflect queue modification
                this.updateEventQueue();
                this.updateState();

                this.showMessage(`External event removed`, 'info');
            } else {
                console.warn(`⚠️ Failed to remove external event at index ${index}`);
                this.showMessage(`Invalid event index`, 'warning');
            }
        } catch (error) {
            console.error('Error removing external event:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    /**
     * Update all UI panels with current state
     */
    async updateState() {
        try {
            // Update step counter
            this.updateStepCounter();

            // Get previous active states (stored from last update)
            const previousActiveStates = this.previousActiveStates || [];

            // Update last transition animation BEFORE updating state diagram
            // (so we can still compare previousActiveStates vs currentActiveStates)
            // Skip transition animation during step backward (time travel doesn't show transitions)
            if (!this.isSteppingBackward) {
                await this.updateTransitionAnimation();
            }

            // Update state diagram (this updates previousActiveStates)
            await this.updateStateDiagram();

            // Update child state machines visualization
            await this.updateChildrenVisualization();

            // Update panels
            this.updateEventQueue();
            this.updateDataModel();
            this.updateStateActions();
            this.updateLog();

            // Auto-highlight newly activated state in State Actions panel
            const currentActiveStates = this.getCurrentActiveStates();
            const newlyActivatedStates = currentActiveStates.filter(state => 
                !previousActiveStates.includes(state)
            );

            // Highlight the first newly activated state (if any)
            if (newlyActivatedStates.length > 0) {
                const stateToHighlight = newlyActivatedStates[0];
                console.log(`[Auto Highlight] State activated: ${stateToHighlight}`);
                this.highlightStateInPanel(stateToHighlight);
            }

        } catch (error) {
            console.error('Error updating state:', error);
        }
    }

    /**
     * Update step counter display
     */
    updateStepCounter() {
        if (this.elements.stepCounter) {
            this.elements.stepCounter.textContent = `Step ${this.currentStep}`;
        }
    }

    /**
     * Update state diagram highlighting
     */
    async updateStateDiagram() {
        try {
            const activeStatesVector = this.runner.getActiveStates();

            // Convert Emscripten VectorString to JavaScript array
            const activeStates = [];
            for (let i = 0; i < activeStatesVector.size(); i++) {
                activeStates.push(activeStatesVector.get(i));
            }

            // Store previous active states for transition detection
            if (!this.previousActiveStates) {
                this.previousActiveStates = [];
            }

            this.visualizer.highlightActiveStates(activeStates);

            // Update previous states for next comparison
            this.previousActiveStates = [...activeStates];
        } catch (error) {
            console.error('Error updating state diagram:', error);
        }
    }

    /**
     * Get current active states as JavaScript array
     * Helper method for panels that need active state information
     */
    getCurrentActiveStates() {
        try {
            const activeStatesVector = this.runner.getActiveStates();
            const activeStates = [];
            for (let i = 0; i < activeStatesVector.size(); i++) {
                activeStates.push(activeStatesVector.get(i));
            }
            return activeStates;
        } catch (error) {
            console.error('Error getting active states:', error);
            return [];
        }
    }

    /**
     * Animate last transition
     * Zero Duplication: CSS handles animation, JavaScript only toggles classes
     */
    async updateTransitionAnimation() {
        try {
            // Get transition from C++ (StateSnapshot restores this correctly)
            const lastTransition = this.runner.getLastTransition();
            console.log('[UPDATE TRANSITION] getLastTransition() returned:', lastTransition);

            if (lastTransition && lastTransition.source && lastTransition.target) {
                console.log(`[UPDATE TRANSITION] Valid transition: ${lastTransition.source} → ${lastTransition.target}`);

                // Single call: highlight with CSS animation
                console.log('[UPDATE TRANSITION] Calling highlightTransition() (CSS animation)...');
                this.visualizer.highlightTransition(lastTransition);
                console.log('[UPDATE TRANSITION] Calling showTransitionInfo()...');
                this.visualizer.showTransitionInfo(lastTransition);
                console.log('[UPDATE TRANSITION] All transition updates complete');
            } else {
                console.log('[UPDATE TRANSITION] No transition to display (initial state or no state change)');
            }
        } catch (error) {
            console.error('Error animating transition:', error);
        }
    }

    /**
     * Update event queue panel
     */
    updateEventQueue() {
        if (!this.elements.eventQueuePanel) {
            console.warn('event-queue-panel not found');
            return;
        }

        try {
            const queue = this.runner.getEventQueue();
            console.log('[DEBUG] getEventQueue() returned:', queue);
            console.log('[DEBUG] queue type:', typeof queue);
            console.log('[DEBUG] queue.external:', queue.external);

            let html = '<h4>Internal Queue</h4>';
            if (queue.internal && queue.internal.length > 0) {
                html += queue.internal.map((e, index) =>
                    `<div class="event event-internal">
                        <span class="event-name">${e.name || e}</span>
                        <button class="event-delete-btn" onclick="window.executionController.removeInternalEvent(${index})" title="Remove event">×</button>
                    </div>`
                ).join('');
            } else {
                html += '<div class="event">Empty</div>';
            }

            html += '<h4 style="margin-top: 16px;">External Queue</h4>';
            if (queue.external && queue.external.length > 0) {
                html += queue.external.map((e, index) =>
                    `<div class="event event-external">
                        <span class="event-name">${e.name || e}</span>
                        <button class="event-delete-btn" onclick="window.executionController.removeExternalEvent(${index})" title="Remove event">×</button>
                    </div>`
                ).join('');
            } else {
                html += '<div class="event">Empty</div>';
            }

            this.elements.eventQueuePanel.innerHTML = html;
        } catch (error) {
            console.error('Error updating event queue:', error);
            this.elements.eventQueuePanel.innerHTML = '<div class="error-message">Failed to load event queue</div>';
        }
    }

    /**
     * Update data model panel
     */
    updateDataModel() {
        if (!this.elements.dataModelPanel) return;

        try {
            const dataModel = this.runner.getDataModel();

            const entries = Object.entries(dataModel);

            if (entries.length === 0) {
                this.elements.dataModelPanel.innerHTML = '<div class="data-entry">No data model variables</div>';
                return;
            }

            const html = entries.map(([key, value]) => `
                <div class="data-entry">
                    <span class="var-name">${key}</span>:
                    <span class="var-value">${this.formatValue(value)}</span>
                </div>
            `).join('');

            this.elements.dataModelPanel.innerHTML = html;
        } catch (error) {
            console.error('Error updating data model:', error);
            this.elements.dataModelPanel.innerHTML = '<div class="error-message">Failed to load data model</div>';
        }
    }

    /**
     * Update state actions panel
     * W3C SCXML 3.7: Display onentry/onexit actions and transitions for all states
     * Interactive: Click state header to focus on diagram
     */
    updateStateActions() {
        if (!this.elements.stateActionsPanel) return;

        try {
            // Get all states from visualizer (show all, not just ones with actions)
            const allStates = this.visualizer.states;

            if (allStates.length === 0) {
                this.elements.stateActionsPanel.innerHTML = '<div class="action-info">No states defined</div>';
                return;
            }

            // Get active states for highlighting
            const activeStates = this.getCurrentActiveStates();

            const html = allStates.map(state => {
                const isActive = activeStates.includes(state.id);
                let content = `<div class="action-info ${isActive ? 'active-state-info' : ''}" data-state-id="${this.escapeHtml(state.id)}">`;

                // State header with type and active status (clickable)
                content += `<div class="action-state-header clickable-state-header" data-state-id="${this.escapeHtml(state.id)}">`;
                content += `State: ${this.escapeHtml(state.id)}`;
                content += ` <span class="state-type">(${this.escapeHtml(state.type || 'atomic')}</span>`;
                if (isActive) {
                    content += `<span class="state-active">, active</span>`;
                }
                content += `)</div>`;

                // onentry actions
                if (state.onentry && state.onentry.length > 0) {
                    state.onentry.forEach(action => {
                        content += `<div class="action-item action-onentry">`;
                        content += `<span class="action-type">↓ ${this.escapeHtml(action.actionType)}</span>`;
                        content += `<span class="action-details">${this.formatAction(action).substring(2)}</span>`;
                        content += `</div>`;
                    });
                }

                // onexit actions
                if (state.onexit && state.onexit.length > 0) {
                    state.onexit.forEach(action => {
                        content += `<div class="action-item action-onexit">`;
                        content += `<span class="action-type">↑ ${this.escapeHtml(action.actionType)}</span>`;
                        content += `<span class="action-details">${this.formatAction(action).substring(2)}</span>`;
                        content += `</div>`;
                    });
                }

                // Show message if no actions
                if ((!state.onentry || state.onentry.length === 0) &&
                    (!state.onexit || state.onexit.length === 0)) {
                    content += `<div class="action-item action-none">No onentry/onexit actions</div>`;
                }

                // Outgoing transitions
                const outgoingTransitions = this.visualizer.transitions.filter(t => t.source === state.id);
                if (outgoingTransitions.length > 0) {
                    content += `<div class="action-item action-transitions">`;
                    content += `<span class="action-type">→ Transitions:</span> `;
                    const transitionList = outgoingTransitions.map(t => {
                        const event = t.event || 'eventless';
                        return `${this.escapeHtml(event)}→${this.escapeHtml(t.target)}`;
                    }).join(', ');
                    content += `<span class="action-details">${transitionList}</span>`;
                    content += `</div>`;
                }

                content += `</div>`;
                return content;
            }).join('');

            this.elements.stateActionsPanel.innerHTML = html;

            // Add click event listeners to state headers
            const stateHeaders = this.elements.stateActionsPanel.querySelectorAll('.clickable-state-header');
            stateHeaders.forEach(header => {
                header.addEventListener('click', (e) => {
                    const stateId = e.currentTarget.getAttribute('data-state-id');
                    this.focusState(stateId);
                });
            });
        } catch (error) {
            console.error('Error updating state actions:', error);
            this.elements.stateActionsPanel.innerHTML = '<div class="error-message">Failed to load state actions</div>';
        }
    }

    /**
     * Focus on a specific state in the diagram
     * Temporarily highlights the state for visual feedback
     */
    focusState(stateId) {
        if (!stateId || !this.visualizer) return;

        try {
            console.log(`[Focus State] Focusing on state: ${stateId}`);

            // Search across ALL diagram containers (single view + split view parent/children)
            const allDiagramContainers = [
                '#state-diagram-single',
                '#state-diagram-parent-split',
                ...Array.from(document.querySelectorAll('[id^="state-diagram-child-"]')).map(el => `#${el.id}`)
            ];

            let foundStateNode = null;
            let foundInContainer = null;

            // Find the state element across all containers
            for (const containerId of allDiagramContainers) {
                const container = d3.select(containerId);
                if (!container.empty()) {
                    const stateNode = container.select(`[data-state-id="${stateId}"]`);
                    if (!stateNode.empty()) {
                        foundStateNode = stateNode;
                        foundInContainer = container;
                        console.log(`[Focus State] Found state in container: ${containerId}`);
                        break;
                    }
                }
            }

            if (!foundStateNode) {
                console.warn(`[Focus State] State not found in any container: ${stateId}`);
                return;
            }

            console.log(`[Focus State] Found state element: ${stateId}`);

            // Visual feedback: add blue border effect (like Transition Info)
            // Select all state elements: g.node.state, compound-collapsed, compound-container
            const stateElements = foundInContainer.selectAll('.node.state, .compound-collapsed, .compound-container');
            stateElements.classed('focused', d => {
                const isFocused = d && d.id === stateId;
                if (isFocused) {
                    console.log(`[Focus State] Adding focused class to: ${stateId}`);
                }
                return isFocused;
            });

            // Scroll the state into view
            const stateNodeElement = foundStateNode.node();
            if (stateNodeElement && stateNodeElement.scrollIntoView) {
                stateNodeElement.scrollIntoView({
                    behavior: 'smooth',
                    block: 'center',
                    inline: 'center'
                });
                console.log(`[Focus State] Scrolling state into view`);
            }

            // Remove focus after animation duration
            setTimeout(() => {
                stateElements.classed('focused', false);
                console.log(`[Focus State] Removed focused class from all states`);
            }, FOCUS_HIGHLIGHT_DURATION);
        } catch (error) {
            console.error('Error focusing state:', error);
        }
    }

    /**
     * Highlight a state in the State Actions panel
     * @param {string} stateId - State ID to highlight
     */
    highlightStateInPanel(stateId) {
        if (!stateId) return;

        try {
            console.log(`[Highlight Panel] Highlighting state in panel: ${stateId}`);

            // Find the state info block in the State Actions panel
            const stateInfoBlocks = document.querySelectorAll('.action-info');
            
            // Remove previous highlights
            stateInfoBlocks.forEach(block => {
                block.classList.remove('panel-highlighted');
            });

            // Add highlight to the clicked state's entire block
            stateInfoBlocks.forEach(block => {
                const blockStateId = block.getAttribute('data-state-id');
                if (blockStateId === stateId) {
                    block.classList.add('panel-highlighted');
                    
                    // Scroll into view
                    block.scrollIntoView({
                        behavior: 'smooth',
                        block: 'nearest'
                    });
                    
                    console.log(`[Highlight Panel] Added highlight to: ${stateId}`);
                }
            });

            // Remove highlight after animation duration
            setTimeout(() => {
                stateInfoBlocks.forEach(block => {
                    block.classList.remove('panel-highlighted');
                });
                console.log(`[Highlight Panel] Removed highlight from panel`);
            }, PANEL_HIGHLIGHT_DURATION);
        } catch (error) {
            console.error('Error highlighting state in panel:', error);
        }
    }

    /**
     * Escape HTML to prevent XSS attacks
     * @param {string} unsafe - Unsafe string from user input (SCXML file content)
     * @return {string} HTML-escaped safe string
     */
    escapeHtml(unsafe) {
        if (!unsafe) return '';
        return String(unsafe)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }

    /**
     * Format action for display (DRY principle)
     * W3C SCXML 3.7: Reusable action formatting logic for both state and transition actions
     * @param {object} action - Action object with actionType and properties
     * @return {string} Formatted action text
     */
    formatAction(action) {
        let text = `• ${this.escapeHtml(action.actionType)}`;
        
        if (action.actionType === 'raise') {
            text += ` event: ${this.escapeHtml(action.event)}`;
        } else if (action.actionType === 'assign') {
            text += ` ${this.escapeHtml(action.location)} = ${this.escapeHtml(action.expr)}`;
        } else if (action.actionType === 'log') {
            if (action.label) text += ` label: ${this.escapeHtml(action.label)}`;
            if (action.expr) text += ` expr: ${this.escapeHtml(action.expr)}`;
        } else if (action.actionType === 'foreach') {
            text += ` item: ${this.escapeHtml(action.item || 'none')}`;
            if (action.index) text += `, index: ${this.escapeHtml(action.index)}`;
            if (action.array) text += `, array: ${this.escapeHtml(action.array)}`;
        } else if (action.actionType === 'send') {
            if (action.event) text += ` event: ${this.escapeHtml(action.event)}`;
            if (action.target) text += ` target: ${this.escapeHtml(action.target)}`;
            if (action.delay) text += ` delay: ${this.escapeHtml(action.delay)}`;
        }
        
        return text;
    }

    /**
     * Update transition info panel
     * W3C SCXML 3.3: Display transition details (source, target, event, guard, actions)
     */
    updateTransitionInfo(detail) {
        if (!this.elements.transitionInfoPanel) return;

        try {
            if (!detail || !detail.source || !detail.target) {
                this.elements.transitionInfoPanel.innerHTML = '<div class="transition-hint">Click a transition to view details</div>';
                return;
            }

            let html = '<div class="transition-details">';
            
            // Transition header: source → target
            html += `<div class="transition-header">${this.escapeHtml(detail.source)} → ${this.escapeHtml(detail.target)}</div>`;

            // Event field
            if (detail.event) {
                html += `<div class="transition-field">`;
                html += `<span class="transition-field-label">Event:</span>`;
                html += `<span class="transition-field-value">${this.escapeHtml(detail.event)}</span>`;
                html += `</div>`;
            } else {
                html += `<div class="transition-field">`;
                html += `<span class="transition-field-label">Event:</span>`;
                html += `<span class="transition-field-value">(eventless)</span>`;
                html += `</div>`;
            }

            // Guard condition (W3C SCXML 3.12.1)
            if (detail.cond) {
                html += `<div class="transition-guard">`;
                html += `<div class="transition-field">`;
                html += `<span class="transition-field-label">Guard:</span>`;
                html += `<span class="transition-field-value">${this.escapeHtml(detail.cond)}</span>`;
                html += `</div>`;
                html += `</div>`;
            }

            // Actions (W3C SCXML 3.7)
            if (detail.actions && detail.actions.length > 0) {
                html += `<div class="transition-actions">`;
                html += `<div class="transition-field">`;
                html += `<span class="transition-field-label">Actions:</span>`;
                html += `</div>`;
                
                detail.actions.forEach(action => {
                    html += `<div class="transition-action-item">`;
                    html += this.formatAction(action);
                    html += `</div>`;
                });
                
                html += `</div>`;
            }

            html += '</div>';
            this.elements.transitionInfoPanel.innerHTML = html;
        } catch (error) {
            console.error('Error updating transition info:', error);
            this.elements.transitionInfoPanel.innerHTML = '<div class="error-message">Failed to load transition info</div>';
        }
    }

    /**
     * Update execution log
     */
    updateLog() {
        if (!this.elements.logPanel) return;

        try {
            const lastTransition = this.runner.getLastTransition();

            if (!lastTransition || !lastTransition.source) {
                return;
            }

            const logEntry = document.createElement('div');
            logEntry.className = 'log-entry';

            const eventText = lastTransition.event || '(eventless)';

            logEntry.innerHTML = `
                <span class="log-step">[Step ${this.currentStep}]</span>
                <span class="log-event">${eventText}</span>
                <span class="log-transition">${lastTransition.source} → ${lastTransition.target}</span>
                ${this.getW3CReference(lastTransition)}
            `;

            // Prepend (newest first)
            this.elements.logPanel.insertBefore(logEntry, this.elements.logPanel.firstChild);

            // Limit log entries (keep last 100)
            while (this.elements.logPanel.children.length > 100) {
                this.elements.logPanel.removeChild(this.elements.logPanel.lastChild);
            }
        } catch (error) {
            console.error('Error updating log:', error);
        }
    }

    /**
     * Get W3C spec reference for transition
     */
    getW3CReference(transition) {
        try {
            // Get test ID from URL
            const params = new URLSearchParams(window.location.hash.substring(1));
            const testId = params.get('test');

            if (!testId || !window.specReferences || !window.specReferences[testId]) {
                return '';
            }

            const testRefs = window.specReferences[testId];

            // Check transition-specific references
            if (testRefs.transitions && transition.id && testRefs.transitions[transition.id]) {
                const ref = testRefs.transitions[transition.id];
                return `<a href="https://www.w3.org/TR/scxml/#${ref.section}"
                           target="_blank"
                           class="w3c-ref"
                           title="${ref.description}">
                        W3C SCXML ${ref.section}
                       </a>`;
            }

            // Fallback to general test specs
            if (testRefs.specs && testRefs.specs.length > 0) {
                const section = testRefs.specs[0];
                return `<a href="https://www.w3.org/TR/scxml/#${section}"
                           target="_blank"
                           class="w3c-ref"
                           title="${testRefs.description}">
                        W3C SCXML ${section}
                       </a>`;
            }
        } catch (error) {
            console.error('Error getting W3C reference:', error);
        }

        return '';
    }

    /**
     * Format value for display
     */
    formatValue(value) {
        if (typeof value === 'string') {
            return `"${value}"`;
        }
        if (typeof value === 'object') {
            return JSON.stringify(value);
        }
        return String(value);
    }

    /**
     * Show debug message (console only, for development)
     * For user-facing errors, see showMessage() in main.js
     */
    showMessage(message, type = 'info') {
        console.log(`[${type.toUpperCase()}] ${message}`);
    }

    /**
     * Disable button
     */
    disableButton(buttonId) {
        const button = this.elements[this.getElementKeyFromId(buttonId)];
        if (button) {
            button.disabled = true;
            button.classList.add('disabled');
        }
    }

    /**
     * Get element key from element ID
     */
    getElementKeyFromId(id) {
        const idToKeyMap = {
            'btn-step-back': 'btnStepBack',
            'btn-step-forward': 'btnStepForward',
            'btn-reset': 'btnReset'
        };
        return idToKeyMap[id] || id;
    }

    /**
     * Update child state machines visualization
     */
    async updateChildrenVisualization() {
        if (!this.runner) {
            return;
        }

        try {
            const childrenData = this.runner.getInvokedChildren();

            // W3C SCXML 6.3: Check static sub-SCXML structures (detected at load time)
            const staticChildren = this.runner.getSubSCXMLStructures ? this.runner.getSubSCXMLStructures() : [];
            const hasStaticChildren = staticChildren && staticChildren.length > 0;

            // Don't change layout if static children exist (main.js already set up split view)
            if (hasStaticChildren) {
                console.log('Static sub-SCXML detected - preserving split view layout');
                return;
            }

            if (!childrenData || !childrenData.children || childrenData.children.length === 0) {
                // No children (neither static nor runtime) - hide split view, show single diagram
                if (this.elements.singleViewContainer) this.elements.singleViewContainer.style.display = 'block';
                if (this.elements.splitViewContainer) this.elements.splitViewContainer.style.display = 'none';

                return;
            }

            // Has children - show split view, hide single diagram
            console.log(`Found ${childrenData.children.length} invoked child state machines`);

            if (this.elements.singleViewContainer) this.elements.singleViewContainer.style.display = 'none';
            if (this.elements.splitViewContainer) this.elements.splitViewContainer.style.display = 'block';

            // Update child diagrams
            for (const childInfo of childrenData.children) {
                await this.updateChildDiagram(childInfo);
            }

            // Update communication log
            this.updateCommunicationLog(childrenData);

        } catch (error) {
            console.error('Error updating children visualization:', error);
        }
    }

    /**
     * Update individual child diagram
     */
    async updateChildDiagram(childInfo) {
        const containerId = `child-diagram-${childInfo.sessionId}`;
        let container = document.getElementById(containerId);

        // Create container if doesn't exist
        if (!container) {
            if (!this.elements.childDiagramsContainer) {
                console.error('child-diagrams-container not found');
                return;
            }

            container = document.createElement('div');
            container.id = containerId;
            container.className = 'child-diagram active';
            this.elements.childDiagramsContainer.appendChild(container);

            // Create visualizer for this child
            const visualizer = new SCXMLVisualizer(containerId, childInfo.structure);

            // Store visualizer reference in manager
            if (this.visualizerManager) {
                this.visualizerManager.addChild(childInfo.sessionId, visualizer);
            }

            console.log(`Created child diagram for session: ${childInfo.sessionId}`);
        }

        // Update active states highlighting
        const visualizer = this.visualizerManager?.getChild(childInfo.sessionId);
        if (visualizer) {
            visualizer.highlightActiveStates(childInfo.activeStates);
        }
    }

    /**
     * Update communication log between parent and children
     */
    updateCommunicationLog(childrenData) {
        if (!this.elements.communicationLog) {
            return;
        }

        // For now, just show child count
        const timestamp = new Date().toLocaleTimeString();
        const entry = document.createElement('div');
        entry.className = 'comm-entry comm-info';
        entry.innerHTML = `
            <div class="comm-timestamp">${timestamp}</div>
            <div class="comm-type">Active Children: ${childrenData.children.length}</div>
        `;

        // Keep last 20 entries
        while (this.elements.communicationLog.children.length >= 20) {
            this.elements.communicationLog.removeChild(this.elements.communicationLog.firstChild);
        }

        this.elements.communicationLog.appendChild(entry);
        this.elements.communicationLog.scrollTop = this.elements.communicationLog.scrollHeight;
    }

    /**
     * Enable button
     */
    enableButton(buttonId) {
        const button = this.elements[this.getElementKeyFromId(buttonId)];
        if (button) {
            button.disabled = false;
            button.classList.remove('disabled');
        }
    }

    /**
     * Cleanup - remove event listeners and clear references
     */
    destroy() {
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

        // Clear references
        this.runner = null;
        this.visualizer = null;
        this.visualizerManager = null;
        this.elements = null;
    }
}

