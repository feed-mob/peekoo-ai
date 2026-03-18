import { Host } from "@extism/as-pdk";
import * as config from "@peekoo/plugin-sdk/assembly/config";
import * as crypto from "@peekoo/plugin-sdk/assembly/crypto";
import { extractIntField, extractRawField, extractStringField, quote } from "@peekoo/plugin-sdk/assembly/json";
import * as notify from "@peekoo/plugin-sdk/assembly/notify";
import * as state from "@peekoo/plugin-sdk/assembly/state";
import * as system from "@peekoo/plugin-sdk/assembly/system";
import * as websocket from "@peekoo/plugin-sdk/assembly/websocket";

const CONFIG_WEBSOCKET_URL = "websocketUrl";
const CONFIG_TOKEN = "token";
const CONFIG_PASSWORD = "password";
const STATE_CONFIG_EXISTS = "configExists";
const STATE_SESSIONS_CACHE = "sessionsCache";
const CONFIG_DEFAULT_PAGE_SIZE = "default_page_size";
const DEVICE_KEY_ALIAS = "openclaw-device-v2";
const DEFAULT_WEBSOCKET_URL = "ws://127.0.0.1:18789";
const DEFAULT_PAGE_SIZE: i32 = 10;

class OpenClawConfig {
  constructor(
    public websocketUrl: string,
    public token: string,
    public password: string,
    public configExists: bool,
  ) {}
}

export function abort(
  message: string | null,
  fileName: string | null,
  lineNumber: u32,
  columnNumber: u32,
): void {
  const messageText = message === null ? "abort" : changetype<string>(message);
  const fileText = fileName === null ? "unknown" : changetype<string>(fileName);
  Host.outputString(
    '{"success":false,"error":' +
      quote(fileText + ":" + lineNumber.toString() + ":" + columnNumber.toString() + ": " + messageText) +
      "}",
  );
}

export function plugin_init(): i32 {
  initializeDefaults();
  Host.outputString('{"status":"ok"}');
  return 0;
}

export function plugin_shutdown(): i32 {
  Host.outputString('{"status":"ok"}');
  return 0;
}

export function tool_get_openclaw_config(): i32 {
  initializeDefaults();
  Host.outputString(configToJson(loadConfig()));
  return 0;
}

export function tool_save_openclaw_config(): i32 {
  initializeDefaults();

  const input = Host.inputString();
  const websocketUrl = extractStringField(input, "websocketUrl");
  const token = extractStringField(input, "token");
  const password = extractStringField(input, "password");

  if (websocketUrl == "") {
    Host.outputString(errorJson("WebSocket URL is required"));
    return 0;
  }
  if (token == "" && password == "") {
    Host.outputString(errorJson("Either token or password must be provided"));
    return 0;
  }

  state.set(CONFIG_WEBSOCKET_URL, websocketUrl);
  state.set(CONFIG_TOKEN, token);
  state.set(CONFIG_PASSWORD, password);
  state.set(STATE_CONFIG_EXISTS, "true");

  Host.outputString(configToJson(loadConfig()));
  return 0;
}

export function tool_list_sessions(): i32 {
  initializeDefaults();

  const input = Host.inputString();
  const page = extractIntField(input, "page");
  const pageSize = extractIntField(input, "pageSize");
  const resolvedPage = page > 0 ? page : 1;
  const resolvedPageSize = pageSize > 0 ? pageSize : resolveDefaultPageSize();
  const cached = readCachedSessions(resolvedPage, resolvedPageSize);
  if (cached != "") {
    Host.outputString(cached);
    return 0;
  }

  Host.outputString(refreshSessions(resolvedPage, resolvedPageSize));
  return 0;
}

export function tool_refresh_sessions(): i32 {
  initializeDefaults();
  Host.outputString(refreshSessions(1, resolveDefaultPageSize()));
  return 0;
}

export function tool_openclaw_chat_history(): i32 {
  initializeDefaults();

  const input = Host.inputString();
  const sessionKey = extractStringField(input, "sessionKey");
  const limit = extractIntField(input, "limit");

  if (sessionKey == "") {
    Host.outputString(errorJson("sessionKey is required"));
    return 0;
  }

  const params = '{"sessionKey":' + quote(sessionKey) + ',"limit":' + positiveInt(limit, 200).toString() + '}';
  Host.outputString(gatewayRpc("chat.history", params));
  return 0;
}

