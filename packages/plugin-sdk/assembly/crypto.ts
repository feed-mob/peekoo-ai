import {
  peekoo_crypto_ed25519_get_or_create,
  peekoo_crypto_ed25519_sign,
} from "./host";
import { extractStringField, quote } from "./json";
import { readString, writeString } from "./memory";

export class Ed25519PublicKeyInfo {
  constructor(
    public publicKeyBase64Url: string,
    public publicKeySha256Hex: string,
  ) {}
}

export function ed25519GetOrCreate(alias: string): Ed25519PublicKeyInfo {
  const input = "{\"alias\":" + quote(alias) + "}";
  const offset = peekoo_crypto_ed25519_get_or_create(writeString(input));
  const response = readString(offset);
  return new Ed25519PublicKeyInfo(
    extractStringField(response, "publicKeyBase64Url"),
    extractStringField(response, "publicKeySha256Hex"),
  );
}

export function ed25519Sign(alias: string, payload: string): string {
  const input = "{\"alias\":" + quote(alias) + ",\"payload\":" + quote(payload) + "}";
  const offset = peekoo_crypto_ed25519_sign(writeString(input));
  return extractStringField(readString(offset), "signatureBase64Url");
}
