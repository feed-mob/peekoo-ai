import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Trash2 } from "lucide-react";
import type { Task } from "@/types/task";

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
    <div
      className={`flex items-center gap-3 p-3 bg-space-surface border border-glass-border rounded-lg hover:bg-space-overlay transition-colors ${
        task.completed ? "opacity-60" : ""
      }`}
    >
      <Checkbox
        checked={task.completed}
        onCheckedChange={onToggle}
        className="shrink-0 border-glass-border data-[state=checked]:bg-glow-blue data-[state=checked]:border-glow-blue"
      />
      <span
        className={`flex-1 text-sm ${
          task.completed ? "line-through text-text-muted" : "text-text-primary"
        }`}
      >
        {task.title}
      </span>
      <Badge
        variant="outline"
        className={`text-xs capitalize shrink-0 ${getPriorityColor(task.priority)}`}
      >
        {task.priority}
      </Badge>
      <button
        onClick={onDelete}
        className="text-text-muted hover:text-danger transition-colors shrink-0"
        title="Delete task"
      >
        <Trash2 size={16} />
      </button>
    </div>
  );
}
