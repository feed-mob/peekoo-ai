/**
 * OpenClaw Sessions Manager Plugin
 * Manage OpenClaw sessions with configuration and session list view
 */

import { Host } from "@extism/as-pdk";
import * as state from "@peekoo/plugin-sdk/assembly/state";
import * as log from "@peekoo/plugin-sdk/assembly/log";
import * as notify from "@peekoo/plugin-sdk/assembly/notify";
import * as config from "@peekoo/plugin-sdk/assembly/config";
import { quote, extractStringField, extractIntField } from "@peekoo/plugin-sdk/assembly/json";

// Configuration keys
const CONFIG_WEBSOCKET_URL = "websocketUrl";
const CONFIG_TOKEN = "token";
const CONFIG_PASSWORD = "password";
const CONFIG_PAGE_SIZE = "pageSize";

// State keys
const STATE_SESSIONS_CACHE = "sessionsCache";
const STATE_CURRENT_PAGE = "currentPage";
const STATE_TOTAL_SESSIONS = "totalSessions";
const STATE_CONFIG_EXISTS = "configExists";

// Default values
const DEFAULT_WEBSOCKET_URL: string = "ws://127.0.0.1:18789";
const DEFAULT_PAGE_SIZE: i32 = 10;

// Mock sessions data for UI demonstration
const MOCK_SESSIONS: string[] = [
  '{"kind":"direct","key":"agent:main:main","age":"21h ago","model":"MiniMax-M2.5","tokens":"13k/200k","ctxPercent":"7%","flags":"system id:a0c932da-b65e-4c4f-a97e-a75d5e7a1aaf"}',
  '{"kind":"direct","key":"agent:main:cron:task1","age":"22h ago","model":"MiniMax-M2.5","tokens":"27k/200k","ctxPercent":"14%","flags":"system id:b9f77996-4a54-4482-a662-e1707ac12ac5"}',
  '{"kind":"direct","key":"agent:main:cron:task2","age":"22h ago","model":"MiniMax-M2.5","tokens":"26k/200k","ctxPercent":"13%","flags":"system id:af506100-2b36-495b-84bd-dfbc87d09323"}',
  '{"kind":"direct","key":"agent:main:cron:daily","age":"23h ago","model":"GPT-4","tokens":"45k/128k","ctxPercent":"35%","flags":"system id:c8d4e5f6-a1b2-4c3d-8e9f-0a1b2c3d4e5f"}',
  '{"kind":"direct","key":"agent:main:cron:weekly","age":"1d ago","model":"Claude-3.5","tokens":"82k/200k","ctxPercent":"41%","flags":"system id:d9e5f6a7-b2c3-4d5e-9f0a-1b2c3d4e5f6a"}',
  '{"kind":"proxy","key":"agent:proxy:api","age":"2h ago","model":"GPT-3.5","tokens":"5k/16k","ctxPercent":"31%","flags":"system rate-limited"}',
  '{"kind":"proxy","key":"agent:proxy:web","age":"3h ago","model":"GPT-4","tokens":"12k/128k","ctxPercent":"9%","flags":"system"}',
  '{"kind":"direct","key":"agent:main:cron:hourly","age":"45m ago","model":"MiniMax-M2.5","tokens":"8k/200k","ctxPercent":"4%","flags":"system id:e1f2a3b4-c5d6-4e7f-8a9b-0c1d2e3f4a5b"}',
  '{"kind":"direct","key":"agent:main:test","age":"5m ago","model":"Claude-3.5","tokens":"2k/200k","ctxPercent":"1%","flags":"system id:f2a3b4c5-d6e7-4f8a-9b0c-1d2e3f4a5b6c"}',
  '{"kind":"proxy","key":"agent:proxy:mobile","age":"15m ago","model":"GPT-4","tokens":"18k/128k","ctxPercent":"14%","flags":"system"}'
];

// ═══════════════════════════════════════════════════════════════
// Plugin Lifecycle
// ═══════════════════════════════════════════════════════════════

export function plugin_init(): i32 {
  log.info("OpenClaw Sessions Manager 插件已加载");

  // Initialize default configuration if not exists
  const configExists = state.get(STATE_CONFIG_EXISTS);
  if (configExists === "" || configExists === "false") {
    state.set(CONFIG_WEBSOCKET_URL, DEFAULT_WEBSOCKET_URL);
    state.set(CONFIG_TOKEN, "");
    state.set(CONFIG_PASSWORD, "");
    state.set(CONFIG_PAGE_SIZE, DEFAULT_PAGE_SIZE.toString());
    state.set(STATE_CONFIG_EXISTS, "false");
    log.info("默认配置已初始化");
  }

  // Initialize sessions cache with mock data
  const existingCache = state.get(STATE_SESSIONS_CACHE);
  if (existingCache === "") {
    // Build mock sessions array string
    let mockData = "[";
    for (let i = 0; i < MOCK_SESSIONS.length; i++) {
      if (i > 0) mockData += ",";
      mockData += MOCK_SESSIONS[i];
    }
    mockData += "]";
    state.set(STATE_SESSIONS_CACHE, mockData);
    state.set(STATE_TOTAL_SESSIONS, MOCK_SESSIONS.length.toString());
    state.set(STATE_CURRENT_PAGE, "1");
    log.info("模拟会话数据已加载: " + MOCK_SESSIONS.length.toString() + " 条");
  }

  Host.outputString('{"status":"ok","message":"OpenClaw Sessions Manager initialized"}');
  return 0;
}

export function plugin_shutdown(): i32 {
  log.info("OpenClaw Sessions Manager 正在关闭...");
  Host.outputString('{"status":"ok"}');
  return 0;
}

