import { Checkbox } from "@/components/ui/checkbox";
import { Trash2, User, Bot, Calendar, Repeat, Clock } from "lucide-react";
import type { Task } from "@/types/task";
import { TaskLabelPills } from "./TaskLabelPills";
import { PRIORITY_CONFIG, STATUS_CONFIG, formatTimeRange } from "../utils/task-formatting";
import { isOverdue } from "../utils/date-helpers";
import { getAgentWorkStatusBadge } from "../utils/task-agent-work";
import { shouldShowAgentExecutingIndicator } from "../utils/task-agent-work-display";
import { getDoneTaskVisualStyle } from "../utils/task-visuals";
import { LoadingSpinner } from "./LoadingSpinner";

interface TaskListItemProps {
  task: Task;
  onToggle: () => void;
  onDelete: () => void;
  onStatusChange: (status: Task["status"]) => void;
  onSelect: () => void;
  isTodayTab?: boolean;
  isToggling?: boolean;
  isUpdating?: boolean;
  isDeleting?: boolean;
}

export function TaskListItem({
  task,
  onToggle,
  onDelete,
  onStatusChange,
  onSelect,
  isTodayTab = false,
  isToggling = false,
  isUpdating = false,
  isDeleting = false,
}: TaskListItemProps) {
  const priority = PRIORITY_CONFIG[task.priority];
  const status = STATUS_CONFIG[task.status];
  const isDone = task.status === "done";

  // FIXED: Now passing all 4 arguments correctly
  const timeLabel = formatTimeRange(
    task.scheduled_start_at,
    task.scheduled_end_at,
    task.recurrence_rule,
    task.recurrence_time_of_day
  );

  const overdue = isOverdue(task.scheduled_start_at, task.status);
  const agentWorkBadge = getAgentWorkStatusBadge(task.agent_work_status);
  const showExecutingIndicator = shouldShowAgentExecutingIndicator(task);
  const doneTaskVisualStyle = getDoneTaskVisualStyle(isDone, isTodayTab);

  return (
    <div
      className={`group flex items-stretch gap-2 bg-space-surface border border-glass-border rounded-sm shadow-sm hover:shadow-md hover:border-glow-green/40 overflow-hidden transition-all ${doneTaskVisualStyle} ${overdue ? "border-l-2 border-l-[#E5484D]" : ""} ${
        isDeleting ? "opacity-50" : ""
      }`}
    >
      {/* Priority color bar */}
      <div
        className="w-1.5 shrink-0"
        style={{ backgroundColor: priority.color }}
      />

      {/* Content */}
      <div
        className="flex flex-1 items-start gap-2 py-3.5 pr-3 min-w-0 cursor-pointer"
        onClick={onSelect}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onSelect();
          }
        }}
      >
        {/* Priority dot + checkbox */}
        <div className="flex flex-col items-center gap-1.5 shrink-0">
          <div
            className="w-2 h-2 rounded-full shrink-0"
            style={{ backgroundColor: priority.dotColor }}
            title={`${priority.label} priority`}
          />
          {isToggling ? (
            <LoadingSpinner size="sm" />
          ) : (
            <Checkbox
              checked={isDone}
              onCheckedChange={() => onToggle()}
              onClick={(e) => e.stopPropagation()}
              className="shrink-0 w-5 h-5 data-[state=checked]:bg-[var(--priority-color)] data-[state=checked]:border-[var(--priority-color)]"
              style={{ "--priority-color": priority.color } as React.CSSProperties}
            />
          )}
        </div>

        <div className="flex-1 min-w-0">
          {/* Title row with status badge and assignee */}
          <div className="flex items-center gap-2">
            <span
              className={`flex-1 text-[13px] font-medium leading-relaxed truncate ${
                isDone ? "line-through text-text-muted" : "text-text-primary"
              }`}
            >
              {task.title}
            </span>

            {showExecutingIndicator && (
              <span
                className="inline-flex items-center gap-1 rounded-full border border-blue-400/30 bg-blue-500/10 px-1.5 py-0.5 text-[9px] font-semibold text-blue-300"
                title="Agent is currently working on this task"
              >
                <span className="h-1.5 w-1.5 rounded-full bg-blue-400 animate-pulse" />
                Live
              </span>
            )}

            {/* Recurrence icon */}
            {task.recurrence_rule && (
              <Repeat size={13} className="shrink-0 text-text-muted/60" />
            )}

            {/* Assignee icon */}
            {task.assignee === "user" ? (
              <User size={14} className="shrink-0 text-text-muted" />
            ) : (
              <Bot size={14} className="shrink-0 text-glow-olive dark:text-glow-mint" />
            )}
          </div>

          {/* Time + Labels row */}
          <div className="flex items-center gap-2 mt-1 flex-wrap">
            {timeLabel && (
              <span
                className={`inline-flex items-center gap-1 text-[10px] font-medium ${
                  overdue ? "text-accent-orange" : "text-text-muted"
                }`}
              >
                {overdue ? <Clock size={10} /> : <Calendar size={10} />}
                {timeLabel}
              </span>
            )}
            {task.estimated_duration_min && (
              <span className="text-[10px] text-text-muted">
                {task.estimated_duration_min}m
              </span>
            )}
            {agentWorkBadge && (
              <span
                className="inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-semibold"
                style={{
                  backgroundColor: `${agentWorkBadge.color}20`,
                  color: agentWorkBadge.color,
                  border: `1px solid ${agentWorkBadge.color}40`,
                }}
                title={`Agent work status: ${agentWorkBadge.label}`}
              >
                <span
                  className={`h-1.5 w-1.5 rounded-full ${agentWorkBadge.animated ? "animate-pulse" : ""}`}
                  style={{ backgroundColor: agentWorkBadge.color }}
                />
                {agentWorkBadge.label}
              </span>
            )}
            <TaskLabelPills labels={task.labels} />
          </div>
        </div>

        {/* Status badge (click to cycle) */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            onStatusChange(status.next);
          }}
          disabled={isToggling || isUpdating}
          className="shrink-0 px-2 py-0.5 rounded-full text-[10px] font-semibold leading-tight transition-colors hover:brightness-125 disabled:opacity-50"
          style={{
            backgroundColor: `${status.color}15`,
            color: status.color,
            border: `1px solid ${status.color}30`,
          }}
          title={`Click to move to ${STATUS_CONFIG[status.next].label}`}
        >
          {status.label}
        </button>

        {/* Delete */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          disabled={isDeleting}
          className="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg text-text-muted hover:text-accent-orange hover:bg-accent-orange/10 transition-all shrink-0 disabled:opacity-50"
          aria-label="Delete task"
        >
          {isDeleting ? (
            <LoadingSpinner size="sm" />
          ) : (
            <Trash2 size={14} />
          )}
        </button>
      </div>
    </div>
  );
}
