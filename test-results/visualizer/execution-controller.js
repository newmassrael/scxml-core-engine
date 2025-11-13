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

        // Back button for navigation (hidden by default)
        const backButton = document.getElementById('btn-back');
        if (backButton) {
            backButton.onclick = () => this.navigateBack();
            backButton.style.display = 'none';  // Hidden until navigate to child
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
            } else if (e.key === 'b' || e.key === 'B' || e.key === 'Backspace') {
                // Back to parent state machine (if navigated to child)
                if (this.navigationStack.length > 0) {
                    e.preventDefault();
                    this.navigateBack();
                }
            }
        };
        
        document.addEventListener('keydown', this.keyboardHandler);

        // W3C SCXML 3.3: Transition click event handler
        this.transitionClickHandler = (event) => {
            this.updateTransitionInfo(event.detail);
        };
        document.addEventListener('transition-click', this.transitionClickHandler);

        // W3C SCXML 6.3: State navigate event handler for invoke child SCXML
        this.stateNavigateHandler = async (event) => {
            const {stateId, invokeSrc, invokeSrcExpr, invokeId} = event.detail;
            console.log(`State navigate event received: ${stateId} -> ${invokeSrc || invokeSrcExpr}`);
            
            // Load child SCXML structure
            await this.handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId);
        };
        document.addEventListener('state-navigate', this.stateNavigateHandler);

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

            // Set active transition (last executed) and update detail panel
            const executedTransition = this.runner.getLastTransition();
            if (executedTransition && executedTransition.source && executedTransition.target) {
                this.visualizer.setActiveTransition(executedTransition);    // Set active state (permanent)
                this.updateTransitionInfo(executedTransition);               // Update detail panel
                this.visualizer.highlightTransition(executedTransition);     // Highlight in diagram (temporary)
            }

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
            // Immediately cancel ongoing animations for instant UI response
            this.visualizer.clearTransitionHighlights();

            const success = this.runner.stepBackward();

            if (!success) {
                this.showMessage('Already at initial state', 'info');
                return;
            }

            this.currentStep = this.runner.getCurrentStep();

            // C++ StateSnapshot automatically restores transition info
            // Get restored transition from C++ and set active state
            const restoredTransition = this.runner.getLastTransition();
            console.log(`[STEP BACK] Restored to step ${this.currentStep}, transition:`, restoredTransition);

            if (restoredTransition && restoredTransition.source && restoredTransition.target) {
                console.log(`[STEP BACK] Setting active transition: ${restoredTransition.source} → ${restoredTransition.target}`);
                // Set active transition (permanent)
                this.visualizer.setActiveTransition(restoredTransition);
                // Update detail panel
                this.updateTransitionInfo(restoredTransition);
                // Highlight in diagram (temporary)
                this.visualizer.highlightTransition(restoredTransition);
            } else {
                console.log(`[STEP BACK] No transition at step ${this.currentStep}, clearing active state`);
                // Clear active transition state (step 0 = initial state, no transitions executed)
                this.visualizer.clearActiveTransition();
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

            // Clear active transition state (reset to initial, no transitions executed)
            this.visualizer.clearActiveTransition();

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

            // Update panels
            this.updateEventQueue();
            this.updateScheduledEvents();  // W3C SCXML 6.2: Visualize delayed send operations
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

                // Set active transition (permanent)
                this.visualizer.setActiveTransition(lastTransition);
                // Update detail panel
                this.updateTransitionInfo(lastTransition);
                // Highlight in diagram (temporary)
                this.visualizer.highlightTransition(lastTransition);
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
     * Update scheduled events panel (delayed <send> operations)
     * W3C SCXML 6.2: Visualize pending delayed events
     */
    updateScheduledEvents() {
        const panel = document.getElementById('scheduled-events-panel');
        if (!panel) {
            console.warn('scheduled-events-panel not found');
            return;
        }

        try {
            const scheduledEvents = this.runner.getScheduledEvents();
            console.log('[DEBUG] getScheduledEvents() returned:', scheduledEvents);

            let html = '';
            if (scheduledEvents && scheduledEvents.length > 0) {
                // Store initial snapshot with JavaScript timestamp for real-time countdown
                const jsNow = Date.now();
                html = scheduledEvents.map((event, index) => {
                    // Convert BigInt to Number for JavaScript operations
                    const remainingTime = Number(event.remainingTime);
                    const timeStr = remainingTime >= 0
                        ? `${remainingTime}ms`
                        : `ready (${Math.abs(remainingTime)}ms overdue)`;
                    return `<div class="event event-scheduled" data-initial-time="${remainingTime}" data-snapshot-time="${jsNow}" data-index="${index}">
                        <span class="event-name">${event.eventName}</span>
                        <span class="event-time">${timeStr}</span>
                    </div>`;
                }).join('');

                // Start real-time update timer if not already running
                this.startScheduledEventsTimer();
            } else {
                html = '<div class="event">No scheduled events</div>';

                // Stop timer when no scheduled events
                this.stopScheduledEventsTimer();
            }

            panel.innerHTML = html;
        } catch (error) {
            console.error('Error updating scheduled events:', error);
            panel.innerHTML = '<div class="error-message">Failed to load scheduled events</div>';
        }
    }

    /**
     * Start real-time update timer for scheduled events
     * Updates every 100ms to show decreasing remaining time
     */
    startScheduledEventsTimer() {
        if (this.scheduledEventsTimer) {
            return; // Already running
        }

        this.scheduledEventsTimer = setInterval(() => {
            const panel = document.getElementById('scheduled-events-panel');
            if (!panel) return;

            try {
                // JavaScript-side time calculation (WASM C++ time doesn't update in real-time)
                const jsNow = Date.now();
                const eventElements = panel.querySelectorAll('.event-scheduled');

                if (eventElements.length === 0) {
                    this.stopScheduledEventsTimer();
                    return;
                }

                let anyEventsReady = false;

                // Update time display using JavaScript elapsed time
                eventElements.forEach(eventElement => {
                    const initialTime = parseInt(eventElement.dataset.initialTime);
                    const snapshotTime = parseInt(eventElement.dataset.snapshotTime);
                    const elapsed = jsNow - snapshotTime;
                    const remaining = initialTime - elapsed;

                    const timeSpan = eventElement.querySelector('.event-time');
                    if (timeSpan) {
                        const timeStr = remaining >= 0
                            ? `${remaining}ms`
                            : `ready (${Math.abs(remaining)}ms overdue)`;
                        timeSpan.textContent = timeStr;

                        // Mark as ready if time expired
                        if (remaining <= 0) {
                            anyEventsReady = true;
                            eventElement.classList.add('event-ready');
                        }
                    }
                });

                // If any events are ready, poll scheduler to move them to queue
                if (anyEventsReady) {
                    console.log('[Timer] Events ready, polling scheduler to move to queue');
                    const polledCount = this.runner.pollScheduler();
                    if (polledCount > 0) {
                        console.log(`[Timer] Polled ${polledCount} events, updating UI`);
                        // Update event queue display to show moved events
                        this.updateEventQueue();
                        // Refresh scheduled events display (events should be removed)
                        this.updateScheduledEvents();
                    }
                }
            } catch (error) {
                console.error('Error in scheduled events timer:', error);
                this.stopScheduledEventsTimer();
            }
        }, 100);
    }

    /**
     * Stop real-time update timer for scheduled events
     */
    stopScheduledEventsTimer() {
        if (this.scheduledEventsTimer) {
            clearInterval(this.scheduledEventsTimer);
            this.scheduledEventsTimer = null;
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

                    // Design System: Panel highlight animation (matches Transition Info panel)
                    const actionInfo = e.currentTarget.closest('.action-info');
                    if (actionInfo) {
                        actionInfo.classList.add('panel-highlighted');
                        setTimeout(() => {
                            actionInfo.classList.remove('panel-highlighted');
                        }, 3000);  // 3s animation duration
                    }

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

            // Single-window navigation: Only check current diagram container
            const container = d3.select('#state-diagram-single');
            const foundStateNode = container.empty() ? null : container.select(`[data-state-id="${stateId}"]`);
            const foundInContainer = foundStateNode && !foundStateNode.empty() ? container : null;

            if (!foundStateNode || foundStateNode.empty()) {
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
     * Helper method to append multiple optional attributes to text
     * DRY principle: Reduces repetitive attribute formatting code
     * @param {string} text - Current text string
     * @param {object} action - Action object
     * @param {string[]} attrNames - Array of attribute names
     * @returns {string} Updated text with all present attributes
     */
    appendOptionalAttributes(text, action, attrNames) {
        let result = text;
        for (const attrName of attrNames) {
            if (action[attrName]) {
                result += ` ${attrName}: ${this.escapeHtml(action[attrName])}`;
            }
        }
        return result;
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
            text = this.appendOptionalAttributes(text, action, ['label', 'expr', 'level']);        } else if (action.actionType === 'foreach') {
            text += ` item: ${this.escapeHtml(action.item || 'none')}`;
            // Optional attributes
            if (action.index) text += `, index: ${this.escapeHtml(action.index)}`;
            if (action.array) text += `, array: ${this.escapeHtml(action.array)}`;        } else if (action.actionType === 'send') {
            // W3C SCXML 6.2: Display comprehensive send attributes
            text = this.appendOptionalAttributes(text, action, [
                'event', 'eventexpr', 'target', 'targetexpr',
                'delay', 'delayexpr', 'type', 'namelist',
                'sendid', 'idlocation', 'data', 'contentexpr'
            ]);
            // Special handling for content (preview with truncation)
            if (action.content) {
                const preview = action.content.substring(0, 50);
                text += ` content: ${this.escapeHtml(preview)}${action.content.length > 50 ? '...' : ''}`;
            }            if (action.params && action.params.length > 0) {
                text += ` params: [${action.params.map(p => `${this.escapeHtml(p.name)}=${this.escapeHtml(p.expr)}`).join(', ')}]`;
            }
        } else if (action.actionType === 'if') {
            // W3C SCXML 3.12.1: Display if condition and branches
            if (action.cond) text += ` cond: ${this.escapeHtml(action.cond)}`;
            if (action.branches && action.branches.length > 0) {
                text += ` [${action.branches.length} branches: `;
                const branchTypes = action.branches.map((b, i) => {
                    if (b.isElse) return 'else';
                    if (i === 0) return 'if';
                    return 'elseif';
                });
                text += branchTypes.join(', ') + ']';
            }
        } else if (action.actionType === 'cancel') {
            // W3C SCXML 6.3: Display cancel attributes
            text = this.appendOptionalAttributes(text, action, ['sendid', 'sendidexpr']);        } else if (action.actionType === 'script') {
            // W3C SCXML 5.9: Display script content preview
            if (action.content) {
                const preview = action.content.substring(0, 50);
                text += ` ${this.escapeHtml(preview)}${action.content.length > 50 ? '...' : ''}`;
            }
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
     * Extract sub-SCXML information from structure
     * W3C SCXML 6.3: Find states with invoke elements
     * @param {object} structure - SCXML structure
     * @returns {Array} Array of sub-SCXML info objects
     */
    extractSubSCXMLInfo(structure) {
        const subSCXMLs = [];
        
        console.log('[extractSubSCXMLInfo] Analyzing structure:', structure);
        
        // Get all sub-SCXMLs from runner (includes nested children)
        const allSubSCXMLs = this.runner.getSubSCXMLStructures ? this.runner.getSubSCXMLStructures() : [];
        console.log('[extractSubSCXMLInfo] All sub-SCXMLs from runner:', allSubSCXMLs);
        
        // Recursively find states with invoke in current structure
        const findInvokeStates = (states, parentPath = '') => {
            for (const state of states) {
                const statePath = parentPath ? `${parentPath}.${state.id}` : state.id;
                
                if (state.hasInvoke && state.invokeSrc) {
                    console.log(`[extractSubSCXMLInfo] Found invoke state: ${state.id}, src: ${state.invokeSrc}`);
                    
                    // Find matching sub-SCXML from runner's list
                    const matchingSubSCXML = allSubSCXMLs.find(sub => 
                        sub.parentStateId === state.id || 
                        sub.srcPath === state.invokeSrc
                    );
                    
                    if (matchingSubSCXML) {
                        subSCXMLs.push(matchingSubSCXML);
                    } else {
                        // Store metadata even if structure not available
                        subSCXMLs.push({
                            parentStateId: state.id,
                            srcPath: state.invokeSrc,
                            invokeId: state.invokeId || state.invokeSrc,
                            structure: null  // Structure not available
                        });
                    }
                }
                
                // Recursively check children
                if (state.children && state.children.length > 0) {
                    findInvokeStates(state.children, statePath);
                }
            }
        };
        
        if (structure.states) {
            findInvokeStates(structure.states);
        }
        
        console.log(`Extracted ${subSCXMLs.length} sub-SCXML(s) from structure`);
        return subSCXMLs;
    }

    /**
     * Handle state navigation to child SCXML
     * W3C SCXML 6.3: Load child structure and navigate
     * @param {string} stateId - Parent state ID
     * @param {string} invokeSrc - Static invoke src
     * @param {string} invokeSrcExpr - Dynamic invoke srcexpr
     * @param {string} invokeId - Invoke ID
     */
    async handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId) {
        console.log(`[handleStateNavigation] Handling navigation from state: ${stateId}`);
        console.log(`[handleStateNavigation] Current machine:`, this.currentMachine);

        // Get child structure from current machine (not root runner)
        // This allows navigation within child machines, not just from root
        const subSCXMLs = this.currentMachine.subSCXMLs || [];
        console.log(`[handleStateNavigation] Available subSCXMLs:`, subSCXMLs);
        
        // Debug: Show all parentStateIds
        subSCXMLs.forEach((sub, idx) => {
            console.log(`  [${idx}] parentStateId: "${sub.parentStateId}", srcPath: "${sub.srcPath}"`);
        });
        
        // Find matching child structure by parent state ID
        console.log(`[handleStateNavigation] Looking for parentStateId: "${stateId}"`);
        const childInfo = subSCXMLs.find(info => info.parentStateId === stateId);
        
        if (childInfo) {
            console.log(`Found static child SCXML: ${childInfo.srcPath}`);
            
            // Get child structure from runner
            const childStructure = childInfo.structure;
            
            if (childStructure) {
                // Navigate to child with childInfo for better labeling
                await this.navigateToChild(stateId, childStructure, childInfo);
            } else {
                console.error(`Child structure not available for ${childInfo.srcPath}`);
                alert(`Cannot navigate to child SCXML: ${childInfo.srcPath}

The child SCXML structure was not loaded. This may happen with:
- Dynamic invoke (srcexpr or contentExpr)
- Failed file loading or parsing

Note: Both file-based (src) and content-based (inline <content>) invoke are supported.`);
            }
        } else {
            console.warn(`No child SCXML found for state ${stateId}`);

            // Check if this is a dynamic invoke (srcexpr or contentExpr)
            const stateHasInvoke = invokeSrc || invokeSrcExpr ||
                this.currentMachine.structure?.states?.find(s => s.id === stateId && s.hasInvoke);

            if (invokeSrcExpr) {
                console.warn(`Dynamic invoke (srcexpr) not yet supported: ${invokeSrcExpr}`);
                alert(`Dynamic invoke not supported for navigation

State "${stateId}" uses srcexpr attribute, which requires runtime evaluation.

Navigation is only supported for static src attribute.`);
            } else if (stateHasInvoke) {
                alert(`No sub-SCXML found for state "${stateId}"

This state has an invoke element, but it was not loaded. This may happen with:
- Failed file loading or parsing
- Dynamic invoke with srcexpr or contentExpr (not supported for navigation)

Note: Both file-based (src) and content-based (inline <content>) invoke are supported.`);
            } else {
                alert(`No sub-SCXML found for state "${stateId}"

This state does not appear to have an invoke element.`);
            }
        }
    }

    /**
     * Navigate to child state machine
     * W3C SCXML 6.3: Invoke transitions to child state machine visualization
     * @param {string} stateId - Parent state ID with invoke
     * @param {object} childStructure - Child SCXML structure
     * @param {object} childInfo - Child info with srcPath (optional)
     */
    async navigateToChild(stateId, childStructure, childInfo = null) {
        console.log(`Navigating to child state machine from state: ${stateId}`);

        // Push current machine to stack for back navigation
        this.navigationStack.push({
            id: this.currentMachine.id,
            label: this.currentMachine.label,
            structure: this.currentMachine.structure,
            visualizer: this.currentMachine.visualizer,
            subSCXMLs: this.currentMachine.subSCXMLs
        });

        // Create new visualizer for child in single-view container
        const containerId = 'state-diagram-single';
        const container = document.getElementById(containerId);
        if (!container) {
            console.error('state-diagram-single container not found');
            return;
        }

        // Clear existing visualization
        container.innerHTML = '';

        // Create child visualizer
        const childVisualizer = new SCXMLVisualizer(containerId, childStructure);

        // Extract sub-SCXML info from child structure
        // Look for states with hasInvoke = true in the child structure
        const childSubSCXMLs = this.extractSubSCXMLInfo(childStructure);

        // Generate label based on srcPath
        let label = `Child of ${stateId}`;
        if (childInfo && childInfo.srcPath) {
            if (childInfo.srcPath.startsWith('inline-content:')) {
                label = `${stateId} (inline content)`;
            } else {
                // Extract filename from path
                const filename = childInfo.srcPath.split('/').pop();
                label = `${stateId} (${filename})`;
            }
        }

        // Update current machine
        this.currentMachine = {
            id: stateId,
            label: label,
            structure: childStructure,
            visualizer: childVisualizer,
            subSCXMLs: childSubSCXMLs
        };

        // Update breadcrumb UI
        this.updateBreadcrumb();

        // Show back button
        const backButton = document.getElementById('btn-back');
        if (backButton) {
            backButton.style.display = 'block';
        }

        // Update state highlighting for child
        await this.updateState();

        console.log(`Navigation complete. Stack depth: ${this.navigationStack.length}`);
    }

    /**
     * Navigate back to parent state machine
     */
    async navigateBack() {
        if (this.navigationStack.length === 0) {
            console.log('Already at root level');
            return;
        }

        console.log('Navigating back to parent');

        // Pop parent from stack
        const parent = this.navigationStack.pop();

        // Get container
        const containerId = 'state-diagram-single';
        const container = document.getElementById(containerId);
        if (!container) {
            console.error('state-diagram-single container not found');
            return;
        }

        // Clear existing visualization
        container.innerHTML = '';

        // Restore parent visualizer (re-create to ensure clean state)
        const parentVisualizer = new SCXMLVisualizer(containerId, parent.structure);

        // Restore current machine
        this.currentMachine = {
            id: parent.id,
            label: parent.label,
            structure: parent.structure,
            visualizer: parentVisualizer,
            subSCXMLs: parent.subSCXMLs
        };

        // Update breadcrumb UI
        this.updateBreadcrumb();

        // Hide back button if at root
        if (this.navigationStack.length === 0) {
            const backButton = document.getElementById('btn-back');
            if (backButton) {
                backButton.style.display = 'none';
            }
        }

        // Update state highlighting
        await this.updateState();

        console.log(`Navigation back complete. Stack depth: ${this.navigationStack.length}`);
    }

    /**
     * Navigate to specific depth in navigation stack
     * @param {number} depth - Target depth (0 = root)
     */
    async navigateToDepth(depth) {
        if (depth < 0 || depth > this.navigationStack.length) {
            console.error(`Invalid depth: ${depth}`);
            return;
        }

        // Navigate back multiple times to reach target depth
        const stepsBack = this.navigationStack.length - depth;
        for (let i = 0; i < stepsBack; i++) {
            await this.navigateBack();
        }
    }

    /**
     * Update breadcrumb navigation UI
     */
    updateBreadcrumb() {
        const breadcrumbContainer = document.getElementById('breadcrumb-container');
        if (!breadcrumbContainer) {
            console.warn('breadcrumb-container not found');
            return;
        }

        // Build breadcrumb path: [stack items] + current
        const path = [
            ...this.navigationStack.map(m => m.label),
            this.currentMachine.label
        ];

        // Render breadcrumb with separators
        const breadcrumbHTML = path.map((label, i) => {
            const isLast = (i === path.length - 1);
            if (isLast) {
                // Current (active) item
                return `<span class="breadcrumb-item active">${this.escapeHtml(label)}</span>`;
            } else {
                // Clickable parent items
                return `<a href="#" class="breadcrumb-item" data-depth="${i}">${this.escapeHtml(label)}</a>`;
            }
        }).join(' <span class="breadcrumb-separator">›</span> ');

        breadcrumbContainer.innerHTML = breadcrumbHTML;

        // Add click handlers for breadcrumb navigation
        breadcrumbContainer.querySelectorAll('a.breadcrumb-item').forEach(item => {
            item.addEventListener('click', async (e) => {
                e.preventDefault();
                const depth = parseInt(e.target.dataset.depth);
                await this.navigateToDepth(depth);
            });
        });

        console.log(`Breadcrumb updated: ${path.join(' > ')}`);
    }

    /**
     * Cleanup - remove event listeners and clear references
     */
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
}

