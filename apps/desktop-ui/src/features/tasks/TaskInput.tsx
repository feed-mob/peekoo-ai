import { useState, FormEvent } from "react";
import { Plus } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import type { Task } from "@/types/task";

interface TaskInputProps {
  onAdd: (title: string, priority: Task["priority"]) => void;
}

export function TaskInput({ onAdd }: TaskInputProps) {
  const [newTask, setNewTask] = useState("");
  const [priority, setPriority] = useState<Task["priority"]>("medium");

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!newTask.trim()) return;
    onAdd(newTask, priority);
    setNewTask("");
    setPriority("medium");
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2">
      <Input
        type="text"
        value={newTask}
        onChange={(e) => setNewTask(e.target.value)}
        placeholder="Add a new task..."
        className="flex-1 bg-space-deep border-glass-border text-text-primary placeholder:text-text-muted"
      />
      <select
        value={priority}
        onChange={(e) => setPriority(e.target.value as Task["priority"])}
        className="px-3 py-2 bg-space-deep border border-glass-border rounded-md text-text-primary text-sm focus:outline-none focus:border-glow-blue"
      >
        <option value="low">Low</option>
        <option value="medium">Medium</option>
        <option value="high">High</option>
      </select>
      <Button
        type="submit"
        size="icon"
        className="bg-success hover:bg-success/80 text-space-void"
      >
        <Plus size={16} />
      </Button>
    </form>
  );
}
