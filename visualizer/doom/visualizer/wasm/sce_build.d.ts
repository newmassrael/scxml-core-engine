/* tslint:disable */
/* eslint-disable */

/**
 * Compile SCXML to generated code for any supported language.
 *
 * Returns a JSON string: `[["filename", "code"], ...]`
 * All templates are embedded in the WASM binary — no network requests needed.
 */
export function compile_scxml_lang(scxml_content: string, scxml_name: string, language: string): string;

/**
 * Extract the state machine name from SCXML content.
 */
export function get_machine_name(scxml_content: string): string;

/**
 * Languages this build can generate, as the identifiers
 * [`compile_scxml_lang`] accepts.
 *
 * Exported so a caller's language menu is a projection of what the
 * generator actually supports rather than a second list to keep in
 * step — the visualizer offered a Go button against a dispatcher that
 * rejected Go for exactly as long as the two were maintained apart.
 */
export function supported_languages(): string[];

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile_scxml_lang: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly get_machine_name: (a: number, b: number) => [number, number, number, number];
    readonly supported_languages: () => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_drop_slice: (a: number, b: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
