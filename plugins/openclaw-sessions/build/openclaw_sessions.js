export async function instantiate(module, imports = {}) {
  const __module0 = imports["extism:host/env"];
  const __module1 = imports["extism:host/user"];
  const adaptedImports = {
    "extism:host/env": Object.assign(Object.create(__module0), {
      alloc(a) {
        // ~lib/@extism/as-pdk/lib/env/alloc(u64) => u64
        a = BigInt.asUintN(64, a);
        return __module0.alloc(a) || 0n;
      },
      store_u8(a, b) {
        // ~lib/@extism/as-pdk/lib/env/store_u8(u64, u8) => void
        a = BigInt.asUintN(64, a);
        __module0.store_u8(a, b);
      },
      store_u64(a, b) {
        // ~lib/@extism/as-pdk/lib/env/store_u64(u64, u64) => void
        a = BigInt.asUintN(64, a);
        b = BigInt.asUintN(64, b);
        __module0.store_u64(a, b);
      },
      output_set(a, b) {
        // ~lib/@extism/as-pdk/lib/env/output_set(u64, u64) => void
        a = BigInt.asUintN(64, a);
        b = BigInt.asUintN(64, b);
        __module0.output_set(a, b);
      },
      length(a) {
        // ~lib/@extism/as-pdk/lib/env/length(u64) => u64
        a = BigInt.asUintN(64, a);
        return __module0.length(a) || 0n;
      },
      load_u8(a) {
        // ~lib/@extism/as-pdk/lib/env/load_u8(u64) => u8
        a = BigInt.asUintN(64, a);
        return __module0.load_u8(a);
      },
      load_u64(a) {
        // ~lib/@extism/as-pdk/lib/env/load_u64(u64) => u64
        a = BigInt.asUintN(64, a);
        return __module0.load_u64(a) || 0n;
      },
      input_length() {
        // ~lib/@extism/as-pdk/lib/env/input_length() => u64
        return __module0.input_length() || 0n;
      },
      input_load_u8(offs) {
        // ~lib/@extism/as-pdk/lib/env/input_load_u8(u64) => u8
        offs = BigInt.asUintN(64, offs);
        return __module0.input_load_u8(offs);
      },
      input_load_u64(offs) {
        // ~lib/@extism/as-pdk/lib/env/input_load_u64(u64) => u64
        offs = BigInt.asUintN(64, offs);
        return __module0.input_load_u64(offs) || 0n;
      },
    }),
    "extism:host/user": Object.assign(Object.create(__module1), {
      peekoo_state_get(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_state_get(i64) => i64
        return __module1.peekoo_state_get(input) || 0n;
      },
      peekoo_state_set(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_state_set(i64) => i64
        return __module1.peekoo_state_set(input) || 0n;
      },
      peekoo_config_get(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_config_get(i64) => i64
        return __module1.peekoo_config_get(input) || 0n;
      },
      peekoo_websocket_connect(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_websocket_connect(i64) => i64
        return __module1.peekoo_websocket_connect(input) || 0n;
      },
      peekoo_websocket_recv(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_websocket_recv(i64) => i64
        return __module1.peekoo_websocket_recv(input) || 0n;
      },
      peekoo_websocket_close(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_websocket_close(i64) => i64
        return __module1.peekoo_websocket_close(input) || 0n;
      },
      peekoo_crypto_ed25519_get_or_create(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_crypto_ed25519_get_or_create(i64) => i64
        return __module1.peekoo_crypto_ed25519_get_or_create(input) || 0n;
      },
      peekoo_system_time_millis(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_system_time_millis(i64) => i64
        return __module1.peekoo_system_time_millis(input) || 0n;
      },
      peekoo_crypto_ed25519_sign(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_crypto_ed25519_sign(i64) => i64
        return __module1.peekoo_crypto_ed25519_sign(input) || 0n;
      },
      peekoo_system_uuid_v4(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_system_uuid_v4(i64) => i64
        return __module1.peekoo_system_uuid_v4(input) || 0n;
      },
      peekoo_websocket_send(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_websocket_send(i64) => i64
        return __module1.peekoo_websocket_send(input) || 0n;
      },
      peekoo_notify(input) {
        // ~lib/@peekoo/plugin-sdk/assembly/host/peekoo_notify(i64) => i64
        return __module1.peekoo_notify(input) || 0n;
      },
    }),
  };
  const { exports } = await WebAssembly.instantiate(module, adaptedImports);
  const memory = exports.memory || imports.env.memory;
  const adaptedExports = Object.setPrototypeOf({
    abort(message, fileName, lineNumber, columnNumber) {
      // assembly/index/abort(~lib/string/String | null, ~lib/string/String | null, u32, u32) => void
      message = __retain(__lowerString(message));
      fileName = __lowerString(fileName);
      try {
        exports.abort(message, fileName, lineNumber, columnNumber);
      } finally {
        __release(message);
      }
    },
  }, exports);
  function __lowerString(value) {
    if (value == null) return 0;
    const
      length = value.length,
      pointer = exports.__new(length << 1, 2) >>> 0,
      memoryU16 = new Uint16Array(memory.buffer);
    for (let i = 0; i < length; ++i) memoryU16[(pointer >>> 1) + i] = value.charCodeAt(i);
    return pointer;
  }
  const refcounts = new Map();
  function __retain(pointer) {
    if (pointer) {
      const refcount = refcounts.get(pointer);
      if (refcount) refcounts.set(pointer, refcount + 1);
      else refcounts.set(exports.__pin(pointer), 1);
    }
    return pointer;
  }
  function __release(pointer) {
    if (pointer) {
      const refcount = refcounts.get(pointer);
      if (refcount === 1) exports.__unpin(pointer), refcounts.delete(pointer);
      else if (refcount) refcounts.set(pointer, refcount - 1);
      else throw Error(`invalid refcount '${refcount}' for reference '${pointer}'`);
    }
  }
  return adaptedExports;
}
