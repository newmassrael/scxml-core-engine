/**
 * The codegen WASM (`sce-build-wasm`), loaded once for the whole page.
 *
 * The GUI's structure, the annotation overlay and the browser code
 * generator all come from this one module. Each used to import it on its
 * own; three loaders of one module are three places to keep in step, and
 * the structure — the one the GUI cannot draw without — must not depend on
 * which of them happened to run first.
 *
 * ⚠ Failure policy is the CALLER's, not this module's: the overlay is
 * optional (an unmarked diagram is honest), the structure is not (a GUI
 * drawing some other reader's interpretation is exactly what the Rust
 * structure replaced). So `load()` rejects, and each caller decides.
 */
const SceBuildWasm = (() => {
    let ready = null;

    /** The initialised module; the same promise for every caller. */
    function load(base = 'wasm/') {
        if (!ready) {
            ready = (async () => {
                const module = await import(`./${base}sce_build.js`);
                await module.default(`./${base}sce_build_bg.wasm`);
                return module;
            })();
        }
        return ready;
    }

    /**
     * The statechart structure the GUI draws, built from the Rust model
     * (`sce_build::gui_structure`). Throws with the product's own message
     * when the product refuses the document — never falls back to another
     * reader's structure.
     */
    async function guiStructure(scxmlContent, scxmlName) {
        const module = await load();
        return JSON.parse(module.gui_structure(scxmlContent, scxmlName || 'untitled'));
    }

    /**
     * Hand over the module a host initialised itself, for a host that has no
     * `fetch` for a `.wasm` file (a Node harness): everything below then runs
     * on it, the code the page runs.
     */
    function provide(module) {
        ready = Promise.resolve(module);
    }

    /** `name` read from the directory `dir`, with `.` and `..` resolved. */
    function beside(dir, name) {
        const parts = [];
        for (const part of (dir ? `${dir}/${name}` : name).split('/')) {
            if (part === '' || part === '.') continue;
            if (part === '..' && parts.length > 0) parts.pop();
            else parts.push(part);
        }
        return parts.join('/');
    }

    /**
     * The documents the Interpreter runs for `scxmlContent`, the document
     * asked for first: itself, or the ECMAScript lowering of a
     * `datamodel="sce-static"` one with every document its hybrid `<invoke>`s
     * may start. Throws with the product's own message when the document
     * cannot be lowered — the Interpreter is never handed a typed document it
     * would run untyped.
     *
     * The lowering reads what the document names beside itself (an imported
     * algorithm or schema, a candidate). The page has no file system, so
     * `fetchSibling(path)` answers each such path with the text, or `null` for
     * a file that is not there; the product's own reader names the paths
     * (`documents_named_by`), down to the ones the documents named name in
     * turn, and a path that comes back `null` is refused by the lowering as a
     * file that is not there. The built-in standard library is fetched from
     * nowhere.
     */
    async function lowerForInterpreter(scxmlContent, scxmlName, fetchSibling) {
        const module = await load();
        const siblings = {};
        const wanted = JSON.parse(module.documents_named_by(scxmlContent))
            .map((name) => beside('', name));
        const asked = new Set();
        while (wanted.length > 0) {
            const path = wanted.shift();
            if (asked.has(path)) continue;
            asked.add(path);
            const text = fetchSibling ? await fetchSibling(path) : null;
            if (text === null || text === undefined) continue;
            siblings[path] = text;
            const dir = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
            for (const name of JSON.parse(module.documents_named_by(text))) {
                wanted.push(beside(dir, name));
            }
        }
        return JSON.parse(module.lower_for_interpreter(
            scxmlContent, scxmlName || 'untitled', JSON.stringify(siblings)));
    }

    return { load, provide, guiStructure, lowerForInterpreter };
})();

if (typeof window !== 'undefined') {
    window.SceBuildWasm = SceBuildWasm;
}
// A Node harness takes the same object the page runs.
if (typeof module !== 'undefined' && module.exports) {
    module.exports = SceBuildWasm;
}
