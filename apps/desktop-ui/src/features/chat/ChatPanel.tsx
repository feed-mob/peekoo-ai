import { useState, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import { MessageSquarePlus, Send, Settings2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ChatMessage } from "./ChatMessage";
import { ChatSettingsPanel } from "./settings/ChatSettingsPanel";
import { useChatSession } from "./chat-session";

export function ChatPanel() {
  const [input, setInput] = useState("");
  const [showSettings, setShowSettings] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const { messages, isTyping, sendMessage, startNewChat } = useChatSession();

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) {
      return;
    }

    const didSend = await sendMessage(input);
    if (didSend) {
      setInput("");
    }
  };

  const handleNewChat = async () => {
    const didStart = await startNewChat();
    if (didStart) {
      setInput("");
    }
  };

  return (
    <div className="flex flex-col h-full">
      <ScrollArea className="flex-1 mb-4">
        <div className="mb-3 flex justify-end gap-2">
          <Button
            type="button"
            variant="glass"
            size="sm"
            disabled={isTyping}
            onClick={() => void handleNewChat()}
          >
            <MessageSquarePlus size={14} />
            New Chat
          </Button>
          <Button
            type="button"
            variant="glass"
            size="sm"
            onClick={() => setShowSettings((prev) => !prev)}
          >
            <Settings2 size={14} />
            {showSettings ? "Hide Settings" : "Settings"}
          </Button>
        </div>

        {showSettings && (
          <div className="mb-4">
            <ChatSettingsPanel onClose={() => setShowSettings(false)} />
          </div>
        )}

        {messages.length === 0 ? (
          <div className="text-center text-text-muted py-8 italic">
            Start chatting with your Peekoo pet!
          </div>
        ) : (
          <div className="space-y-3 pr-4">
            {messages.map((msg) => (
              <ChatMessage key={msg.id} message={msg} />
            ))}
            {isTyping && messages.every((message) => !message.streaming) && (
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
          variant="gradient"
        >
          <Send size={16} />
        </Button>
      </form>
    </div>
  );
}
