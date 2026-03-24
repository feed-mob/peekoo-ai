import { Checkbox } from "@/components/ui/checkbox";
import { Trash2 } from "lucide-react";
import type { Task } from "@/types/task";
import { motion } from "framer-motion";

interface TaskItemProps {
  task: Task;
  onToggle: () => void;
  onDelete: () => void;
}

const PRIORITY_CONFIG = {
  high: { color: "#E9762B", label: "High" },
  medium: { color: "#8DD9CF", label: "Medium" },
  low: { color: "#7B9AC7", label: "Low" },
} as const;

export function TaskItem({ task, onToggle, onDelete }: TaskItemProps) {
  const { color } = PRIORITY_CONFIG[task.priority];

  return (
    <motion.div
      initial={{ opacity: 0, y: 10, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9, x: -20 }}
      whileHover={{ scale: 1.01, y: -2 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={`group flex items-stretch gap-3 bg-space-surface border border-glass-border rounded-sm shadow-sm hover:shadow-md hover:border-glow-green/40 overflow-hidden transition-all ${
        task.completed ? "opacity-60" : ""
      }`}
    >
      <div className="w-1 shrink-0" style={{ backgroundColor: color }} />

      <div className="flex flex-1 items-center gap-3 py-4 pr-4 min-w-0">
        <Checkbox
          checked={task.completed}
          onCheckedChange={onToggle}
          className="shrink-0 w-5 h-5 data-[state=checked]:bg-[var(--priority-color)] data-[state=checked]:border-[var(--priority-color)]"
          style={{ "--priority-color": color } as React.CSSProperties}
        />
        <span
          className={`flex-1 text-sm font-medium leading-relaxed truncate ${
            task.completed ? "line-through text-text-muted" : "text-text-primary"
          }`}
        >
          {task.title}
        </span>
        <button
          onClick={onDelete}
          className="opacity-40 group-hover:opacity-100 p-2 rounded-lg text-text-muted hover:text-color-danger hover:bg-color-danger/10 transition-all shrink-0"
          aria-label="Delete task"
        >
          <Trash2 size={16} />
        </button>
      </div>
    </motion.div>
  );
}
