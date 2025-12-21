/* tslint:disable */
/* eslint-disable */

export class PirClient {
  free(): void;
  [Symbol.dispose](): void;
  entry_count(): bigint;
  query_binary(index: bigint): Promise<Uint8Array>;
  constructor(server_url: string);
  init(lane: string): Promise<void>;
  query(index: bigint): Promise<Uint8Array>;
}

export function init(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_pirclient_free: (a: number, b: number) => void;
  readonly pirclient_entry_count: (a: number) => [bigint, number, number];
  readonly pirclient_init: (a: number, b: number, c: number) => any;
  readonly pirclient_new: (a: number, b: number) => number;
  readonly pirclient_query: (a: number, b: bigint) => any;
  readonly pirclient_query_binary: (a: number, b: bigint) => any;
  readonly init: () => void;
  readonly wasm_bindgen__convert__closures_____invoke__hd821b0df02ba9af1: (a: number, b: number, c: any) => void;
  readonly wasm_bindgen__closure__destroy__h6a9a7833fcddcc5d: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures_____invoke__h9128ca1f158254e1: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __externref_table_dealloc: (a: number) => void;
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
