import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Trash2 } from "lucide-react";
import type { Task } from "@/types/task";
import { motion } from "framer-motion";

interface TaskItemProps {
  task: Task;
  onToggle: () => void;
  onDelete: () => void;
}

export function TaskItem({ task, onToggle, onDelete }: TaskItemProps) {
  const getPriorityColor = (priority: Task["priority"]) => {
    switch (priority) {
      case "high": return "bg-danger/20 text-danger border-danger/30";
      case "medium": return "bg-warning/20 text-warning border-warning/30";
      case "low": return "bg-success/20 text-success border-success/30";
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 10, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      whileHover={{ scale: 1.01 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={`group flex items-center gap-3 p-3.5 bg-space-surface border border-glass-border rounded-xl shadow-sm hover:shadow-md hover:border-glow-blue/30 transition-all ${
        task.completed ? "opacity-50 grayscale-[50%]" : ""
      }`}
    >
      <Checkbox
        checked={task.completed}
        onCheckedChange={onToggle}
        className="shrink-0 data-[state=checked]:bg-glow-blue data-[state=checked]:border-glow-blue"
      />
      <span
        className={`flex-1 text-sm font-medium ${
          task.completed ? "line-through text-text-muted" : "text-text-primary"
        }`}
      >
        {task.title}
      </span>
      <Badge
        variant="outline"
        className={`text-[10px] font-bold uppercase tracking-wider shrink-0 ${getPriorityColor(task.priority)}`}
      >
        {task.priority}
      </Badge>
      <button
        onClick={onDelete}
        className="opacity-0 group-hover:opacity-100 p-1.5 rounded-md text-text-muted hover:text-danger hover:bg-danger/10 transition-all shrink-0"
        title="Delete task"
      >
        <Trash2 size={16} />
      </button>
    </motion.div>
  );
}
