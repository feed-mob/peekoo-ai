import { peekoo_system_time_millis, peekoo_system_uuid_v4 } from "./host";
import { extractStringField, extractU64Field } from "./json";
import { readString, writeString } from "./memory";

export function timeMillis(): u64 {
  const offset = peekoo_system_time_millis(writeString("{}"));
  return extractU64Field(readString(offset), "timeMillis");
}

export function uuidV4(): string {
  const offset = peekoo_system_uuid_v4(writeString("{}"));
  return extractStringField(readString(offset), "uuid");
}
