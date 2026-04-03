import { useCallback, useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { emitPetReaction } from "@/lib/pet-events";
import type {
  AgentEvent,
  AgentEventMessageUpdate,
  AgentEventToolEnd,
  AgentEventToolStart,
  AssistantMessageEvent,
  LastSessionDto,
  Message,
  ToolCallState,
} from "@/types/chat";

export interface SessionMessageLike {
  role: string;
  text: string;
}

export type MiniChatReplyDisplayMode = "compact" | "expanded";

const CHAT_SESSION_CHANGED_EVENT = "chat-session:changed";
const CHAT_SESSION_NEW_EVENT = "chat-session:new";
const MINI_CHAT_EXPANDED_TEXT_LENGTH = 72;

function isRustTextDeltaEvent(event: AgentEvent): event is { TextDelta: string } {
  return typeof (event as { TextDelta?: unknown }).TextDelta === "string";
}

function isRustThinkingDeltaEvent(
  event: AgentEvent,
): event is { ThinkingDelta: string } {
  return typeof (event as { ThinkingDelta?: unknown }).ThinkingDelta === "string";
}

function isRustToolCallStartEvent(
  event: AgentEvent,
): event is { ToolCallStart: { id: string; name: string } } {
  const payload = (event as { ToolCallStart?: unknown }).ToolCallStart;
  return (
    typeof payload === "object" &&
    payload !== null &&
    typeof (payload as { id?: unknown }).id === "string" &&
    typeof (payload as { name?: unknown }).name === "string"
  );
}

function isRustToolCallDeltaEvent(
  event: AgentEvent,
): event is { ToolCallDelta: { id: string; arguments: string } } {
  const payload = (event as { ToolCallDelta?: unknown }).ToolCallDelta;
  return (
    typeof payload === "object" &&
    payload !== null &&
    typeof (payload as { id?: unknown }).id === "string" &&
    typeof (payload as { arguments?: unknown }).arguments === "string"
  );
}

function isRustToolCallCompleteEvent(
  event: AgentEvent,
): event is { ToolCallComplete: { id: string } } {
  const payload = (event as { ToolCallComplete?: unknown }).ToolCallComplete;
  return (
    typeof payload === "object" &&
    payload !== null &&
    typeof (payload as { id?: unknown }).id === "string"
  );
}

export function mapSessionMessagesToMessages(
  sessionMessages: SessionMessageLike[],
): Message[] {
  return sessionMessages.map((message, index) => ({
    id: `history-${index}`,
    role: message.role === "user" ? "user" : "pet",
    text: message.text,
  }));
}

export function getLatestMiniChatMessage(messages: Message[]): Message | null {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message.role === "pet" || message.role === "error") {
      return message;
    }
  }

  return null;
}

export function getMiniChatVisibleMessage({
  messages,
  activeReplyId,
}: {
  messages: Message[];
  activeReplyId: string | null;
}): Message | null {
  if (!activeReplyId) {
    return null;
  }

  return (
    messages.find(
      (message) =>
        message.id === activeReplyId &&
        (message.role === "pet" || message.role === "error"),
    ) ?? null
  );
}

export function getMiniChatReplyDisplayMode(
  message: Message | null,
): MiniChatReplyDisplayMode {
  if (!message) {
    return "compact";
  }

  if (message.role === "error") {
    return "expanded";
  }

  return message.text.trim().length > MINI_CHAT_EXPANDED_TEXT_LENGTH
    ? "expanded"
    : "compact";
}

async function notifyChatSessionChanged() {
  try {
    await emit(CHAT_SESSION_CHANGED_EVENT);
  } catch {
    // Keep chat usable if cross-window sync fails.
  }
}

async function notifyChatSessionNew() {
  try {
    await emit(CHAT_SESSION_NEW_EVENT);
  } catch {
    // Keep chat usable if cross-window sync fails.
  }
}

/** Parse a structured auth_required error from the backend. */
function tryParseAuthRequired(error: unknown): { runtimeId: string } | null {
  if (typeof error !== "string") return null;
  try {
    const parsed = JSON.parse(error) as unknown;
    if (
      parsed !== null &&
      typeof parsed === "object" &&
      "code" in parsed &&
      (parsed as Record<string, unknown>).code === "auth_required" &&
      "runtimeId" in parsed &&
      typeof (parsed as Record<string, unknown>).runtimeId === "string"
    ) {
      return { runtimeId: (parsed as Record<string, unknown>).runtimeId as string };
    }
  } catch {
    // Not JSON — not an auth error.
  }
  return null;
}

