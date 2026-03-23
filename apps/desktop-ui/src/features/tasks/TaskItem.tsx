import { Checkbox } from "@/components/ui/checkbox";
import { Trash2, User, Bot, GripVertical } from "lucide-react";
import type { Task, TaskStatus } from "@/types/task";
import { motion } from "framer-motion";
import { TaskLabels } from "./TaskLabels";

interface TaskItemProps {
  task: Task;
  onToggle: () => void;
  onDelete: () => void;
  onStatusChange: (status: TaskStatus) => void;
}

const PRIORITY_CONFIG = {
  high:   { color: "#E9762B", label: "High" },
  medium: { color: "#F5C842", label: "Medium" },
  low:    { color: "#7B9AC7", label: "Low" },
} as const;

const STATUS_CONFIG: Record<TaskStatus, { color: string; label: string; next: TaskStatus }> = {
  todo:        { color: "#7B9AC7", label: "Todo",        next: "in_progress" },
  in_progress: { color: "#F5C842", label: "In Progress", next: "done" },
  done:        { color: "#30A46C", label: "Done",        next: "todo" },
};

export function TaskItem({ task, onToggle, onDelete, onStatusChange }: TaskItemProps) {
  const priority = PRIORITY_CONFIG[task.priority];
  const status = STATUS_CONFIG[task.status];
  const isDone = task.status === "done";

  return (
    <motion.div
      initial={{ opacity: 0, y: 10, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9, x: -20 }}
      whileHover={{ scale: 1.01, y: -2 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={`group flex items-stretch gap-2 bg-space-surface border border-glass-border rounded-sm shadow-sm hover:shadow-md hover:border-glow-green/40 overflow-hidden transition-all ${
        isDone ? "opacity-60" : ""
      }`}
    >
      {/* Priority color bar */}
      <div className="w-1 shrink-0" style={{ backgroundColor: priority.color }} />

      {/* Drag handle */}
      <div className="flex items-center opacity-0 group-hover:opacity-30 transition-opacity cursor-grab shrink-0 pl-1">
        <GripVertical size={14} />
      </div>

      {/* Content */}
      <div className="flex flex-1 items-start gap-2 py-3 pr-3 min-w-0">
        <Checkbox
          checked={isDone}
          onCheckedChange={onToggle}
          className="shrink-0 w-5 h-5 mt-0.5 data-[state=checked]:bg-[var(--priority-color)] data-[state=checked]:border-[var(--priority-color)]"
          style={{ "--priority-color": priority.color } as React.CSSProperties}
        />

        <div className="flex-1 min-w-0">
          {/* Title row with status badge and assignee */}
          <div className="flex items-center gap-2">
            <span
              className={`flex-1 text-sm font-medium leading-relaxed truncate ${
                isDone ? "line-through text-text-muted" : "text-text-primary"
              }`}
            >
              {task.title}
            </span>

            {/* Assignee icon */}
            {task.assignee === "agent" ? (
              <Bot size={14} className="shrink-0 text-purple-400" />
            ) : (
              <User size={14} className="shrink-0 text-text-muted" />
            )}
          </div>

          {/* Labels */}
          <TaskLabels labels={task.labels} />
        </div>

        {/* Status badge (click to cycle) */}
        <button
          onClick={() => onStatusChange(status.next)}
          className="shrink-0 px-2 py-0.5 rounded-full text-[10px] font-semibold leading-tight transition-colors hover:brightness-125"
          style={{
            backgroundColor: `${status.color}20`,
            color: status.color,
            border: `1px solid ${status.color}40`,
          }}
          title={`Click to move to ${STATUS_CONFIG[status.next].label}`}
        >
          {status.label}
        </button>

        {/* Delete */}
        <button
          onClick={onDelete}
          className="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg text-text-muted hover:text-color-danger hover:bg-color-danger/10 transition-all shrink-0"
          aria-label="Delete task"
        >
          <Trash2 size={14} />
        </button>
      </div>
    </motion.div>
  );
}
