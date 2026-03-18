/**
 * Raw host function imports.
 *
 * These are WASM imports provided by the Peekoo runtime via Extism.
 * Plugin authors should use the higher-level wrappers in the other modules
 * instead of calling these directly.
 *
 * All host functions use Extism's memory-handle ABI:
 *   input: i64 (memory offset of JSON string)
 *   output: i64 (memory offset of JSON response string)
 */

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_state_get")
export declare function peekoo_state_get(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_state_set")
export declare function peekoo_state_set(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_log")
export declare function peekoo_log(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_emit_event")
export declare function peekoo_emit_event(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_notify")
export declare function peekoo_notify(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_schedule_set")
export declare function peekoo_schedule_set(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_schedule_cancel")
export declare function peekoo_schedule_cancel(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_schedule_get")
export declare function peekoo_schedule_get(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_config_get")
export declare function peekoo_config_get(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_set_peek_badge")
export declare function peekoo_set_peek_badge(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_bridge_fs_read")
export declare function peekoo_bridge_fs_read(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_fs_read")
export declare function peekoo_fs_read(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_fs_read_dir")
export declare function peekoo_fs_read_dir(input: i64): i64;

// @ts-ignore: decorator
@external("extism:host/user", "peekoo_set_mood")
export declare function peekoo_set_mood(input: i64): i64;
