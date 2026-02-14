import { motion } from "framer-motion";
import { User, Bot } from "lucide-react";
import type { Message } from "@/types/chat";

interface ChatMessageProps {
  message: Message;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === "user";

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={`flex items-start gap-2 ${isUser ? "flex-row-reverse" : ""}`}
    >
      <div
        className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${
          isUser
            ? "bg-glow-blue/20 text-glow-blue"
            : "bg-glow-purple/20 text-glow-purple"
        }`}
      >
        {isUser ? <User size={16} /> : <Bot size={16} />}
      </div>
      <div
        className={`max-w-[70%] px-4 py-2 rounded-2xl text-sm ${
          isUser
            ? "bg-glow-blue/20 border border-glow-blue/30 text-text-primary"
            : "bg-space-surface border border-glass-border text-text-primary"
        }`}
      >
        {message.text}
      </div>
    </motion.div>
  );
}
