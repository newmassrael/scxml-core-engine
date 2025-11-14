// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * UI Updater - Handles UI updates for execution state
 */

class UIUpdater {
    constructor(controller) {
        this.controller = controller;
    }

    updateStepCounter() {
        if (this.controller.elements.stepCounter) {
            this.controller.elements.stepCounter.textContent = `Step ${this.controller.currentStep}`;
        }
    }

    getCurrentActiveStates() {
        try {
            const activeStatesVector = this.controller.runner.getActiveStates();
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

    updateEventQueue() {
        if (!this.controller.elements.eventQueuePanel) {
            console.warn('event-queue-panel not found');
            return;
        }

        try {
            const queue = this.controller.runner.getEventQueue();
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

            this.controller.elements.eventQueuePanel.innerHTML = html;
        } catch (error) {
            console.error('Error updating event queue:', error);
            this.controller.elements.eventQueuePanel.innerHTML = '<div class="error-message">Failed to load event queue</div>';
        }
    }

    updateScheduledEvents() {
        const panel = document.getElementById('scheduled-events-panel');
        if (!panel) {
            console.warn('scheduled-events-panel not found');
            return;
        }

        try {
            const scheduledEvents = this.controller.runner.getScheduledEvents();
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
                this.controller.startScheduledEventsTimer();
            } else {
                html = '<div class="event">No scheduled events</div>';

                // Stop timer when no scheduled events
                this.controller.stopScheduledEventsTimer();
            }

            panel.innerHTML = html;
        } catch (error) {
            console.error('Error updating scheduled events:', error);
            panel.innerHTML = '<div class="error-message">Failed to load scheduled events</div>';
        }
    }

    startScheduledEventsTimer() {
        if (this.controller.scheduledEventsTimer) {
            return; // Already running
        }

        this.controller.scheduledEventsTimer = setInterval(() => {
            const panel = document.getElementById('scheduled-events-panel');
            if (!panel) return;

            try {
                // JavaScript-side time calculation (WASM C++ time doesn't update in real-time)
                const jsNow = Date.now();
                const eventElements = panel.querySelectorAll('.event-scheduled');

                if (eventElements.length === 0) {
                    this.controller.stopScheduledEventsTimer();
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
                    const polledCount = this.controller.runner.pollScheduler();
                    if (polledCount > 0) {
                        console.log(`[Timer] Polled ${polledCount} events, updating UI`);
                        // Update event queue display to show moved events
                        this.controller.updateEventQueue();
                        // Refresh scheduled events display (events should be removed)
                        this.controller.updateScheduledEvents();
                    }
                }
            } catch (error) {
                console.error('Error in scheduled events timer:', error);
                this.controller.stopScheduledEventsTimer();
            }
        }, 100);
    }

    stopScheduledEventsTimer() {
        if (this.controller.scheduledEventsTimer) {
            clearInterval(this.controller.scheduledEventsTimer);
            this.controller.scheduledEventsTimer = null;
        }
    }

    updateDataModel() {
        if (!this.controller.elements.dataModelPanel) return;

        try {
            const dataModel = this.controller.runner.getDataModel();

            const entries = Object.entries(dataModel);

            if (entries.length === 0) {
                this.controller.elements.dataModelPanel.innerHTML = '<div class="data-entry">No data model variables</div>';
                return;
            }

            const html = entries.map(([key, value]) => `
                <div class="data-entry">
                    <span class="var-name">${key}</span>:
                    <span class="var-value">${this.controller.formatValue(value)}</span>
                </div>
            `).join('');

            this.controller.elements.dataModelPanel.innerHTML = html;
        } catch (error) {
            console.error('Error updating data model:', error);
            this.controller.elements.dataModelPanel.innerHTML = '<div class="error-message">Failed to load data model</div>';
        }
    }

    updateStateActions() {
        if (!this.controller.elements.stateActionsPanel) return;

        try {
            // Get all states from visualizer (show all, not just ones with actions)
            const allStates = this.controller.visualizer.states;

            if (allStates.length === 0) {
                this.controller.elements.stateActionsPanel.innerHTML = '<div class="action-info">No states defined</div>';
                return;
            }

            // Get active states for highlighting
            const activeStates = this.controller.getCurrentActiveStates();

            const html = allStates.map(state => {
                const isActive = activeStates.includes(state.id);
                let content = `<div class="action-info ${isActive ? 'active-state-info' : ''}" data-state-id="${this.controller.escapeHtml(state.id)}">`;

                // State header with type and active status (clickable)
                content += `<div class="action-state-header clickable-state-header" data-state-id="${this.controller.escapeHtml(state.id)}">`;
                content += `State: ${this.controller.escapeHtml(state.id)}`;
                content += ` <span class="state-type">(${this.controller.escapeHtml(state.type || 'atomic')}</span>`;
                if (isActive) {
                    content += `<span class="state-active">, active</span>`;
                }
                content += `)</div>`;

                // onentry actions
                if (state.onentry && state.onentry.length > 0) {
                    state.onentry.forEach(action => {
                        content += `<div class="action-item action-onentry">`;
                        content += `<span class="action-type">↓ ${this.controller.escapeHtml(action.actionType)}</span>`;
                        content += `<span class="action-details">${this.controller.formatAction(action).substring(2)}</span>`;
                        content += `</div>`;
                    });
                }

                // onexit actions
                if (state.onexit && state.onexit.length > 0) {
                    state.onexit.forEach(action => {
                        content += `<div class="action-item action-onexit">`;
                        content += `<span class="action-type">↑ ${this.controller.escapeHtml(action.actionType)}</span>`;
                        content += `<span class="action-details">${this.controller.formatAction(action).substring(2)}</span>`;
                        content += `</div>`;
                    });
                }

                // Show message if no actions
                if ((!state.onentry || state.onentry.length === 0) &&
                    (!state.onexit || state.onexit.length === 0)) {
                    content += `<div class="action-item action-none">No onentry/onexit actions</div>`;
                }

                // Outgoing transitions
                const outgoingTransitions = this.controller.visualizer.transitions.filter(t => t.source === state.id);
                if (outgoingTransitions.length > 0) {
                    content += `<div class="action-item action-transitions">`;
                    content += `<span class="action-type">→ Transitions:</span> `;
                    const transitionList = outgoingTransitions.map(t => {
                        const event = t.event || 'eventless';
                        return `${this.controller.escapeHtml(event)}→${this.controller.escapeHtml(t.target)}`;
                    }).join(', ');
                    content += `<span class="action-details">${transitionList}</span>`;
                    content += `</div>`;
                }

                content += `</div>`;
                return content;
            }).join('');

            this.controller.elements.stateActionsPanel.innerHTML = html;

            // Add click event listeners to state headers
            const stateHeaders = this.controller.elements.stateActionsPanel.querySelectorAll('.clickable-state-header');
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

                    this.controller.focusState(stateId);
                });
            });
        } catch (error) {
            console.error('Error updating state actions:', error);
            this.controller.elements.stateActionsPanel.innerHTML = '<div class="error-message">Failed to load state actions</div>';
        }
    }

    updateTransitionInfo(detail) {
        if (!this.controller.elements.transitionInfoPanel) return;

        try {
            if (!detail || !detail.source || !detail.target) {
                this.controller.elements.transitionInfoPanel.innerHTML = '<div class="transition-hint">Click a transition to view details</div>';
                return;
            }

            let html = '<div class="transition-details">';
            
            // Transition header: source → target
            html += `<div class="transition-header">${this.controller.escapeHtml(detail.source)} → ${this.controller.escapeHtml(detail.target)}</div>`;

            // Event field
            if (detail.event) {
                html += `<div class="transition-field">`;
                html += `<span class="transition-field-label">Event:</span>`;
                html += `<span class="transition-field-value">${this.controller.escapeHtml(detail.event)}</span>`;
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
                html += `<span class="transition-field-value">${this.controller.escapeHtml(detail.cond)}</span>`;
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
                    html += this.controller.formatAction(action);
                    html += `</div>`;
                });
                
                html += `</div>`;
            }

            html += '</div>';
            this.controller.elements.transitionInfoPanel.innerHTML = html;
        } catch (error) {
            console.error('Error updating transition info:', error);
            this.controller.elements.transitionInfoPanel.innerHTML = '<div class="error-message">Failed to load transition info</div>';
        }
    }

    updateLog() {
        if (!this.controller.elements.logPanel) return;

        try {
            const lastTransition = this.controller.runner.getLastTransition();

            if (!lastTransition || !lastTransition.source) {
                return;
            }

            const logEntry = document.createElement('div');
            logEntry.className = 'log-entry';

            const eventText = lastTransition.event || '(eventless)';

            logEntry.innerHTML = `
                <span class="log-step">[Step ${this.controller.currentStep}]</span>
                <span class="log-event">${eventText}</span>
                <span class="log-transition">${lastTransition.source} → ${lastTransition.target}</span>
                ${this.controller.getW3CReference(lastTransition)}
            `;

            // Prepend (newest first)
            this.controller.elements.logPanel.insertBefore(logEntry, this.controller.elements.logPanel.firstChild);

            // Limit log entries (keep last 100)
            while (this.controller.elements.logPanel.children.length > 100) {
                this.controller.elements.logPanel.removeChild(this.controller.elements.logPanel.lastChild);
            }
        } catch (error) {
            console.error('Error updating log:', error);
        }
    }

    async updateState() {
        try {
            // Update step counter
            this.updateStepCounter();

            // Get previous active states (stored from last update)
            const previousActiveStates = this.controller.previousActiveStates || [];

            // Update last transition animation BEFORE updating state diagram
            // (so we can still compare previousActiveStates vs currentActiveStates)
            // Skip transition animation during step backward (time travel doesn't show transitions)
            if (!this.controller.isSteppingBackward) {
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
                this.controller.highlightStateInPanel(stateToHighlight);
            }

        } catch (error) {
            console.error('Error updating state:', error);
        }
    }

    async updateStateDiagram() {
        try {
            const activeStatesVector = this.controller.runner.getActiveStates();

            // Convert Emscripten VectorString to JavaScript array
            const activeStates = [];
            for (let i = 0; i < activeStatesVector.size(); i++) {
                activeStates.push(activeStatesVector.get(i));
            }

            // Store previous active states for transition detection
            if (!this.controller.previousActiveStates) {
                this.controller.previousActiveStates = [];
            }

            this.controller.visualizer.highlightActiveStates(activeStates);

            // Update previous states for next comparison
            this.controller.previousActiveStates = [...activeStates];
        } catch (error) {
            console.error('Error updating state diagram:', error);
        }
    }

    async updateTransitionAnimation() {
        try {
            // Get transition from C++ (StateSnapshot restores this correctly)
            const lastTransition = this.controller.runner.getLastTransition();
            console.log('[UPDATE TRANSITION] getLastTransition() returned:', lastTransition);

            if (lastTransition && lastTransition.source && lastTransition.target) {
                console.log(`[UPDATE TRANSITION] Valid transition: ${lastTransition.source} → ${lastTransition.target}`);

                // Set active transition (permanent)
                this.controller.visualizer.setActiveTransition(lastTransition);
                // Update detail panel
                this.updateTransitionInfo(lastTransition);
                // Highlight in diagram (temporary)
                this.controller.visualizer.highlightTransition(lastTransition);
                console.log('[UPDATE TRANSITION] All transition updates complete');
            } else {
                console.log('[UPDATE TRANSITION] No transition to display (initial state or no state change)');
            }
        } catch (error) {
            console.error('Error animating transition:', error);
        }
    }
}
