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
      {/* Avatar with solid background */}
      <div
        className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 shadow-md ${
          isUser
            ? "bg-accent-orange"
            : isError
            ? "bg-color-danger"
            : "bg-accent-blue"
        }`}
      >
        {isUser ? (
          <User size={16} className="text-white" />
        ) : isError ? (
          <AlertCircle size={16} className="text-white" />
        ) : (
          <Bot size={16} className="text-white" />
        )}
      </div>
      
      {/* Message bubble */}
      <div
        className={`max-w-[85%] px-4 py-2.5 text-sm shadow-sm overflow-x-auto ${
          isUser
            ? "bg-accent-peach/20 border border-accent-peach/40 text-text-primary rounded-2xl rounded-tr-sm"
            : isError
            ? "bg-color-danger/15 border border-color-danger/40 text-text-primary rounded-2xl rounded-tl-sm"
            : "bg-glow-sage/20 border border-glow-sage/40 text-text-primary rounded-2xl rounded-tl-sm"
        }`}
      >
        <Streamdown>{message.text}</Streamdown>
      </div>
    </motion.div>
  );
}
