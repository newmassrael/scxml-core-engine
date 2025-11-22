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
            logger.warn('event-queue-panel not found');
            return;
        }

        try {
            const queue = this.controller.runner.getEventQueue();
            logger.debug('[DEBUG] getEventQueue() returned:', queue);
            logger.debug('[DEBUG] queue type:', typeof queue);
            logger.debug('[DEBUG] queue.external:', queue.external);

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
            logger.warn('scheduled-events-panel not found');
            return;
        }

        try {
            const scheduledEvents = this.controller.runner.getScheduledEvents();
            logger.debug('[DEBUG] getScheduledEvents() returned:', scheduledEvents);

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
                    logger.debug('[Timer] Events ready, polling scheduler to move to queue');
                    const polledCount = this.controller.runner.pollScheduler();
                    if (polledCount > 0) {
                        logger.debug(`[Timer] Polled ${polledCount} events, updating UI`);
                        // Update event queue display to show moved events
                        this.controller.updateEventQueue();
                        // Refresh scheduled events display (events should be removed)
                        this.controller.updateScheduledEvents();
                        // CRITICAL FIX: Enable step forward button when events are available
                        this.controller.enableButton('btn-step-forward');
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

                // onentry actions (using ActionFormatter for consistent display)
                if (state.onentry && state.onentry.length > 0) {
                    state.onentry.forEach(action => {
                        const formatted = ActionFormatter.formatAction(action);
                        content += `<div class="action-item action-onentry">`;
                        content += `<span class="action-type">↓ entry</span>`;
                        content += `<div class="action-details">${formatted.main}</div>`;
                        // Add detail lines for send actions with content, params, etc.
                        if (formatted.details && formatted.details.length > 0) {
                            formatted.details.forEach(detail => {
                                content += `<div class="action-detail-line">${detail}</div>`;
                            });
                        }
                        content += `</div>`;
                    });
                }

                // onexit actions (using ActionFormatter for consistent display)
                if (state.onexit && state.onexit.length > 0) {
                    state.onexit.forEach(action => {
                        const formatted = ActionFormatter.formatAction(action);
                        content += `<div class="action-item action-onexit">`;
                        content += `<span class="action-type">↑ exit</span>`;
                        content += `<div class="action-details">${formatted.main}</div>`;
                        // Add detail lines for send actions with content, params, etc.
                        if (formatted.details && formatted.details.length > 0) {
                            formatted.details.forEach(detail => {
                                content += `<div class="action-detail-line">${detail}</div>`;
                            });
                        }
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
        // No-op: Transition detail panel removed (only list panel remains)
        // Transition info is shown in the transition-list-panel via setActiveTransition()
    }

    updateLog() {
        if (!this.controller.elements.logPanel) return;

        try {
            const lastTransition = this.controller.runner.getLastTransition();

            if (!lastTransition || !lastTransition.source) {
                return;
            }

            // W3C SCXML 3.7: Find transition actions from structure (getLastTransition doesn't include actions)
            // Use currentMachine.structure.transitions to support both parent and child SCXMLs
            const transitions = this.controller.currentMachine?.structure?.transitions || [];
            const matchingTransition = transitions.find(t =>
                t.source === lastTransition.source &&
                t.target === lastTransition.target &&
                (t.event === lastTransition.event || (!t.event && !lastTransition.event))
            );

            const logEntry = document.createElement('div');
            logEntry.className = 'log-entry';

            const eventText = lastTransition.event || '(eventless)';

            // W3C SCXML 3.13: Transition executable content display
            let actionsHtml = '';
            const actions = matchingTransition?.actions || lastTransition.actions;
            if (actions && actions.length > 0) {
                const actionList = actions.map(action => {
                    const actionType = action.actionType || action.type;  // Support both field names
                    if (actionType === 'send') {
                        let sendText = `send(${action.event || '?'})`;

                        // Add compact data indicator
                        if (action.content !== undefined && action.content !== null && action.content !== '') {
                            const contentStr = String(action.content);
                            const truncated = contentStr.length > 15 ? contentStr.substring(0, 15) + '...' : contentStr;
                            sendText += `[${truncated}]`;
                        } else if (action.params && action.params.length > 0) {
                            sendText += `[params]`;
                        } else if (action.namelist) {
                            sendText += `[vars]`;
                        }

                        if (action.target && action.target !== '#_internal') {
                            sendText += ` to ${action.target}`;
                        }
                        return sendText;
                    } else if (actionType === 'raise') {
                        return `📢 Raise: ${action.event || '?'}`;
                    } else if (actionType === 'assign') {
                        return `💾 Assign: ${action.location || '?'} = ${action.expr || '?'}`;
                    } else if (actionType === 'log') {
                        return `📝 Log: ${action.label || action.expr || '?'}`;
                    } else if (actionType === 'cancel') {
                        return `🚫 Cancel: ${action.sendid || action.sendidexpr || '?'}`;
                    } else {
                        return actionType || 'unknown';
                    }
                }).join(', ');
                actionsHtml = `<br><span class="log-actions">  → Execute: ${actionList}</span>`;
            }

            logEntry.innerHTML = `
                <span class="log-step">[Step ${this.controller.currentStep}]</span>
                <span class="log-event">${eventText}</span>
                <span class="log-transition">${lastTransition.source} → ${lastTransition.target}</span>
                ${actionsHtml}
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
                logger.debug(`[Auto Highlight] State activated: ${stateToHighlight}`);
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

            logger.debug(`[updateStateDiagram] getActiveStates() returned: [${activeStates.join(', ')}]`);

            // W3C SCXML Appendix D.2: Get conflict resolution data for visualization
            try {
                const enabledTransitionsData = this.controller.runner.getEnabledTransitions();
                const optimalTransitionsData = this.controller.runner.getOptimalTransitions();

                // Convert to JavaScript arrays (handles both Emscripten val::array and native arrays)
                const enabledTransitions = [];
                const enabledLen = enabledTransitionsData.length ?? enabledTransitionsData.size?.();
                for (let i = 0; i < enabledLen; i++) {
                    const trans = enabledTransitionsData[i] ?? enabledTransitionsData.get?.(i);
                    enabledTransitions.push({
                        source: trans.source,
                        target: trans.target,
                        event: trans.event,
                        isInternal: trans.isInternal,
                        isExternal: trans.isExternal
                    });
                }

                const optimalTransitions = [];
                const optimalLen = optimalTransitionsData.length ?? optimalTransitionsData.size?.();
                for (let i = 0; i < optimalLen; i++) {
                    const trans = optimalTransitionsData[i] ?? optimalTransitionsData.get?.(i);
                    optimalTransitions.push({
                        source: trans.source,
                        target: trans.target,
                        isInternal: trans.isInternal,
                        isExternal: trans.isExternal
                    });
                }

                logger.debug(`[Conflict Resolution] Enabled: ${enabledTransitions.length}, Optimal: ${optimalTransitions.length}`);
                this.controller.visualizer.updateConflictResolutionData(enabledTransitions, optimalTransitions);
            } catch (error) {
                // Silently fail if methods not available (backwards compatibility)
                logger.debug('[Conflict Resolution] Methods not available:', error.message);
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
            logger.debug('[UPDATE TRANSITION] getLastTransition() returned:', lastTransition);

            if (lastTransition && lastTransition.source && lastTransition.target) {
                logger.debug(`[UPDATE TRANSITION] Valid transition: ${lastTransition.source} → ${lastTransition.target}`);

                // Note: Auto-expansion is handled by InteractionHandler.highlightActiveStates()

                // Set active transition (permanent)
                this.controller.visualizer.setActiveTransition(lastTransition);
                // Highlight in diagram (temporary)
                this.controller.visualizer.highlightTransition(lastTransition);
                logger.debug('[UPDATE TRANSITION] All transition updates complete');
            } else {
                logger.debug('[UPDATE TRANSITION] No transition to display (initial state or no state change)');
            }
        } catch (error) {
            console.error('Error animating transition:', error);
        }
    }
}