export function useChatSession() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isTyping, setIsTyping] = useState(false);
  const [authRequired, setAuthRequired] = useState<{ runtimeId: string } | null>(null);
  const isTypingRef = useRef(false);
  const streamingTextRef = useRef("");
  const streamingIdRef = useRef<string | null>(null);
  const thinkingRef = useRef("");
  const toolCallsRef = useRef<Map<string, ToolCallState>>(new Map());

  useEffect(() => {
    isTypingRef.current = isTyping;
  }, [isTyping]);

  const loadLastSession = useCallback(async () => {
    try {
      const session = await invoke<LastSessionDto | null>("chat_get_last_session");
      if (!session?.messages?.length) {
        setMessages([]);
        return;
      }

      setMessages(mapSessionMessagesToMessages(session.messages));
    } catch {
      // Keep the chat usable even if history loading fails.
    }
  }, []);

  useEffect(() => {
    void loadLastSession();
  }, [loadLastSession]);

  useEffect(() => {
    const unlisten = listen(CHAT_SESSION_CHANGED_EVENT, () => {
      if (!isTypingRef.current) {
        void loadLastSession();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadLastSession]);

  useEffect(() => {
    const unlisten = listen(CHAT_SESSION_NEW_EVENT, () => {
      setMessages([]);
      streamingTextRef.current = "";
      streamingIdRef.current = null;
      thinkingRef.current = "";
      toolCallsRef.current = new Map();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const flushStreaming = useCallback(() => {
    const id = streamingIdRef.current;
    if (!id) {
      return;
    }

    const text = streamingTextRef.current;
    const thinking = thinkingRef.current || undefined;
    const toolCalls = Array.from(toolCallsRef.current.values());

    setMessages((prev) => {
      const index = prev.findIndex((message) => message.id === id);
      if (index === -1) {
        return [...prev, { 
          id, 
          role: "pet", 
          text, 
          streaming: true,
          thinking,
          toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
        }];
      }

      const updated = [...prev];
      updated[index] = { 
        ...updated[index], 
        text, 
        streaming: true,
        thinking,
        toolCalls: toolCalls.length > 0 ? toolCalls : undefined,
      };
      return updated;
    });
  }, []);

  const sendMessage = useCallback(async (rawInput: string) => {
    const input = rawInput.trim();
    if (!input) {
      return false;
    }

    const userMessage: Message = {
      id: Date.now().toString(),
      role: "user",
      text: input,
    };

    setMessages((prev) => [...prev, userMessage]);
    setIsTyping(true);

    streamingTextRef.current = "";
    streamingIdRef.current = (Date.now() + 1).toString();
    thinkingRef.current = "";
    toolCallsRef.current = new Map();

    void emitPetReaction("chat-message");

    const unlisten = await listen<AgentEvent>("agent-event", (ev) => {
      const event = ev.payload;

      if (isRustTextDeltaEvent(event)) {
        streamingTextRef.current += event.TextDelta;
        flushStreaming();
        return;
      }

      if (isRustThinkingDeltaEvent(event)) {
        thinkingRef.current += event.ThinkingDelta;
        flushStreaming();
        return;
      }

      if (isRustToolCallStartEvent(event)) {
        toolCallsRef.current.set(event.ToolCallStart.id, {
          id: event.ToolCallStart.id,
          name: event.ToolCallStart.name,
          status: 'running',
        });
        flushStreaming();
        return;
      }

      if (isRustToolCallDeltaEvent(event)) {
        const tool = toolCallsRef.current.get(event.ToolCallDelta.id);
        if (tool) {
          tool.args = (tool.args || "") + event.ToolCallDelta.arguments;
          flushStreaming();
        }
        return;
      }

      if (isRustToolCallCompleteEvent(event)) {
        const tool = toolCallsRef.current.get(event.ToolCallComplete.id);
        if (tool) {
          tool.status = 'complete';
          tool.result = 'success';
        }
        flushStreaming();
        return;
      }

      // Handle the "Complete" string event (AgentEvent::Complete serializes as just "Complete")
      if (typeof event === "string" && event === "Complete") {
        // Streaming is complete, tool calls will be cleaned up in finally block
        return;
      }

      // Defensive check for malformed events
      if (!event || typeof event !== "object") {
        console.warn("[chat-session] Received malformed event (not an object):", event);
        return;
      }

      if (!("type" in event) || typeof event.type !== "string") {
        // Only warn if it's not one of our known Rust events (which have different structure)
        const isKnownRustEvent = 
          isRustTextDeltaEvent(event) || 
          isRustThinkingDeltaEvent(event) || 
          isRustToolCallStartEvent(event) || 
          isRustToolCallCompleteEvent(event) ||
          isRustToolCallDeltaEvent(event);
        
        if (!isKnownRustEvent) {
          console.warn("[chat-session] Received event without valid type field:", event);
        }
        return;
      }

      switch (event.type) {
        case "tool_execution_start": {
          const toolEvent = event as AgentEventToolStart;
          toolCallsRef.current.set(toolEvent.toolCallId, {
            id: toolEvent.toolCallId,
            name: toolEvent.toolName,
            status: 'running',
          });
          flushStreaming();
          break;
        }

        case "tool_execution_end": {
          const toolEvent = event as AgentEventToolEnd;
          const tool = toolCallsRef.current.get(toolEvent.toolCallId);
          if (tool) {
            tool.status = toolEvent.isError ? 'error' : 'complete';
            tool.result = toolEvent.isError ? 'failure' : 'success';
          }
          flushStreaming();
          break;
        }

        case "message_update": {
          const messageEvent = event as AgentEventMessageUpdate;
          const assistantEvent =
            messageEvent.assistantMessageEvent as AssistantMessageEvent;
          if (assistantEvent?.Text) {
            streamingTextRef.current += assistantEvent.Text.text;
            flushStreaming();
          }
          break;
        }
      }
    });

    void emitPetReaction("ai-processing", { sticky: true });

    try {
      const result = await invoke<{ response: string }>("agent_prompt", {
        message: input,
      });

      setAuthRequired(null);

      const finalId = streamingIdRef.current ?? (Date.now() + 2).toString();

      setMessages((prev) => {
        const index = prev.findIndex((message) => message.id === finalId);
        const existingMessage = index !== -1 ? prev[index] : null;
        const finalMessage: Message = {
          id: finalId,
          role: "pet",
          text: result.response,
          streaming: false,
          // Preserve thinking content but remove tool calls after streaming completes
          thinking: existingMessage?.thinking,
          // toolCalls intentionally omitted - they disappear after streaming
        };

        if (index === -1) {
          return [...prev, finalMessage];
        }

        const updated = [...prev];
        updated[index] = finalMessage;
        return updated;
      });

      await notifyChatSessionChanged();
      void emitPetReaction("agent-result");
      return true;
    } catch (error) {
      // Check for structured auth_required error from the backend.
      const authError = tryParseAuthRequired(error);
      if (authError) {
        setAuthRequired(authError);
      } else {
        setMessages((prev) => [
          ...prev,
          {
            id: (Date.now() + 3).toString(),
            role: "error",
            text: `Error: ${error}`,
          },
        ]);
      }
      void emitPetReaction("agent-result");
      return false;
    } finally {
      unlisten();
      streamingIdRef.current = null;
      setIsTyping(false);
      
      // Clean up completed tools to prevent memory bloat
      // Keep only running tools, remove completed/error ones
      for (const [id, tool] of toolCallsRef.current) {
        if (tool.status === 'complete' || tool.status === 'error') {
          toolCallsRef.current.delete(id);
        }
      }
    }
  }, [flushStreaming]);

  const startNewChat = useCallback(async () => {
    if (isTypingRef.current) {
      return false;
    }

    try {
      await invoke("chat_new_session");
      setMessages([]);
      streamingTextRef.current = "";
      streamingIdRef.current = null;
      thinkingRef.current = "";
      toolCallsRef.current = new Map();
      await notifyChatSessionNew();
      return true;
    } catch {
      return false;
    }
  }, []);

  return {
    messages,
    isTyping,
    authRequired,
    clearAuthRequired: () => setAuthRequired(null),
    loadLastSession,
    sendMessage,
    startNewChat,
  };
}
