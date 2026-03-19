declare namespace __AdaptedExports {
  /** Exported memory */
  export const memory: WebAssembly.Memory;
  // Exported runtime interface
  export function __new(size: number, id: number): number;
  export function __pin(ptr: number): number;
  export function __unpin(ptr: number): void;
  export function __collect(): void;
  export const __rtti_base: number;
  /**
   * assembly/index/abort
   * @param message `~lib/string/String | null`
   * @param fileName `~lib/string/String | null`
   * @param lineNumber `u32`
   * @param columnNumber `u32`
   */
  export function abort(message: string | null, fileName: string | null, lineNumber: number, columnNumber: number): void;
  /**
   * assembly/index/plugin_init
   * @returns `i32`
   */
  export function plugin_init(): number;
  /**
   * assembly/index/plugin_shutdown
   * @returns `i32`
   */
  export function plugin_shutdown(): number;
  /**
   * assembly/index/tool_get_openclaw_config
   * @returns `i32`
   */
  export function tool_get_openclaw_config(): number;
  /**
   * assembly/index/tool_save_openclaw_config
   * @returns `i32`
   */
  export function tool_save_openclaw_config(): number;
  /**
   * assembly/index/tool_list_sessions
   * @returns `i32`
   */
  export function tool_list_sessions(): number;
  /**
   * assembly/index/tool_refresh_sessions
   * @returns `i32`
   */
  export function tool_refresh_sessions(): number;
  /**
   * assembly/index/tool_openclaw_chat_history
   * @returns `i32`
   */
  export function tool_openclaw_chat_history(): number;
  /**
   * assembly/index/tool_openclaw_chat_send
   * @returns `i32`
   */
  export function tool_openclaw_chat_send(): number;
}
/** Instantiates the compiled WebAssembly module with the given imports. */
export declare function instantiate(module: WebAssembly.Module, imports: {
  "extism:host/env": unknown,
  "extism:host/user": unknown,
}): Promise<typeof __AdaptedExports>;