export function tool_openclaw_chat_send(): i32 {
  initializeDefaults();

  const input = Host.inputString();
  const sessionKey = extractStringField(input, "sessionKey");
  const message = extractStringField(input, "message");

  if (sessionKey == "") {
    Host.outputString(errorJson("sessionKey is required"));
    return 0;
  }
  if (message == "") {
    Host.outputString(errorJson("message is required"));
    return 0;
  }

  const params =
    '{"sessionKey":' + quote(sessionKey) +
    ',"message":' + quote(message) +
    ',"idempotencyKey":' + quote(system.uuidV4()) + '}';
  Host.outputString(gatewayRpc("chat.send", params));
  return 0;
}

function refreshSessions(page: i32, pageSize: i32): string {
  const params = '{"limit":100,"includeLastMessage":true,"includeDerivedTitles":true}';
  const payload = gatewayRpc("sessions.list", params);
  if (!isErrorPayload(payload)) {
    state.set(STATE_SESSIONS_CACHE, buildCachedSessions(page, pageSize, payload));
    notify.send("OpenClaw Sessions", "Sessions refreshed successfully");
  }
  return payload;
}

function gatewayRpc(method: string, paramsJson: string): string {
  const cfg = loadConfig();
  if (!cfg.configExists) {
    return errorJson("OpenClaw configuration is not set");
  }

  let socketId = "";
  if (cfg.websocketUrl == "") {
    return errorJson("WebSocket URL is required");
  }

  socketId = websocket.connect(cfg.websocketUrl);
  if (socketId == "") {
    return errorJson("Failed to open WebSocket connection");
  }

  const nonce = waitForConnectChallenge(socketId);
  if (isErrorPayload(nonce)) {
    websocket.close(socketId);
    return nonce;
  }

  const device = crypto.ed25519GetOrCreate(DEVICE_KEY_ALIAS);
  const signedAt = system.timeMillis();
  const signedPayload = buildSignedPayload(device.publicKeySha256Hex, cfg, signedAt, nonce);
  const signature = crypto.ed25519Sign(DEVICE_KEY_ALIAS, signedPayload);
  const connectId = system.uuidV4();

  const connectReq =
    '{"type":"req","id":' + quote(connectId) + ',"method":"connect","params":{' +
    '"minProtocol":3,"maxProtocol":3,' +
    '"client":{' +
    '"id":"gateway-client",' +
    '"displayName":"OpenClaw Sessions",' +
    '"version":"1.0.0",' +
    '"platform":"peekoo-plugin",' +
    '"mode":"ui",' +
    '"instanceId":' + quote(system.uuidV4()) + '},' +
    '"auth":' + authJson(cfg) + ',' +
    '"role":"operator",' +
    '"scopes":["operator.admin"],' +
    '"device":{' +
    '"id":' + quote(device.publicKeySha256Hex) + ',' +
    '"publicKey":' + quote(device.publicKeyBase64Url) + ',' +
    '"signature":' + quote(signature) + ',' +
    '"signedAt":' + signedAt.toString() + ',' +
    '"nonce":' + (nonce == "" ? "null" : quote(nonce)) + '}}}';

  websocket.send(socketId, connectReq);
  const connectResult = waitForResponsePayload(socketId, connectId);
  if (isErrorPayload(connectResult)) {
    websocket.close(socketId);
    return connectResult;
  }

  const requestId = system.uuidV4();
  const request =
    '{"type":"req","id":' + quote(requestId) + ',"method":' + quote(method) + ',"params":' + paramsJson + '}';
  websocket.send(socketId, request);
  const payload = waitForResponsePayload(socketId, requestId);
  websocket.close(socketId);
  return payload;
}

function waitForConnectChallenge(socketId: string): string {
  for (let i = 0; i < 8; i++) {
    const message = websocket.recv(socketId);
    if (message == "") {
      return errorJson("Gateway returned an empty challenge message");
    }
    if (extractStringField(message, "type") == "event" && extractStringField(message, "event") == "connect.challenge") {
      return extractStringField(message, "nonce");
    }
  }

  return errorJson("Timed out waiting for connect.challenge");
}

