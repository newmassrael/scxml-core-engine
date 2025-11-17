/**
 * Action formatter utilities for SCXML visualizer
 * Provides smart, context-aware formatting for SCXML actions (send, raise, assign, etc.)
 */

const ActionFormatter = (function() {
    'use strict';

    // Truncation limits for display optimization
    const TRUNCATION = {
        CONTENT: 30,      // W3C SCXML 6.2.3 - content display limit for readability
        PARAMS: 40,       // W3C SCXML 6.2.4 - params display limit (can show more data)
        EXPRESSION: 30,   // General expression display limit (assign, log, etc.)
        LOG_TEXT: 30      // Log message display limit
    };

    // Detail line prefix for hierarchical display
    const DETAIL_PREFIX = '   ↳ ';

    /**
     * Check if a value is empty (null, undefined, or empty string)
     * @param {*} value - Value to check
     * @returns {boolean} True if value is empty
     */
    function isEmpty(value) {
        return value === undefined || value === null || value === '';
    }

    /**
     * Check if an array has items
     * @param {Array} arr - Array to check
     * @returns {boolean} True if array exists and has items
     */
    function hasItems(arr) {
        return arr && Array.isArray(arr) && arr.length > 0;
    }

    /**
     * Format send action main line
     * @param {Object} action - Send action object
     * @returns {string} Formatted main line (e.g., "📤 Send event: event1")
     */
    function formatSendAction(action) {
        const icon = '📤';
        const eventName = action.event || action.eventexpr || '?';
        const isDynamic = !action.event && action.eventexpr;

        if (isDynamic) {
            return `${icon} Send event: ${eventName} (dynamic)`;
        }
        return `${icon} Send event: ${eventName}`;
    }

    /**
     * Format send action detail lines
     * @param {Object} action - Send action object
     * @returns {Array<string>} Array of detail lines (e.g., ["↳ content: 123", "↳ delay: 5s"])
     */
    function formatSendDetails(action) {
        const details = [];

        // W3C SCXML 6.2.3: <send> content attribute for message payload
        if (!isEmpty(action.content)) {
            const contentStr = String(action.content);
            const truncated = contentStr.length > TRUNCATION.CONTENT
                ? contentStr.substring(0, TRUNCATION.CONTENT) + '...'
                : contentStr;
            details.push(`${DETAIL_PREFIX}content: ${truncated}`);
        } else if (!isEmpty(action.contentexpr)) {
            details.push(`${DETAIL_PREFIX}contentexpr: ${action.contentexpr}`);
        }

        if (!isEmpty(action.data)) {
            details.push(`${DETAIL_PREFIX}data: ${action.data}`);
        }

        // W3C SCXML 6.2.4: <param> elements for name-value pairs in send
        if (hasItems(action.params)) {
            const paramStrs = action.params.map(p => `${p.name}=${p.expr || '?'}`);
            const paramsStr = paramStrs.join(', ');
            const truncated = paramsStr.length > TRUNCATION.PARAMS
                ? paramsStr.substring(0, TRUNCATION.PARAMS) + '...'
                : paramsStr;
            details.push(`${DETAIL_PREFIX}params: ${truncated}`);
    }

        // W3C SCXML 6.2.5: namelist attribute for variable names to send
        if (!isEmpty(action.namelist)) {
            details.push(`${DETAIL_PREFIX}namelist: ${action.namelist}`);
        }

        // W3C SCXML 6.2.1: target attribute for event destination (only show if not default)
        const target = action.target || action.targetexpr;
        if (!isEmpty(target) && target !== '#_internal') {
            const isDynamic = isEmpty(action.target) && !isEmpty(action.targetexpr);
            const suffix = isDynamic ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}target: ${target}${suffix}`);
        }

        // W3C SCXML 6.2.6: delay/delayexpr attributes for delayed event delivery (static or dynamic)
        const delay = action.delay || action.delayexpr;
        if (!isEmpty(delay)) {
            const isDynamic = isEmpty(action.delay) && !isEmpty(action.delayexpr);
            const suffix = isDynamic ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}delay: ${delay}${suffix}`);
        }

        // W3C SCXML 6.2.2: type attribute for Event I/O Processor selection (only show if not default)
        const type = action.type || action.typeexpr;
        if (!isEmpty(type) && type !== 'scxml' && type !== 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor') {
            const isDynamic = isEmpty(action.type) && !isEmpty(action.typeexpr);
            const suffix = isDynamic ? ' (dynamic)' : '';
            // Simplify BasicHTTP processor type for display
            const displayType = type === 'http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor' ? 'BasicHTTP' : type;
            details.push(`${DETAIL_PREFIX}type: ${displayType}${suffix}`);
        }

        // W3C SCXML 6.2.7: sendid attribute for unique event identification and cancellation
        if (!isEmpty(action.sendid)) {
            details.push(`${DETAIL_PREFIX}id: ${action.sendid}`);
        } else if (!isEmpty(action.idlocation)) {
            details.push(`${DETAIL_PREFIX}idlocation: ${action.idlocation}`);
        }

        return details;
    }

    /**
     * Format raise action
     * @param {Object} action - Raise action object
     * @returns {string} Formatted action text
     */
    function formatRaiseAction(action) {
        const icon = '📢';
        const eventName = action.event || '?';
        return `${icon} Raise: ${eventName}`;
    }

    /**
     * Format assign action
     * @param {Object} action - Assign action object
     * @returns {string} Formatted action text
     */
    function formatAssignAction(action) {
        const icon = '💾';
        const location = action.location || '?';
        const expr = action.expr || '?';
        const truncatedExpr = expr.length > TRUNCATION.EXPRESSION
            ? expr.substring(0, TRUNCATION.EXPRESSION) + '...'
            : expr;
        return `${icon} Assign: ${location} = ${truncatedExpr}`;
    }

    /**
     * Format log action
     * @param {Object} action - Log action object
     * @returns {string} Formatted action text
     */
    function formatLogAction(action) {
        const text = action.label || action.expr || '?';
        const truncated = text.length > TRUNCATION.LOG_TEXT
            ? text.substring(0, TRUNCATION.LOG_TEXT) + '...'
            : text;
        return `📝 Log: ${truncated}`;
    }

    /**
     * Format cancel action
     * @param {Object} action - Cancel action object
     * @returns {string} Formatted action text
     */
    function formatCancelAction(action) {
        const id = action.sendid || action.sendidexpr || '?';
        const isDynamic = !action.sendid && action.sendidexpr;
        const suffix = isDynamic ? ' (dynamic)' : '';
        return `🚫 Cancel: ${id}${suffix}`;
    }

    /**
     * Format any action type
     * @param {Object} action - Action object
     * @returns {Object} {main: string, details: Array<string>}
     */
    function formatAction(action) {
        // Input validation
        if (!action || typeof action !== 'object') {
            return { main: '⚙️ [Invalid action]', details: [] };
        }

        const actionType = action.actionType || action.type;

        // Handle missing action type
        if (!actionType) {
            return { main: '⚙️ [Unknown action type]', details: [] };
        }

        let main = '';
        let details = [];

        try {
            switch (actionType) {
                case 'send':
                    main = formatSendAction(action);
                    details = formatSendDetails(action);
                    break;
                case 'raise':
                    main = formatRaiseAction(action);
                    break;
                case 'assign':
                    main = formatAssignAction(action);
                    break;
                case 'log':
                    main = formatLogAction(action);
                    break;
                case 'cancel':
                    main = formatCancelAction(action);
                    break;
                default:
                    main = `⚙️ ${actionType}`;
            }
        } catch (error) {
            // Graceful error handling - log to console but return safe fallback
            console.error('ActionFormatter error:', error, 'for action:', action);
            main = `⚙️ ${actionType} [Format error]`;
            details = [];
        }

        return { main, details };
    }

    // Public API
    return {
        formatAction: formatAction,
        formatSendAction: formatSendAction,
        formatSendDetails: formatSendDetails,
        formatRaiseAction: formatRaiseAction,
        formatAssignAction: formatAssignAction,
        formatLogAction: formatLogAction,
        formatCancelAction: formatCancelAction
    };
})();
