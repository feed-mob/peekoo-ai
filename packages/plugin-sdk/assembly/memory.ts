import { Memory } from "@extism/as-pdk";
import { length } from "@extism/as-pdk/lib/env";

/**
 * Write a string into Extism memory and return the offset handle.
 */
export function writeString(s: string): i64 {
  const mem = Memory.allocateString(s);
  return <i64>mem.offset;
}

/**
 * Read a string from an Extism memory offset handle.
 */
export function readString(offset: i64): string {
  if (offset == 0) return "";
  const mem = new Memory(<u64>offset, length(<u64>offset));
  return mem.toString();
}
