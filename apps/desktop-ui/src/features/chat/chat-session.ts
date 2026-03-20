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
} from "@/types/chat";

export interface SessionMessageLike {
  role: string;
  text: string;
}

export type MiniChatReplyDisplayMode = "compact" | "expanded";

const CHAT_SESSION_CHANGED_EVENT = "chat-session:changed";
const MINI_CHAT_EXPANDED_TEXT_LENGTH = 72;

function buildStreamingText(
  streamingText: string,
  tools: Map<string, { name: string; done: boolean }>,
) {
  let text = "";

  if (tools.size > 0) {
    for (const [, tool] of tools) {
      if (tool.done) {
        text += `> ✅ **${tool.name}** - done\n`;
      } else {
        text += `> 🔧 Running **${tool.name}**...\n`;
      }
    }

    text += "\n";
  }

  return text + streamingText;
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

export function useChatSession() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isTyping, setIsTyping] = useState(false);
  const isTypingRef = useRef(false);
  const streamingTextRef = useRef("");
  const streamingIdRef = useRef<string | null>(null);
  const toolStatusRef = useRef<Map<string, { name: string; done: boolean }>>(
    new Map(),
  );

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

  const flushStreaming = useCallback(() => {
    const id = streamingIdRef.current;
    if (!id) {
      return;
    }

    const text = buildStreamingText(streamingTextRef.current, toolStatusRef.current);

    setMessages((prev) => {
      const index = prev.findIndex((message) => message.id === id);
      if (index === -1) {
        return [...prev, { id, role: "pet", text, streaming: true }];
      }

      const updated = [...prev];
      updated[index] = { ...updated[index], text, streaming: true };
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
    toolStatusRef.current = new Map();

    void emitPetReaction("chat-message");

    const unlisten = await listen<AgentEvent>("agent-event", (ev) => {
      const event = ev.payload;

      switch (event.type) {
        case "tool_execution_start": {
          const toolEvent = event as AgentEventToolStart;
          toolStatusRef.current.set(toolEvent.toolCallId, {
            name: toolEvent.toolName,
            done: false,
          });
          flushStreaming();
          break;
        }

        case "tool_execution_end": {
          const toolEvent = event as AgentEventToolEnd;
          if (toolStatusRef.current.has(toolEvent.toolCallId)) {
            toolStatusRef.current.set(toolEvent.toolCallId, {
              name: toolEvent.toolName,
              done: true,
            });
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

      const finalId = streamingIdRef.current ?? (Date.now() + 2).toString();

      setMessages((prev) => {
        const index = prev.findIndex((message) => message.id === finalId);
        const finalMessage: Message = {
          id: finalId,
          role: "pet",
          text: result.response,
          streaming: false,
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
      setMessages((prev) => [
        ...prev,
        {
          id: (Date.now() + 3).toString(),
          role: "error",
          text: `Error: ${error}`,
        },
      ]);
      void emitPetReaction("agent-result");
      return false;
    } finally {
      unlisten();
      streamingIdRef.current = null;
      setIsTyping(false);
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
      toolStatusRef.current = new Map();
      await notifyChatSessionChanged();
      return true;
    } catch {
      return false;
    }
  }, []);

  return {
    messages,
    isTyping,
    loadLastSession,
    sendMessage,
    startNewChat,
  };
}
