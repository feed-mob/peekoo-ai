import { motion } from "framer-motion";
import { Loader2, CheckCircle2, XCircle, Wrench } from "lucide-react";
import type { ToolCallState } from "@/types/chat";

interface ToolCallCardProps {
  tool: ToolCallState;
}

export function ToolCallCard({ tool }: ToolCallCardProps) {
  const isRunning = tool.status === 'running';
  const isError = tool.status === 'error';
  
  const getStatusIcon = () => {
    if (isRunning) {
      return <Loader2 size={16} className="animate-spin text-blue-500 flex-shrink-0" />;
    }
    if (isError) {
      return <XCircle size={16} className="text-red-500 flex-shrink-0" />;
    }
    return <CheckCircle2 size={16} className="text-green-500 flex-shrink-0" />;
  };

  const getStatusText = () => {
    if (isRunning) {
      return (
        <>
          Running <strong className="font-semibold">{tool.name}</strong>
        </>
      );
    }
    return (
      <>
        <strong className="font-semibold">{tool.name}</strong>
        {tool.result && (
          <span className={isError ? "text-red-600 ml-1" : "text-green-600 ml-1"}>
            - {tool.result}
          </span>
        )}
      </>
    );
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: -10, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.2, ease: "easeOut" }}
      className="mb-2 p-3 bg-card border rounded-lg shadow-sm hover:shadow-md transition-shadow"
    >
      <div className="flex items-center gap-2">
        <Wrench size={14} className="text-muted-foreground flex-shrink-0" />
        <div className="flex-1 min-w-0">
          <span className="text-sm text-foreground flex items-center gap-1 flex-wrap">
            {getStatusIcon()}
            <span className="truncate">{getStatusText()}</span>
          </span>
        </div>
      </div>
      
      {/* Collapsible args for debugging */}
      {tool.args && (
        <details className="mt-2 text-xs group">
          <summary className="text-muted-foreground cursor-pointer hover:text-foreground transition-colors flex items-center gap-1 select-none">
            <span className="group-open:rotate-90 transition-transform inline-block">▶</span>
            Arguments
          </summary>
          <div className="mt-1 p-2 bg-muted rounded overflow-x-auto">
            <pre className="text-muted-foreground font-mono text-xs whitespace-pre">
              {(() => {
                try {
                  // Try to pretty-print JSON
                  const parsed = JSON.parse(tool.args || '{}');
                  return JSON.stringify(parsed, null, 2);
                } catch {
                  // Fallback to raw string if not valid JSON
                  return tool.args;
                }
              })()}
            </pre>
          </div>
        </details>
      )}
    </motion.div>
  );
}
