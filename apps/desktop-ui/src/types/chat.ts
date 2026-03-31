export interface ToolCallState {
  id: string;
  name: string;
  status: 'running' | 'complete' | 'error';
  args?: string; // JSON arguments as string
  result?: 'success' | 'failure';
}

export interface Message {
  id: string;
  role: "user" | "pet" | "error";
  text: string;
  /** True while the message is still being streamed */
  streaming?: boolean;
  /** Thinking/reasoning content (if agent provides it) */
  thinking?: string;
  /** Tool calls made during this message */
  toolCalls?: ToolCallState[];
  /** UI state for thinking block collapse/expand */
  showThinking?: boolean;
}

export interface SessionMessageDto {
  role: string;
  text: string;
}

export interface LastSessionDto {
  sessionPath: string;
  messages: SessionMessageDto[];
}

// ── Agent event types (mirroring Rust AgentEvent) ──

export interface AgentEventToolStart {
  type: "tool_execution_start";
  toolCallId: string;
  toolName: string;
  args: unknown;
}

export interface AgentEventToolEnd {
  type: "tool_execution_end";
  toolCallId: string;
  toolName: string;
  result: unknown;
  isError: boolean;
}

export interface AgentEventMessageUpdate {
  type: "message_update";
  message: AgentMessagePayload;
  assistantMessageEvent: AssistantMessageEvent;
}

export interface AgentEventMessageEnd {
  type: "message_end";
  message: AgentMessagePayload;
}

export interface AgentEventGeneric {
  type: string;
  [key: string]: unknown;
}

export interface AgentEventTextDelta {
  TextDelta: string;
}

export interface AgentEventThinkingDelta {
  ThinkingDelta: string;
}

export interface AgentEventToolCallDeltaRust {
  ToolCallDelta: {
    id: string;
    arguments: string;
  };
}

export interface AgentEventToolCallStartRust {
  ToolCallStart: {
    id: string;
    name: string;
  };
}

export interface AgentEventToolCallCompleteRust {
  ToolCallComplete: {
    id: string;
  };
}

export interface AgentEventCompleteRust {
  Complete: null | Record<string, never>;
}

// Helper type for when Complete is sent as just a string
export type AgentEventCompleteString = "Complete";

export interface AgentEventErrorRust {
  Error: string;
}

export type AgentEvent =
  | AgentEventToolStart
  | AgentEventToolEnd
  | AgentEventMessageUpdate
  | AgentEventMessageEnd
  | AgentEventTextDelta
  | AgentEventThinkingDelta
  | AgentEventToolCallDeltaRust
  | AgentEventToolCallStartRust
  | AgentEventToolCallCompleteRust
  | AgentEventCompleteRust
  | AgentEventErrorRust
  | AgentEventGeneric;

// Sub-types from pi_agent_rust serialization

export interface AgentMessagePayload {
  Assistant?: AssistantMessageContent;
  [key: string]: unknown;
}

export interface AssistantMessageContent {
  content: ContentBlock[];
  [key: string]: unknown;
}

export type ContentBlock =
  | { Text: { text: string } }
  | { Thinking: { thinking: string } }
  | { ToolCall: { id: string; name: string; args: unknown } }
  | { [key: string]: unknown };

export interface AssistantMessageEvent {
  Text?: { text: string };
  Thinking?: { thinking: string };
  [key: string]: unknown;
}
