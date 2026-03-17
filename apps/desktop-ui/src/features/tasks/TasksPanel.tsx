import { useState, useMemo } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CheckCircle2 } from "lucide-react";
import type { Task } from "@/types/task";
import { emitPetReaction } from "@/lib/pet-events";
import { TaskItem } from "./TaskItem";
import { TaskInput } from "./TaskInput";

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      <CheckCircle2 size={48} className="text-text-muted/40 mb-3" />
      <p className="text-sm font-medium text-text-primary mb-1">No tasks yet</p>
      <p className="text-xs text-text-muted">Add your first task to get started</p>
    </div>
  );
}

export function TasksPanel() {
  const [tasks, setTasks] = useState<Task[]>([
    { id: "1", title: "Complete project documentation", completed: false, priority: "high" },
    { id: "2", title: "Review pull requests", completed: true, priority: "medium" },
    { id: "3", title: "Update dependencies", completed: false, priority: "low" },
  ]);

  const stats = useMemo(() => {
    const total = tasks.length;
    const completed = tasks.filter(t => t.completed).length;
    return { total, completed };
  }, [tasks]);

  const handleAddTask = (title: string, priority: Task["priority"]) => {
    const task: Task = {
      id: Date.now().toString(),
      title,
      completed: false,
      priority,
    };
    setTasks([...tasks, task]);
  };

  const toggleTask = (id: string) => {
    const shouldCelebrate = tasks.some((task) => task.id === id && !task.completed);

    setTasks(tasks.map(task =>
      task.id === id ? { ...task, completed: !task.completed } : task
    ));

    if (shouldCelebrate) {
      void emitPetReaction("task-completed");
    }
  };

  const deleteTask = (id: string) => {
    setTasks(tasks.filter(task => task.id !== id));
  };

  return (
    <div className="flex flex-col h-full gap-4">
      {/* Header with stats */}
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold text-text-primary">Tasks</h2>
        <span className="text-xs text-text-muted font-medium">
          {stats.completed} / {stats.total} completed
        </span>
      </div>

      {/* Input */}
      <TaskInput onAdd={handleAddTask} />
      
      {/* Task List */}
      <ScrollArea className="flex-1 -mx-1 px-1">
        {tasks.length === 0 ? (
          <EmptyState />
        ) : (
          <div className="space-y-3 pr-2">
            {tasks.map((task) => (
              <TaskItem
                key={task.id}
                task={task}
                onToggle={() => toggleTask(task.id)}
                onDelete={() => deleteTask(task.id)}
              />
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}
