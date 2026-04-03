import { motion } from "framer-motion";
import { User, Bot, AlertCircle } from "lucide-react";
import { Streamdown } from "streamdown";
import type { Message } from "@/types/chat";
import { ThinkingBlock } from "./ThinkingBlock";
import { ToolCallCard } from "./ToolCallCard";


interface ChatMessageProps {
  message: Message;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === "user";
  const isError = message.role === "error";
  const isPet = message.role === "pet";

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
      
      {/* Message content container */}
      <div className={`flex flex-col gap-2 ${isUser ? "items-end" : "items-start"} max-w-[85%]`}>
        {/* Thinking Block - only for pet messages */}
        {isPet && message.thinking && (
          <div className="w-full">
            <ThinkingBlock content={message.thinking} />
          </div>
        )}

        {/* Tool Call Cards - only for pet messages */}
        {isPet && message.toolCalls && message.toolCalls.length > 0 && (
          <div className="w-full space-y-1">
            {message.toolCalls.map((tool) => (
              <ToolCallCard key={tool.id} tool={tool} />
            ))}
          </div>
        )}

        {/* Message bubble */}
        <div
          className={`px-4 py-2.5 text-sm shadow-sm overflow-x-auto ${
            isUser
              ? "bg-accent-peach/20 border border-accent-peach/40 text-text-primary rounded-2xl rounded-tr-sm"
              : isError
              ? "bg-color-danger/15 border border-color-danger/40 text-text-primary rounded-2xl rounded-tl-sm"
              : "bg-glow-sage/20 border border-glow-sage/40 text-text-primary rounded-2xl rounded-tl-sm"
          }`}
        >
          <Streamdown>{message.text}</Streamdown>
          {message.streaming && (
            <span className="inline-block w-2 h-4 ml-1 bg-current animate-pulse" />
          )}
        </div>
      </div>
    </motion.div>
  );
}