// ═══════════════════════════════════════════════════════════════
// Tool Functions
// ═══════════════════════════════════════════════════════════════

export function tool_get_openclaw_config(): i32 {
  log.debug("获取 OpenClaw 配置");

  const websocketUrl = state.get(CONFIG_WEBSOCKET_URL);
  const token = state.get(CONFIG_TOKEN);
  const password = state.get(CONFIG_PASSWORD);
  const configExists = state.get(STATE_CONFIG_EXISTS);

  const result = '{"websocketUrl":' + quote(websocketUrl) +
                 ',"token":' + quote(token) +
                 ',"password":' + quote(password) +
                 ',"configExists":' + configExists + '}';

  Host.outputString(result);
  return 0;
}

export function tool_save_openclaw_config(): i32 {
  const input = Host.inputString();
  log.debug("保存 OpenClaw 配置: " + input);

  const websocketUrl = extractStringField(input, "websocketUrl");
  const token = extractStringField(input, "token");
  const password = extractStringField(input, "password");

  // Validate: at least one of token or password must be provided
  if (token === "" && password === "") {
    Host.outputString('{"success":false,"error":"Either token or password must be provided"}');
    return 0;
  }

  // Save configuration
  state.set(CONFIG_WEBSOCKET_URL, websocketUrl !== "" ? websocketUrl : DEFAULT_WEBSOCKET_URL);
  state.set(CONFIG_TOKEN, token);
  state.set(CONFIG_PASSWORD, password);
  state.set(STATE_CONFIG_EXISTS, "true");

  log.info("OpenClaw 配置已保存");
  Host.outputString('{"success":true}');
  return 0;
}

export function tool_list_sessions(): i32 {
  const input = Host.inputString();
  const page = extractIntField(input, "page");
  const pageSize = extractIntField(input, "pageSize");

  const currentPage: i32 = page > 0 ? page : 1;
  const itemsPerPage: i32 = pageSize > 0 ? pageSize : DEFAULT_PAGE_SIZE;

  log.debug("列出会话 - 页码: " + currentPage.toString() + ", 每页: " + itemsPerPage.toString());

  // Get cached sessions
  const sessionsData = state.get(STATE_SESSIONS_CACHE);
  const totalStr = state.get(STATE_TOTAL_SESSIONS);
  const totalSessions: i32 = totalStr != "" ? parseInt(totalStr) as i32 : 0;

  // Calculate pagination
  const totalPages: i32 = ceilDiv(totalSessions, itemsPerPage);
  const safePage: i32 = currentPage > totalPages ? totalPages : currentPage;

  // Build response - return all sessions with pagination info
  // Client-side will handle the actual pagination display
  const result = '{"sessions":' + sessionsData +
                 ',"pagination":{"page":' + safePage.toString() +
                 ',"pageSize":' + itemsPerPage.toString() +
                 ',"total":' + totalSessions.toString() +
                 ',"totalPages":' + totalPages.toString() + '}}';

  Host.outputString(result);
  return 0;
}

export function tool_refresh_sessions(): i32 {
  log.info("刷新会话列表...");

  // In a real implementation, this would fetch from OpenClaw WebSocket
  // For now, we'll just reload the mock data with a timestamp update

  notify.send("OpenClaw Sessions", "Sessions refreshed successfully");

  Host.outputString('{"success":true,"message":"Sessions refreshed","timestamp":' + Date.now().toString() + '}');
  return 0;
}

// ═══════════════════════════════════════════════════════════════
// Data Providers
// ═══════════════════════════════════════════════════════════════

export function data_session_stats(): i32 {
  const sessionsData = state.get(STATE_SESSIONS_CACHE);
  const totalStr = state.get(STATE_TOTAL_SESSIONS);
  const totalSessions: i32 = totalStr != "" ? parseInt(totalStr) as i32 : 0;

  // Parse sessions to count by kind
  let directCount: i32 = 0;
  let proxyCount: i32 = 0;

  // Simple string search for counting (simplified for AssemblyScript)
  // Note: In real implementation, we'd parse the JSON properly
  const searchPattern1: string = '"kind":"direct"';
  const searchPattern2: string = '"kind":"proxy"';

  // Count occurrences (simplified)
  for (let i: i32 = 0; i < sessionsData.length - 14; i++) {
    if (sessionsData.substring(i, i + 14) == searchPattern1) {
      directCount++;
    } else if (sessionsData.substring(i, i + 13) == searchPattern2) {
      proxyCount++;
    }
  }

  const result = '{"total":' + totalSessions.toString() +
                 ',"direct":' + directCount.toString() +
                 ',"proxy":' + proxyCount.toString() +
                 ',"cachedAt":"' + getTimestamp() + '"}';

  Host.outputString(result);
  return 0;
}

// ═══════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════

function ceilDiv(a: i32, b: i32): i32 {
  return (a + b - 1) / b;
}

function parseInt(s: string): i32 {
  if (s.length == 0) return 0;

  let result: i32 = 0;
  let negative: bool = false;
  let start: i32 = 0;

  if (s.charAt(0) == '-') {
    negative = true;
    start = 1;
  }

  for (let i: i32 = start; i < s.length; i++) {
    const c: i32 = s.charCodeAt(i);
    if (c >= 48 && c <= 57) {
      result = result * 10 + (c - 48);
    } else {
      break;
    }
  }

  return negative ? -result : result;
}

// Get timestamp as string for session data
function getTimestamp(): string {
  // In real implementation, this would call a host function
  // For now return empty string
  return "";
}
