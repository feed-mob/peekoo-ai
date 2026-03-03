import { useState, useRef, useEffect, useCallback } from "react";
import { motion } from "framer-motion";
import { Send } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ChatMessage } from "./ChatMessage";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Message,
  AgentEvent,
  AgentEventToolStart,
  AgentEventToolEnd,
  AgentEventMessageUpdate,
  AssistantMessageEvent,
} from "@/types/chat";

export function ChatPanel() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Refs to accumulate streaming content without causing re-renders per token
  const streamingTextRef = useRef("");
  const streamingIdRef = useRef<string | null>(null);
  const toolStatusRef = useRef<Map<string, { name: string; done: boolean }>>(
    new Map()
  );

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  /** Build the streaming message text from accumulated state */
  const buildStreamingText = useCallback(() => {
    let text = "";

    // Show active / completed tool calls
    const tools = toolStatusRef.current;
    if (tools.size > 0) {
      for (const [, tool] of tools) {
        if (tool.done) {
          text += `> ✅ **${tool.name}** — done\n`;
        } else {
          text += `> 🔧 Running **${tool.name}**…\n`;
        }
      }
      text += "\n";
    }

    // Append streamed text
    text += streamingTextRef.current;

    return text;
  }, []);

  /** Push the latest streaming snapshot into React state */
  const flushStreaming = useCallback(() => {
    const id = streamingIdRef.current;
    if (!id) return;
    const text = buildStreamingText();

    setMessages((prev) => {
      const idx = prev.findIndex((m) => m.id === id);
      if (idx === -1) {
        // First time — append
        return [
          ...prev,
          { id, role: "pet", text, streaming: true },
        ];
      }
      // Update in-place
      const updated = [...prev];
      updated[idx] = { ...updated[idx], text, streaming: true };
      return updated;
    });
  }, [buildStreamingText]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    const userMessage: Message = {
      id: Date.now().toString(),
      role: "user",
      text: input,
    };
    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setIsTyping(true);

    // Reset streaming state
    streamingTextRef.current = "";
    streamingIdRef.current = (Date.now() + 1).toString();
    toolStatusRef.current = new Map();

    // Listen for agent events
    const unlisten = await listen<AgentEvent>("agent-event", (ev) => {
      const event = ev.payload;

      switch (event.type) {
        case "tool_execution_start": {
          const e = event as AgentEventToolStart;
          toolStatusRef.current.set(e.toolCallId, {
            name: e.toolName,
            done: false,
          });
          flushStreaming();
          break;
        }

        case "tool_execution_end": {
          const e = event as AgentEventToolEnd;
          if (toolStatusRef.current.has(e.toolCallId)) {
            toolStatusRef.current.set(e.toolCallId, {
              name: e.toolName,
              done: true,
            });
          }
          flushStreaming();
          break;
        }

        case "message_update": {
          const e = event as AgentEventMessageUpdate;
          const ame = e.assistantMessageEvent as AssistantMessageEvent;
          if (ame?.Text) {
            streamingTextRef.current += ame.Text.text;
            flushStreaming();
          }
          break;
        }
      }
    });

    try {
      const result = await invoke<{ response: string }>("agent_prompt", {
        message: input,
      });

      // Replace the streaming message with the final clean response
      const finalId = streamingIdRef.current!;
      setMessages((prev) => {
        const idx = prev.findIndex((m) => m.id === finalId);
        const finalMsg: Message = {
          id: finalId,
          role: "pet",
          text: result.response,
          streaming: false,
        };
        if (idx === -1) {
          return [...prev, finalMsg];
        }
        const updated = [...prev];
        updated[idx] = finalMsg;
        return updated;
      });
    } catch (err) {
      const errorMessage: Message = {
        id: (Date.now() + 2).toString(),
        role: "error",
        text: `Error: ${err}`,
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      unlisten();
      streamingIdRef.current = null;
      setIsTyping(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <ScrollArea className="flex-1 mb-4">
        {messages.length === 0 ? (
          <div className="text-center text-text-muted py-8 italic">
            Start chatting with your Peekoo pet!
          </div>
        ) : (
          <div className="space-y-3 pr-4">
            {messages.map((msg) => (
              <ChatMessage key={msg.id} message={msg} />
            ))}
            {isTyping && !streamingIdRef.current && (
              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                className="flex items-start gap-2"
              >
                <span className="text-text-muted text-sm">Thinking...</span>
              </motion.div>
            )}
          </div>
        )}
        <div ref={messagesEndRef} />
      </ScrollArea>

      <form onSubmit={handleSubmit} className="flex gap-2">
        <Input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Type a message..."
          disabled={isTyping}
          className="flex-1 bg-space-deep border-glass-border text-text-primary placeholder:text-text-muted"
        />
        <Button
          type="submit"
          disabled={isTyping || !input.trim()}
          size="icon"
          className="bg-glow-blue hover:bg-glow-blue/80 text-space-void"
        >
          <Send size={16} />
        </Button>
      </form>
    </div>
  );
}
