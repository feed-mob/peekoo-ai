import { useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { Task } from "@/types/task";
import { TaskItem } from "./TaskItem";
import { TaskInput } from "./TaskInput";

export function TasksPanel() {
  const [tasks, setTasks] = useState<Task[]>([
    { id: "1", title: "Complete project documentation", completed: false, priority: "high" },
    { id: "2", title: "Review pull requests", completed: true, priority: "medium" },
    { id: "3", title: "Update dependencies", completed: false, priority: "low" },
  ]);

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
    setTasks(tasks.map(task =>
      task.id === id ? { ...task, completed: !task.completed } : task
    ));
  };

  const deleteTask = (id: string) => {
    setTasks(tasks.filter(task => task.id !== id));
  };

  return (
    <div className="flex flex-col h-full">
      <TaskInput onAdd={handleAddTask} />
      
      <ScrollArea className="flex-1 mt-4">
        <div className="space-y-2 pr-2">
          {tasks.map((task) => (
            <TaskItem
              key={task.id}
              task={task}
              onToggle={() => toggleTask(task.id)}
              onDelete={() => deleteTask(task.id)}
            />
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
