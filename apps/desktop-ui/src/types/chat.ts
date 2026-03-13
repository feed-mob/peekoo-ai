export interface Message {
  id: string;
  role: "user" | "pet" | "error";
  text: string;
  /** True while the message is still being streamed */
  streaming?: boolean;
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

export type AgentEvent =
  | AgentEventToolStart
  | AgentEventToolEnd
  | AgentEventMessageUpdate
  | AgentEventMessageEnd
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
