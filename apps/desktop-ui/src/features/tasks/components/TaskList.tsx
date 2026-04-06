import { useState } from "react";
import type { Task, TaskStatus } from "@/types/task";
import { splitTodayTasks } from "../utils/task-grouping";
import { TaskListItem } from "./TaskListItem";
import { DeleteConfirmDialog } from "./DeleteConfirmDialog";
import { useTranslation } from "react-i18next";

interface TaskListProps {
  tasks: Task[];
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
  onStatusChange: (id: string, status: TaskStatus) => void;
  onSelect: (id: string) => void;
  isTodayTab?: boolean;
  isToggling: string | null;
  isUpdating: string | null;
  isDeleting: string | null;
}

export function TaskList({
  tasks,
  onToggle,
  onDelete,
  onStatusChange,
  onSelect,
  isTodayTab = false,
  isToggling,
  isUpdating,
  isDeleting,
}: TaskListProps) {
  const { t } = useTranslation();
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

  const todayGroups = isTodayTab ? splitTodayTasks(tasks) : null;

  const renderTaskItem = (task: Task) => (
    <TaskListItem
      key={task.id}
      task={task}
      onToggle={() => onToggle(task.id)}
      onDelete={() => handleDeleteClick(task)}
      onStatusChange={(status) => onStatusChange(task.id, status)}
      onSelect={() => onSelect(task.id)}
      isTodayTab={isTodayTab}
      isToggling={isToggling === task.id}
      isUpdating={isUpdating === task.id}
      isDeleting={isDeleting === task.id}
    />
  );

  return (
    <>
      <div className="space-y-2 pr-2">
        {todayGroups ? (
          <>
            {todayGroups.overdue.length > 0 && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 px-1 pb-1">
                  <div className="h-px flex-1 bg-[#E5484D]/30" />
                  <span className="text-[10px] font-medium uppercase tracking-[0.18em] text-[#E5484D]/80">
                    {t("tasks.sections.overdue")}
                  </span>
                  <div className="h-px flex-1 bg-[#E5484D]/30" />
                </div>
                {todayGroups.overdue.map(renderTaskItem)}
              </div>
            )}

            {todayGroups.today.length > 0 && todayGroups.overdue.length > 0 && (
              <div className="flex items-center gap-2 px-1 pb-1 pt-1">
                <div className="h-px flex-1 bg-glass-border/60" />
                <span className="text-[10px] font-medium uppercase tracking-[0.18em] text-text-muted/70">
                  {t("tasks.tabs.today")}
                </span>
                <div className="h-px flex-1 bg-glass-border/60" />
              </div>
            )}
            {todayGroups.today.map(renderTaskItem)}

            {todayGroups.unscheduled.length > 0 && (
              <div className="space-y-2 pt-1">
                {(todayGroups.overdue.length > 0 || todayGroups.today.length > 0) && (
                  <div className="flex items-center gap-2 px-1 pb-1">
                    <div className="h-px flex-1 bg-glass-border/60" />
                    <span className="text-[10px] font-medium uppercase tracking-[0.18em] text-text-muted/70">
                      {t("tasks.sections.unscheduled")}
                    </span>
                    <div className="h-px flex-1 bg-glass-border/60" />
                  </div>
                )}
                {todayGroups.unscheduled.map(renderTaskItem)}
              </div>
            )}

            {todayGroups.completed.length > 0 && (
              <div className="pt-2 space-y-2">
                <div className="flex items-center gap-2 px-1 pb-1">
                  <div className="h-px flex-1 bg-glass-border/60" />
                  <span className="text-[10px] font-medium uppercase tracking-[0.18em] text-text-muted/70">
                    {t("tasks.sections.completedToday")}
                  </span>
                  <div className="h-px flex-1 bg-glass-border/60" />
                </div>
                {todayGroups.completed.map(renderTaskItem)}
              </div>
            )}
          </>
        ) : (
          tasks.map(renderTaskItem)
        )}
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
