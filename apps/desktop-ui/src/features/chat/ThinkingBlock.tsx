import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, ChevronRight, Brain } from "lucide-react";

interface ThinkingBlockProps {
  content: string;
  defaultExpanded?: boolean;
}

export function ThinkingBlock({ content, defaultExpanded = false }: ThinkingBlockProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);
  
  // Don't render if content is empty or just whitespace
  if (!content || !content.trim()) {
    return null;
  }

  return (
    <div className="mb-3 border-l-2 border-muted-foreground/30 bg-muted/30 rounded-r-lg overflow-hidden">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 w-full px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors hover:bg-muted/50"
      >
        {isExpanded ? (
          <ChevronDown size={16} className="flex-shrink-0" />
        ) : (
          <ChevronRight size={16} className="flex-shrink-0" />
        )}
        <Brain size={14} className="flex-shrink-0 opacity-70" />
        <span className="italic font-medium">Thinking...</span>
      </button>
      
      <AnimatePresence initial={false}>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeInOut" }}
          >
            <div className="px-3 pb-3">
              <pre className="text-xs text-muted-foreground whitespace-pre-wrap font-mono leading-relaxed">
                {content}
              </pre>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
