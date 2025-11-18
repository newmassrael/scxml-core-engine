// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Control Handler - Handles user controls and event buttons
 */

class ControlHandler {
    constructor(controller) {
        this.controller = controller;
    }

    setupControls() {
        // Step buttons
        if (this.controller.elements.btnStepBack) {
            this.controller.elements.btnStepBack.onclick = () => this.controller.stepBackward();
        }

        if (this.controller.elements.btnStepForward) {
            this.controller.elements.btnStepForward.onclick = () => this.controller.stepForward();
        }

        if (this.controller.elements.btnReset) {
            this.controller.elements.btnReset.onclick = () => this.controller.reset();
        }

        // Back button for navigation (hidden by default)
        const backButton = document.getElementById('btn-back');
        if (backButton) {
            backButton.onclick = () => this.controller.navigateBack();
            backButton.style.display = 'none';  // Hidden until navigate to child
        }

        // Keyboard shortcuts
        this.controller.keyboardHandler = (e) => {
            // Don't capture if typing in input fields
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
                return;
            }

            if (e.key === 'ArrowLeft') {
                e.preventDefault();
                this.controller.stepBackward();
            } else if (e.key === 'ArrowRight') {
                e.preventDefault();
                this.controller.stepForward();
            } else if (e.key === 'r' || e.key === 'R') {
                e.preventDefault();
                this.controller.reset();
            } else if (e.key === 'b' || e.key === 'B' || e.key === 'Backspace') {
                // Back to parent state machine (if navigated to child)
                if (this.controller.navigationStack.length > 0) {
                    e.preventDefault();
                    this.controller.navigateBack();
                }
            }
        };
        
        document.addEventListener('keydown', this.controller.keyboardHandler);

        // W3C SCXML 3.3: Transition click event handler (diagram -> list panel sync)
        this.controller.transitionClickHandler = (event) => {
            // When diagram transition is clicked, highlight in list panel (temporary)
            if (event.detail && event.detail.source && event.detail.target) {
                this.controller.highlightTransitionInPanel(event.detail);
            }
        };
        document.addEventListener('transition-click', this.controller.transitionClickHandler);

        // W3C SCXML 6.3: State navigate event handler for invoke child SCXML
        this.controller.stateNavigateHandler = async (event) => {
            const {stateId, invokeSrc, invokeSrcExpr, invokeId} = event.detail;
            logger.debug(`State navigate event received: ${stateId} -> ${invokeSrc || invokeSrcExpr}`);
            
            // Load child SCXML structure
            await this.controller.handleStateNavigation(stateId, invokeSrc, invokeSrcExpr, invokeId);
        };
        document.addEventListener('state-navigate', this.controller.stateNavigateHandler);

