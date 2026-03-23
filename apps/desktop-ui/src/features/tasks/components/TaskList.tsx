import { useState } from "react";
import type { Task, TaskStatus } from "@/types/task";
import { TaskListItem } from "./TaskListItem";
import { DeleteConfirmDialog } from "./DeleteConfirmDialog";

interface TaskListProps {
  tasks: Task[];
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
  onStatusChange: (id: string, status: TaskStatus) => void;
  onSelect: (id: string) => void;
  isToggling: string | null;
  isDeleting: string | null;
}

export function TaskList({
  tasks,
  onToggle,
  onDelete,
  onStatusChange,
  onSelect,
  isToggling,
  isDeleting,
}: TaskListProps) {
  const [deleteDialog, setDeleteDialog] = useState<{
    isOpen: boolean;
    taskId: string | null;
    taskTitle: string;
  }>({
    isOpen: false,
    taskId: null,
    taskTitle: "",
  });

  const handleDeleteClick = (task: Task) => {
    setDeleteDialog({
      isOpen: true,
      taskId: task.id,
      taskTitle: task.title,
    });
  };

  const handleDeleteConfirm = () => {
    if (deleteDialog.taskId) {
      onDelete(deleteDialog.taskId);
    }
    setDeleteDialog({ isOpen: false, taskId: null, taskTitle: "" });
  };

  return (
    <>
      <div className="space-y-2 pr-2">
        {tasks.map((task) => (
          <TaskListItem
            key={task.id}
            task={task}
            onToggle={() => onToggle(task.id)}
            onDelete={() => handleDeleteClick(task)}
            onStatusChange={(status) => onStatusChange(task.id, status)}
            onSelect={() => onSelect(task.id)}
            isToggling={isToggling === task.id}
            isDeleting={isDeleting === task.id}
          />
        ))}
      </div>

      <DeleteConfirmDialog
        isOpen={deleteDialog.isOpen}
        taskTitle={deleteDialog.taskTitle}
        onConfirm={handleDeleteConfirm}
        onCancel={() =>
          setDeleteDialog({ isOpen: false, taskId: null, taskTitle: "" })
        }
        isDeleting={deleteDialog.taskId ? isDeleting === deleteDialog.taskId : false}
      />
    </>
  );
}
