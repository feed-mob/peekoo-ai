import { useState, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import { MessageSquarePlus, Send, Settings2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ChatMessage } from "./ChatMessage";
import { ChatSettingsPanel } from "./settings/ChatSettingsPanel";
import { useChatSession } from "./chat-session";
import { useTranslation } from "react-i18next";

export function ChatPanel() {
  const { t } = useTranslation();
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
    <div className="flex min-h-0 h-full flex-col">
      <div className="mb-3 shrink-0 space-y-3">
        <div className="flex justify-end gap-2">
          <Button
            type="button"
            variant="glass"
            size="sm"
            disabled={isTyping}
            onClick={() => void handleNewChat()}
          >
            <MessageSquarePlus size={14} />
            {t("chat.newChat")}
          </Button>
          <Button
            type="button"
            variant="glass"
            size="sm"
            onClick={() => setShowSettings((prev) => !prev)}
          >
            <Settings2 size={14} />
            {showSettings ? t("chat.hideSettings") : t("chat.settings")}
          </Button>
        </div>

        {showSettings && (
          <ChatSettingsPanel onClose={() => setShowSettings(false)} />
        )}
      </div>

      <ScrollArea className="mb-4 min-h-0 flex-1">
        {messages.length === 0 ? (
          <div className="text-center text-text-muted py-8 italic">
            {t("chat.empty")}
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
                <span className="text-text-muted text-sm">{t("chat.thinking")}</span>
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
          placeholder={t("chat.inputPlaceholder")}
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
