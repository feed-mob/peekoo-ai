import { motion } from "framer-motion";
import { User, Bot, AlertCircle } from "lucide-react";
import { Streamdown } from "streamdown";
import type { Message } from "@/types/chat";


interface ChatMessageProps {
  message: Message;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === "user";
  const isError = message.role === "error";

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={`flex items-start gap-2 ${isUser ? "flex-row-reverse" : ""}`}
    >
      <div
        className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${isUser
          ? "bg-glow-blue/20 text-glow-blue"
          : isError
            ? "bg-red-500/20 text-red-400"
            : "bg-glow-purple/20 text-glow-purple"
          }`}
      >
        {isUser ? (
          <User size={16} />
        ) : isError ? (
          <AlertCircle size={16} />
        ) : (
          <Bot size={16} />
        )}
      </div>
      <div
        className={`max-w-[70%] px-4 py-2 rounded-2xl text-sm ${isUser
          ? "bg-glow-blue/20 border border-glow-blue/30 text-text-primary"
          : isError
            ? "bg-red-500/10 border border-red-500/30 text-red-300"
            : "bg-space-surface border border-glass-border text-text-primary"
          }`}
      >
        <Streamdown>{message.text}</Streamdown>
      </div>
    </motion.div>
  );
}