function waitForResponsePayload(socketId: string, expectedId: string): string {
  for (let i = 0; i < 32; i++) {
    const message = websocket.recv(socketId);
    if (message == "") {
      return errorJson("Gateway returned an empty response");
    }
    if (extractStringField(message, "type") != "res") {
      continue;
    }
    if (extractStringField(message, "id") != expectedId) {
      continue;
    }
    if (extractRawField(message, "ok") == "true") {
      const payload = extractRawField(message, "payload");
      return payload == "" ? "{}" : payload;
    }

    const errorPayload = extractRawField(message, "error");
    const errorMessage = extractStringField(errorPayload, "message");
    if (errorMessage != "") {
      return errorJson(errorMessage);
    }
    return errorJson("Gateway request failed");
  }

  return errorJson("Timed out waiting for gateway response");
}

function buildSignedPayload(deviceId: string, cfg: OpenClawConfig, signedAt: u64, nonce: string): string {
  let payload =
    (nonce != "" ? "v2" : "v1") + "|" +
    deviceId + "|" +
    "gateway-client|ui|operator|operator.admin|" +
    signedAt.toString() + "|" +
    secretFor(cfg);

  if (nonce != "") {
    payload += "|" + nonce;
  }

  return payload;
}

function authJson(cfg: OpenClawConfig): string {
  let body = "";
  if (cfg.token != "") {
    body += '"token":' + quote(cfg.token);
  }
  if (cfg.password != "") {
    if (body != "") {
      body += ",";
    }
    body += '"password":' + quote(cfg.password);
  }
  return "{" + body + "}";
}

function secretFor(cfg: OpenClawConfig): string {
  return cfg.token != "" ? cfg.token : cfg.password;
}

function configToJson(cfg: OpenClawConfig): string {
  return (
    '{"websocketUrl":' + quote(cfg.websocketUrl) +
    ',"token":' + quote(cfg.token) +
    ',"password":' + quote(cfg.password) +
    ',"configExists":' + boolJson(cfg.configExists) +
    ',"defaultPageSize":' + resolveDefaultPageSize().toString() +
    '}'
  );
}

function loadConfig(): OpenClawConfig {
  return new OpenClawConfig(
    defaultIfEmpty(state.get(CONFIG_WEBSOCKET_URL), DEFAULT_WEBSOCKET_URL),
    state.get(CONFIG_TOKEN),
    state.get(CONFIG_PASSWORD),
    state.get(STATE_CONFIG_EXISTS) == "true",
  );
}

function initializeDefaults(): void {
  if (state.get(CONFIG_WEBSOCKET_URL) == "") {
    state.set(CONFIG_WEBSOCKET_URL, DEFAULT_WEBSOCKET_URL);
  }
  if (state.get(CONFIG_TOKEN) == "") {
    state.set(CONFIG_TOKEN, "");
  }
  if (state.get(CONFIG_PASSWORD) == "") {
    state.set(CONFIG_PASSWORD, "");
  }
  if (state.get(STATE_CONFIG_EXISTS) == "") {
    state.set(STATE_CONFIG_EXISTS, "false");
  }
}

function readCachedSessions(page: i32, pageSize: i32): string {
  const cached = state.get(STATE_SESSIONS_CACHE);
  if (cached == "") {
    return "";
  }

  if (extractIntField(cached, "page") != page) {
    return "";
  }
  if (extractIntField(cached, "pageSize") != pageSize) {
    return "";
  }

  return extractRawField(cached, "payload");
}

function buildCachedSessions(page: i32, pageSize: i32, payload: string): string {
  return (
    '{"page":' + page.toString() +
    ',"pageSize":' + pageSize.toString() +
    ',"payload":' + payload + "}"
  );
}

function resolveDefaultPageSize(): i32 {
  const configured = parseIntString(config.get(CONFIG_DEFAULT_PAGE_SIZE));
  return configured > 0 ? configured : DEFAULT_PAGE_SIZE;
}

function defaultIfEmpty(value: string, fallback: string): string {
  return value == "" ? fallback : value;
}

function positiveInt(value: i32, fallback: i32): i32 {
  return value > 0 ? value : fallback;
}

function boolJson(value: bool): string {
  return value ? "true" : "false";
}

function parseIntString(raw: string): i32 {
  if (raw.length == 0) {
    return 0;
  }

  let result: i32 = 0;
  for (let i = 0; i < raw.length; i++) {
    const c = raw.charCodeAt(i);
    if (c >= 48 && c <= 57) {
      result = result * 10 + (c - 48);
      continue;
    }
    break;
  }

  return result;
}

function isErrorPayload(payload: string): bool {
  return extractRawField(payload, "success") == "false" && extractStringField(payload, "error") != "";
}

function errorJson(message: string): string {
  return '{"success":false,"error":' + quote(message) + '}';
}
