import { useState, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import { Send } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ChatMessage } from "./ChatMessage";
import type { Message } from "@/types/chat";

export function ChatPanel() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

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

    // TODO: Connect to Tauri backend
    setTimeout(() => {
      const petMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: "pet",
        text: "I received your message! AI integration coming soon...",
      };
      setMessages((prev) => [...prev, petMessage]);
      setIsTyping(false);
    }, 1000);
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
            {isTyping && (
              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                className="flex items-start gap-2"
              >
                <span className="text-text-muted text-sm">Typing...</span>
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
