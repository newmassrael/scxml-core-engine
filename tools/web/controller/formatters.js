// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Formatters - Formatting utilities for execution controller
 */

class Formatters {
    constructor(controller) {
        this.controller = controller;
    }

    escapeHtml(unsafe) {
        if (!unsafe) return '';
        return String(unsafe)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }

    appendOptionalAttributes(text, action, attrNames) {
        let result = text;
        for (const attrName of attrNames) {
            if (action[attrName]) {
                result += ` ${attrName}: ${this.controller.escapeHtml(action[attrName])}`;
            }
        }
        return result;
    }

    formatAction(action) {
        let text = `• ${this.controller.escapeHtml(action.actionType)}`;

        if (action.actionType === 'raise') {
            text += ` event: ${this.controller.escapeHtml(action.event)}`;
        } else if (action.actionType === 'assign') {
            text += ` ${this.controller.escapeHtml(action.location)} = ${this.controller.escapeHtml(action.expr)}`;
        } else if (action.actionType === 'log') {
            text = this.controller.appendOptionalAttributes(text, action, ['label', 'expr', 'level']);        } else if (action.actionType === 'foreach') {
            text += ` item: ${this.controller.escapeHtml(action.item || 'none')}`;
            // Optional attributes
            if (action.index) text += `, index: ${this.controller.escapeHtml(action.index)}`;
            if (action.array) text += `, array: ${this.controller.escapeHtml(action.array)}`;        } else if (action.actionType === 'send') {
            // W3C SCXML 6.2: Display comprehensive send attributes
            text = this.controller.appendOptionalAttributes(text, action, [
                'event', 'eventexpr', 'target', 'targetexpr',
                'delay', 'delayexpr', 'type', 'namelist',
                'sendid', 'idlocation', 'data', 'contentexpr'
            ]);
            // Special handling for content (preview with truncation)
            if (action.content) {
                const preview = action.content.substring(0, 50);
                text += ` content: ${this.controller.escapeHtml(preview)}${action.content.length > 50 ? '...' : ''}`;
            }            if (action.params && action.params.length > 0) {
                text += ` params: [${action.params.map(p => `${this.controller.escapeHtml(p.name)}=${this.controller.escapeHtml(p.expr)}`).join(', ')}]`;
            }
        } else if (action.actionType === 'if') {
            // W3C SCXML 3.12.1: Display if condition and branches
            if (action.cond) text += ` cond: ${this.controller.escapeHtml(action.cond)}`;
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
            text = this.controller.appendOptionalAttributes(text, action, ['sendid', 'sendidexpr']);        } else if (action.actionType === 'script') {
            // W3C SCXML 5.9: Display script content preview
            if (action.content) {
                const preview = action.content.substring(0, 50);
                text += ` ${this.controller.escapeHtml(preview)}${action.content.length > 50 ? '...' : ''}`;
            }
        }

        return text;
    }

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

    formatValue(value) {
        if (typeof value === 'string') {
            return `"${value}"`;
        }
        if (typeof value === 'object') {
            return JSON.stringify(value);
        }
        return String(value);
    }

    getElementKeyFromId(id) {
        const idToKeyMap = {
            'btn-step-back': 'btnStepBack',
            'btn-step-forward': 'btnStepForward',
            'btn-reset': 'btnReset'
        };
        return idToKeyMap[id] || id;
    }
}
