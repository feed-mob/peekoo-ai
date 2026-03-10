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
      initial={{ opacity: 0, scale: 0.95, y: 10 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={`flex items-start gap-3 ${isUser ? "flex-row-reverse" : ""}`}
    >
      <div
        className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 shadow-sm ${isUser
          ? "bg-glow-blue text-space-void"
          : isError
            ? "bg-danger text-space-void"
            : "bg-glow-purple text-space-void"
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
        className={`max-w-[75%] px-4 py-2.5 text-sm shadow-sm ${isUser
          ? "bg-glow-blue/15 border border-glow-blue/20 text-text-primary rounded-2xl rounded-tr-sm"
          : isError
            ? "bg-danger/10 border border-danger/20 text-danger rounded-2xl rounded-tl-sm"
            : "bg-space-surface border border-glass-border text-text-primary rounded-2xl rounded-tl-sm"
          }`}
      >
        <Streamdown>{message.text}</Streamdown>
      </div>
    </motion.div>
  );
}
