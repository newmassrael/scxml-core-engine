/**
 * Invoke formatter utilities for SCXML visualizer
 * Provides detailed formatting for W3C SCXML invoke elements
 */

const InvokeFormatter = (function() {
    'use strict';

    // Truncation limits for display optimization (matching ActionFormatter)
    const TRUNCATION = {
        TYPE: 50,          // Invoke type display limit
        SRC: 40,           // Source path display limit
        NAMELIST: 40,      // Namelist display limit
        CONTENT: 30,       // Content preview limit
        PARAMS: 50,        // Params display limit
        FINALIZE: 30       // Finalize script preview limit
    };

    // Detail line prefix for hierarchical display (matching ActionFormatter)
    const DETAIL_PREFIX = '   ↳ ';

    /**
     * Cut a value to a display limit, saying so where it was cut.
     *
     * ⚠ The limits above were declared and never read. Every detail line
     * pushed the attribute whole, so a `finalize` holding an `<assign>`
     * element — which is what W3C test 234 writes — rendered as one line
     * wider than the node, over the neighbouring state's text and off the
     * diagram. The node box is sized from its label, so an uncapped label
     * is also why the box no longer contains what it draws.
     *
     * ⚠⚠ The ellipsis is part of the contract, not decoration. A value
     * silently cut at 30 characters reads as the value the document holds,
     * and a reviewer comparing the diagram against the source would be
     * comparing against something the page invented by omission.
     */
    function cut(value, limit) {
        const text = String(value);
        return text.length > limit ? `${text.slice(0, limit)}…` : text;
    }

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
     * Format invoke information
     * W3C SCXML 6.4: Invoke external state machines or services
     * @param {Object} invokeData - Invoke data object from state
     * @returns {Object} {main: string, details: Array<string>}
     */
    function formatInvokeInfo(invokeData) {
        // Skip if no invoke data
        if (!invokeData || !invokeData.hasInvoke) {
            return { main: '', details: [] };
        }

        const icon = '🚀';

        // Determine type (static or dynamic)
        const type = invokeData.invokeType || invokeData.invokeTypeExpr || '?';
        const isDynamicType = isEmpty(invokeData.invokeType) && !isEmpty(invokeData.invokeTypeExpr);

        // Simplify common SCXML type for display
        const typeDisplay = type === 'http://www.w3.org/TR/scxml/' ? 'SCXML' : type;

        // Build main line
        const main = isDynamicType
            ? `${icon} ${typeDisplay} (dynamic)`
            : `${icon} ${typeDisplay}`;

        const details = [];

        // W3C SCXML 6.4.1: Type attribute (show if not default SCXML)
        if (type && type !== 'http://www.w3.org/TR/scxml/') {
            const suffix = isDynamicType ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}type: ${typeDisplay}${suffix}`);
        }

        // W3C SCXML 6.4.1: ID or idlocation attribute
        if (!isEmpty(invokeData.invokeId)) {
            details.push(`${DETAIL_PREFIX}id: ${invokeData.invokeId}`);
        } else if (!isEmpty(invokeData.invokeIdLocation)) {
            details.push(`${DETAIL_PREFIX}idlocation: ${invokeData.invokeIdLocation}`);
        }

        // W3C SCXML 6.4.1: Source (static or dynamic)
        const src = invokeData.invokeSrc || invokeData.invokeSrcExpr;
        if (!isEmpty(src)) {
            const isDynamicSrc = isEmpty(invokeData.invokeSrc) && !isEmpty(invokeData.invokeSrcExpr);
            const suffix = isDynamicSrc ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}src: ${cut(src, TRUNCATION.SRC)}${suffix}`);
        }

        // W3C SCXML 6.4.2: Content (inline SCXML or dynamic expression)
        if (!isEmpty(invokeData.invokeContent)) {
            // Show inline content (no truncation)
            details.push(`${DETAIL_PREFIX}content: <scxml...> (inline)`);
        } else if (!isEmpty(invokeData.invokeContentExpr)) {
            details.push(
                `${DETAIL_PREFIX}contentexpr: `
                + `${cut(invokeData.invokeContentExpr, TRUNCATION.CONTENT)}`);
        }

        // W3C SCXML 6.4.2: Params (name-value pairs to pass to child)
        if (hasItems(invokeData.invokeParams)) {
            const paramStrs = invokeData.invokeParams.map(p => {
                const name = p.name || '?';
                const expr = p.expr || p.location || '?';
                return `${name}=${expr}`;
            });
            const paramsStr = paramStrs.join(', ');
            details.push(`${DETAIL_PREFIX}params: ${cut(paramsStr, TRUNCATION.PARAMS)}`);
        }

        // W3C SCXML 6.4.1: Namelist (variable names to pass)
        if (!isEmpty(invokeData.invokeNamelist)) {
            details.push(
                `${DETAIL_PREFIX}namelist: `
                + `${cut(invokeData.invokeNamelist, TRUNCATION.NAMELIST)}`);
        }

        // W3C SCXML 6.4.1: AutoForward (automatic event forwarding)
        if (invokeData.invokeAutoForward === true) {
            details.push(`${DETAIL_PREFIX}autoforward: true`);
        }

        // W3C SCXML 6.5: Finalize (script to execute when child sends events)
        if (!isEmpty(invokeData.invokeFinalize)) {
            const finalizeContent = invokeData.invokeFinalize.trim();
            details.push(`${DETAIL_PREFIX}finalize: ${cut(finalizeContent, TRUNCATION.FINALIZE)}`);
        }

        return { main, details };
    }

    // Public API
    return {
        formatInvokeInfo: formatInvokeInfo
    };
})();
