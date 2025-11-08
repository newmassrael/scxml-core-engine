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
class ExecutionController {
    constructor(wasmRunner, visualizer, availableEvents = []) {
        this.runner = wasmRunner;
        this.visualizer = visualizer;
        this.currentStep = 0;
        this.maxSteps = 0;
        this.availableEvents = availableEvents;

        this.setupControls();
        this.setupEventButtons();
    }

    /**
     * Setup control buttons and keyboard shortcuts
     */
    setupControls() {
        // Step buttons
        const btnStepBack = document.getElementById('btn-step-back');
        const btnStepForward = document.getElementById('btn-step-forward');
        const btnReset = document.getElementById('btn-reset');

        if (btnStepBack) {
            btnStepBack.onclick = () => this.stepBackward();
        }

        if (btnStepForward) {
            btnStepForward.onclick = () => this.stepForward();
        }

        if (btnReset) {
            btnReset.onclick = () => this.reset();
        }

        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
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
        });

        console.log('✅ Execution controls initialized (Arrow keys: step, R: reset)');
    }

    /**
     * Setup event buttons from available events
     */
    setupEventButtons() {
        const eventButtonsContainer = document.getElementById('event-buttons-container');
        if (!eventButtonsContainer) return;

        // Clear existing content
        eventButtonsContainer.innerHTML = '';

        if (this.availableEvents.length === 0) {
            eventButtonsContainer.innerHTML = '<div class="text-small text-gray">No events available</div>';
            return;
        }

        // Create button for each event
        this.availableEvents.forEach(eventName => {
            const button = document.createElement('button');
            button.className = 'btn btn-sm event-button';
            button.textContent = eventName;
            button.onclick = () => this.raiseEvent(eventName);
            eventButtonsContainer.appendChild(button);
        });

        console.log(`✅ Created ${this.availableEvents.length} event buttons`);
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

            console.log(`✅ Step ${this.currentStep} executed`);
        } catch (error) {
            console.error('❌ Error during stepForward:', error);
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
            await this.updateState();

            // Re-enable forward button
            this.enableButton('btn-step-forward');

            console.log(`⬅️ Restored to step ${this.currentStep}`);
        } catch (error) {
            console.error('❌ Error during stepBackward:', error);
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
            await this.updateState();

            // Re-enable forward button
            this.enableButton('btn-step-forward');

            // Clear log
            const logPanel = document.getElementById('log-panel');
            if (logPanel) {
                logPanel.innerHTML = '<div class="log-entry">Reset to initial configuration</div>';
            }

            console.log('🔄 Reset to initial state');
        } catch (error) {
            console.error('❌ Error during reset:', error);
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
            console.error('❌ Error raising event:', error);
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

            // Update state diagram
            await this.updateStateDiagram();

            // Update last transition animation
            await this.updateTransitionAnimation();

            // Update panels
            this.updateEventQueue();
            this.updateDataModel();
            this.updateLog();

        } catch (error) {
            console.error('❌ Error updating state:', error);
        }
    }

    /**
     * Update step counter display
     */
    updateStepCounter() {
        const counter = document.getElementById('step-counter');
        if (counter) {
            counter.textContent = `Step ${this.currentStep}`;
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

            this.visualizer.highlightActiveStates(activeStates);
        } catch (error) {
            console.error('❌ Error updating state diagram:', error);
        }
    }

    /**
     * Animate last transition
     */
    async updateTransitionAnimation() {
        try {
            const lastTransition = this.runner.getLastTransition();

            if (lastTransition && lastTransition.source && lastTransition.target) {
                this.visualizer.animateTransition(lastTransition);
            }
        } catch (error) {
            console.error('❌ Error animating transition:', error);
        }
    }

    /**
     * Update event queue panel
     */
    updateEventQueue() {
        const panel = document.getElementById('event-queue-panel');
        if (!panel) {
            console.warn('❌ event-queue-panel not found');
            return;
        }

        try {
            const queue = this.runner.getEventQueue();
            console.log('🔍 [DEBUG] getEventQueue() returned:', queue);
            console.log('🔍 [DEBUG] queue type:', typeof queue);
            console.log('🔍 [DEBUG] queue.external:', queue.external);

            let html = '<h4>Internal Queue</h4>';
            if (queue.internal && queue.internal.length > 0) {
                html += queue.internal.map(e =>
                    `<div class="event event-internal">${e.name || e}</div>`
                ).join('');
            } else {
                html += '<div class="event">Empty</div>';
            }

            html += '<h4 style="margin-top: 16px;">External Queue</h4>';
            if (queue.external && queue.external.length > 0) {
                html += queue.external.map(e =>
                    `<div class="event event-external">${e.name || e}</div>`
                ).join('');
            } else {
                html += '<div class="event">Empty</div>';
            }

            panel.innerHTML = html;
        } catch (error) {
            console.error('❌ Error updating event queue:', error);
            panel.innerHTML = '<div class="error-message">Failed to load event queue</div>';
        }
    }

    /**
     * Update data model panel
     */
    updateDataModel() {
        const panel = document.getElementById('data-model-panel');
        if (!panel) return;

        try {
            const dataModel = this.runner.getDataModel();

            const entries = Object.entries(dataModel);

            if (entries.length === 0) {
                panel.innerHTML = '<div class="data-entry">No data model variables</div>';
                return;
            }

            const html = entries.map(([key, value]) => `
                <div class="data-entry">
                    <span class="var-name">${key}</span>:
                    <span class="var-value">${this.formatValue(value)}</span>
                </div>
            `).join('');

            panel.innerHTML = html;
        } catch (error) {
            console.error('❌ Error updating data model:', error);
            panel.innerHTML = '<div class="error-message">Failed to load data model</div>';
        }
    }

    /**
     * Update execution log
     */
    updateLog() {
        const panel = document.getElementById('log-panel');
        if (!panel) return;

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
            panel.insertBefore(logEntry, panel.firstChild);

            // Limit log entries (keep last 100)
            while (panel.children.length > 100) {
                panel.removeChild(panel.lastChild);
            }
        } catch (error) {
            console.error('❌ Error updating log:', error);
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
            console.error('❌ Error getting W3C reference:', error);
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
     * Show user message
     */
    showMessage(message, type = 'info') {
        console.log(`${type.toUpperCase()}: ${message}`);

        // TODO: Add toast notification UI
        // For now, just console log
    }

    /**
     * Disable button
     */
    disableButton(buttonId) {
        const button = document.getElementById(buttonId);
        if (button) {
            button.disabled = true;
            button.classList.add('disabled');
        }
    }

    /**
     * Enable button
     */
    enableButton(buttonId) {
        const button = document.getElementById(buttonId);
        if (button) {
            button.disabled = false;
            button.classList.remove('disabled');
        }
    }
}