        logger.debug('Execution controls initialized (Arrow keys: step, R: reset)');
    }

    setupEventButtons() {
        if (!this.controller.elements.eventButtonsContainer) return;

        // Clear existing content
        this.controller.elements.eventButtonsContainer.innerHTML = '';

        if (this.controller.availableEvents.length === 0) {
            this.controller.elements.eventButtonsContainer.innerHTML = '<div class="text-small text-gray">No events available</div>';
            return;
        }

        // Create button for each event
        this.controller.availableEvents.forEach(eventName => {
            const button = document.createElement('button');
            button.className = 'btn btn-sm event-button';
            button.textContent = eventName;
            button.onclick = () => this.controller.raiseEvent(eventName);
            this.controller.elements.eventButtonsContainer.appendChild(button);
        });

        logger.debug(`Created ${this.controller.availableEvents.length} event buttons`);
    }

    checkAndHandleFinalState() {
        if (this.controller.runner.isInFinalState()) {
            this.controller.disableButton('btn-step-forward');
            const activeStates = this.controller.runner.getActiveStates();
            const stateList = activeStates.length > 0 ? activeStates.join(', ') : 'unknown';
            logger.debug(`State machine in final state: ${stateList}`);
        } else {
            this.controller.enableButton('btn-step-forward');
        }
    }

    focusState(stateId) {
        if (!stateId || !this.controller.visualizer) return;

        try {
            logger.debug(`[Focus State] Focusing on state: ${stateId}`);

            // Single-window navigation: Only check current diagram container
            const container = d3.select('#state-diagram-single');
            const foundStateNode = container.empty() ? null : container.select(`[data-state-id="${stateId}"]`);
            const foundInContainer = foundStateNode && !foundStateNode.empty() ? container : null;

            if (!foundStateNode || foundStateNode.empty()) {
                logger.warn(`[Focus State] State not found in any container: ${stateId}`);
                return;
            }

            logger.debug(`[Focus State] Found state element: ${stateId}`);

            // Visual feedback: add blue border effect (like Transition Info)
            // Select all state elements: g.node.state, compound-collapsed, compound-container
            const stateElements = foundInContainer.selectAll('.node.state, .compound-collapsed, .compound-container');
            stateElements.classed('focused', d => {
                const isFocused = d && d.id === stateId;
                if (isFocused) {
                    logger.debug(`[Focus State] Adding focused class to: ${stateId}`);
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
                logger.debug(`[Focus State] Scrolling state into view`);
            }

            // Remove focus after animation duration
            setTimeout(() => {
                stateElements.classed('focused', false);
                logger.debug(`[Focus State] Removed focused class from all states`);
            }, FOCUS_HIGHLIGHT_DURATION);
        } catch (error) {
            console.error('Error focusing state:', error);
        }
    }

    highlightStateInPanel(stateId) {
        if (!stateId) return;

        try {
            logger.debug(`[Highlight Panel] Highlighting state in panel: ${stateId}`);

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
                    
                    logger.debug(`[Highlight Panel] Added highlight to: ${stateId}`);
                }
            });

            // Remove highlight after animation duration
            setTimeout(() => {
                stateInfoBlocks.forEach(block => {
                    block.classList.remove('panel-highlighted');
                });
                logger.debug(`[Highlight Panel] Removed highlight from panel`);
            }, PANEL_HIGHLIGHT_DURATION);
        } catch (error) {
            console.error('Error highlighting state in panel:', error);
        }
    }

    highlightTransitionInPanel(transition) {
        if (!transition || !transition.source || !transition.target) return;

        try {
            // Use shared utility function from utils.js (Single Source of Truth)
            const transitionId = getTransitionId(transition);
            logger.debug(`[Highlight Panel] Highlighting transition in panel: ${transitionId}`);

            const panel = document.getElementById('transition-list-panel');
            if (!panel) return;

            // Find the transition list item
            const transitionItems = panel.querySelectorAll('.transition-list-item');
            
            // Remove previous highlights
            transitionItems.forEach(item => {
                item.classList.remove('panel-highlighted');
            });

            // Add highlight to the clicked transition's item
            const targetItem = panel.querySelector(`[data-transition-id="${transitionId}"]`);
            if (targetItem) {
                targetItem.classList.add('panel-highlighted');
                
                // Scroll into view (matches State Actions panel behavior)
                targetItem.scrollIntoView({
                    behavior: 'smooth',
                    block: 'nearest'
                });
                
                logger.debug(`[Highlight Panel] Added highlight to: ${transitionId}`);
            }

            // Remove highlight after animation duration
            setTimeout(() => {
                transitionItems.forEach(item => {
                    item.classList.remove('panel-highlighted');
                });
                logger.debug(`[Highlight Panel] Removed highlight from transition panel`);
            }, PANEL_HIGHLIGHT_DURATION);
        } catch (error) {
            console.error('Error highlighting transition in panel:', error);
        }
    }

    showMessage(message, type = 'info') {
        logger.debug(`[${type.toUpperCase()}] ${message}`);
    }

    disableButton(buttonId) {
        const button = this.controller.elements[this.controller.getElementKeyFromId(buttonId)];
        if (button) {
            button.disabled = true;
            button.classList.add('disabled');
        }
    }

    enableButton(buttonId) {
        const button = this.controller.elements[this.controller.getElementKeyFromId(buttonId)];
        if (button) {
            button.disabled = false;
            button.classList.remove('disabled');
        }
    }

    async stepForward() {
        try {
            const success = this.controller.runner.stepForward();

            if (!success) {
                this.showMessage('Send an event first (use event buttons below)', 'info');
                this.disableButton('btn-step-forward');
                return;
            }

            this.controller.currentStep = this.controller.runner.getCurrentStep();

            // Set active transition (last executed) and update detail panel
            const executedTransition = this.controller.runner.getLastTransition();
            if (executedTransition && executedTransition.source && executedTransition.target) {
                this.controller.visualizer.setActiveTransition(executedTransition);
                this.controller.visualizer.highlightTransition(executedTransition);
            }

            await this.controller.updateState();

            // Update event queue after processing
            this.controller.updateEventQueue();

            // Check if reached final state and disable button if needed
            this.checkAndHandleFinalState();

            logger.debug(`Step ${this.controller.currentStep} executed`);
        } catch (error) {
            console.error('Error during stepForward:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    async stepBackward() {
        try {
            // Immediately cancel ongoing animations for instant UI response
            this.controller.visualizer.clearTransitionHighlights();

            const success = this.controller.runner.stepBackward();

            if (!success) {
                this.showMessage('Already at initial state', 'info');
                return;
            }

            this.controller.currentStep = this.controller.runner.getCurrentStep();

            // C++ StateSnapshot automatically restores transition info
            const restoredTransition = this.controller.runner.getLastTransition();
            logger.debug(`[STEP BACK] Restored to step ${this.controller.currentStep}, transition:`, restoredTransition);

            if (restoredTransition && restoredTransition.source && restoredTransition.target) {
                logger.debug(`[STEP BACK] Setting active transition: ${restoredTransition.source} → ${restoredTransition.target}`);
                this.controller.visualizer.setActiveTransition(restoredTransition);
                this.controller.visualizer.highlightTransition(restoredTransition);
            } else {
                logger.debug(`[STEP BACK] No transition at step ${this.controller.currentStep}, clearing active state`);
                this.controller.visualizer.clearActiveTransition();
            }

            // Set flag to skip transition animation during backward step
            this.controller.isSteppingBackward = true;
            await this.controller.updateState();
            this.controller.isSteppingBackward = false;

            // Re-enable forward button
            this.enableButton('btn-step-forward');

            logger.debug(`Restored to step ${this.controller.currentStep}`);
        } catch (error) {
            console.error('Error during stepBackward:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    async reset() {
        try {
            this.controller.runner.reset();
            this.controller.currentStep = 0;

            // Clear active transition state (reset to initial, no transitions executed)
            this.controller.visualizer.clearActiveTransition();

            await this.controller.updateState();

            // Check if in final state and update button accordingly
            this.checkAndHandleFinalState();

            // Clear log
            if (this.controller.elements.logPanel) {
                this.controller.elements.logPanel.innerHTML = '<div class="log-entry">Reset to initial configuration</div>';
            }

            logger.debug('Reset to initial state');
        } catch (error) {
            console.error('Error during reset:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    async raiseEvent(eventName, eventData = '') {
        try {
            this.controller.runner.raiseEvent(eventName, eventData);

            // W3C SCXML 3.13: Event queuing is NOT a step
            logger.debug(`📨 Event queued: ${eventName} (press Step Forward to process)`);

            // Update UI to show pending event in queue
            this.controller.updateEventQueue();

            // Enable Step Forward button
            this.enableButton('btn-step-forward');

            this.showMessage(`Event "${eventName}" added to queue`, 'info');
        } catch (error) {
            console.error('Error raising event:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    async removeInternalEvent(index) {
        try {
            const success = this.controller.runner.removeInternalEvent(index);

            if (success) {
                logger.debug(`🗑️ Internal event at index ${index} removed`);
                this.controller.updateEventQueue();
                this.controller.updateState();
                this.showMessage(`Internal event removed`, 'info');
            } else {
                logger.warn(`⚠️ Failed to remove internal event at index ${index}`);
                this.showMessage(`Invalid event index`, 'warning');
            }
        } catch (error) {
            console.error('Error removing internal event:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }

    async removeExternalEvent(index) {
        try {
            const success = this.controller.runner.removeExternalEvent(index);

            if (success) {
                logger.debug(`🗑️ External event at index ${index} removed`);
                this.controller.updateEventQueue();
                this.controller.updateState();
                this.showMessage(`External event removed`, 'info');
            } else {
                logger.warn(`⚠️ Failed to remove external event at index ${index}`);
                this.showMessage(`Invalid event index`, 'warning');
            }
        } catch (error) {
            console.error('Error removing external event:', error);
            this.showMessage(`Error: ${error.message}`, 'error');
        }
    }
}
