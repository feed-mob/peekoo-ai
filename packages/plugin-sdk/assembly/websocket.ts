import {
  peekoo_websocket_close,
  peekoo_websocket_connect,
  peekoo_websocket_recv,
  peekoo_websocket_send,
} from "./host";
import { extractStringField, quote } from "./json";
import { readString, writeString } from "./memory";

export function connect(url: string): string {
  const input = "{\"url\":" + quote(url) + "}";
  const offset = peekoo_websocket_connect(writeString(input));
  return extractStringField(readString(offset), "socketId");
}

export function send(socketId: string, text: string): void {
  const input =
    "{\"socketId\":" + quote(socketId) + ",\"text\":" + quote(text) + "}";
  peekoo_websocket_send(writeString(input));
}

export function recv(socketId: string): string {
  const input = "{\"socketId\":" + quote(socketId) + "}";
  const offset = peekoo_websocket_recv(writeString(input));
  return extractStringField(readString(offset), "text");
}

export function close(socketId: string): void {
  const input = "{\"socketId\":" + quote(socketId) + "}";
  peekoo_websocket_close(writeString(input));
}
