import { useState } from "react";
import { ArrowLeft, Trash2, Calendar, User, Bot } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { Task } from "@/types/task";
import { PREDEFINED_LABELS, KNOWN_AGENTS } from "@/types/task";
import { TaskLabelPills } from "./TaskLabelPills";
import { TaskActivitySection } from "./TaskActivitySection";
import { DeleteConfirmDialog } from "./DeleteConfirmDialog";
import { LoadingSpinner } from "./LoadingSpinner";
import {
  PRIORITY_CONFIG,
  RECURRENCE_OPTIONS,
  STATUS_CONFIG,
  TASK_STATUS_OPTIONS,
  TIME_OPTIONS,
  formatRecurrenceDisplay,
} from "../utils/task-formatting";
import { getAgentWorkStatusBadge } from "../utils/task-agent-work";
import {
  getAgentFailureDetail,
  shouldShowAgentExecutingIndicator,
} from "../utils/task-agent-work-display";
import {
  toDateInputValue,
  toTimeInputValue,
  fromDateTimeLocal,
} from "../utils/date-helpers";

interface TaskDetailViewProps {
  task: Task;
  onBack: () => void;
  onUpdate: (fields: Partial<Task>) => void;
  onToggle: () => void;
  onDelete: () => void;
  isUpdating?: boolean;
  isDeleting?: boolean;
}

function isRecurring(task: Task): boolean {
  return !!task.recurrence_rule;
}

export function TaskDetailView({
  task,
  onBack,
  onUpdate,
  onToggle,
  onDelete,
  isUpdating = false,
  isDeleting = false,
}: TaskDetailViewProps) {
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState(task.title);
  const [descDraft, setDescDraft] = useState(task.description || "");
  const [scheduleOpen, setScheduleOpen] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  const recurring = isRecurring(task);

  const handleTitleBlur = () => {
    setEditingTitle(false);
    if (titleDraft.trim() && titleDraft !== task.title) {
      onUpdate({ title: titleDraft.trim() });
    }
  };

  const handleDescBlur = () => {
    if (descDraft !== (task.description || "")) {
      onUpdate({ description: descDraft || null });
    }
  };

  const handleRecurrenceChange = (rule: string) => {
    console.log("[handleRecurrenceChange] Selected rule:", rule);
    const newRule = rule === "__none__" ? null : rule;
    const newTime =
      newRule && !task.recurrence_time_of_day
        ? "09:00"
        : task.recurrence_time_of_day || null;
    console.log("[handleRecurrenceChange] newRule:", newRule, "newTime:", newTime);
    onUpdate({ recurrence_rule: newRule, recurrence_time_of_day: newTime });
  };

  const handleTimeChange = (time: string) => {
    onUpdate({ recurrence_time_of_day: time });
  };


  const isDone = task.status === "done";
  const agentWorkBadge = getAgentWorkStatusBadge(task.agent_work_status);
  const agentFailureDetail = getAgentFailureDetail(task);
  const showExecutingIndicator = shouldShowAgentExecutingIndicator(task);
  const startDate = toDateInputValue(task.scheduled_start_at);
  const startTime = toTimeInputValue(task.scheduled_start_at);
  const endDate = toDateInputValue(task.scheduled_end_at);
  const endTime = toTimeInputValue(task.scheduled_end_at);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-2 pb-3 border-b border-glass-border">
        <button
          onClick={onBack}
          className="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-space-deep transition-colors"
          aria-label="Go back"
        >
          <ArrowLeft size={18} />
        </button>
        <span className="text-xs text-text-muted flex-1">Task Details</span>
        <button
          onClick={() => setShowDeleteDialog(true)}
          disabled={isDeleting}
          className="p-1.5 rounded-lg text-text-muted hover:text-color-danger hover:bg-color-danger/10 transition-colors disabled:opacity-50"
          aria-label="Delete task"
        >
          {isDeleting ? <LoadingSpinner size="sm" /> : <Trash2 size={16} />}
        </button>
      </div>

      <ScrollArea className="flex-1 -mx-1 px-1 mt-3">
        <div className="space-y-4 pr-2">
          {/* Status + Title */}
          <div className="flex items-center gap-2">
            {isUpdating ? (
              <LoadingSpinner size="sm" />
            ) : (
              <Checkbox
                checked={isDone}
                onCheckedChange={onToggle}
                className="w-5 h-5 shrink-0"
              />
            )}

            {editingTitle ? (
              <input
                value={titleDraft}
                onChange={(e) => setTitleDraft(e.target.value)}
                onBlur={handleTitleBlur}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleTitleBlur();
                  if (e.key === "Escape") {
                    setTitleDraft(task.title);
                    setEditingTitle(false);
                  }
                }}
                autoFocus
                className="flex-1 text-sm font-medium bg-transparent border-b border-[var(--glow-green)] outline-none text-text-primary pb-0.5"
              />
            ) : (
              <span
                onClick={() => {
                  setTitleDraft(task.title);
                  setEditingTitle(true);
                }}
                className={`flex-1 text-sm font-medium cursor-text ${
                  isDone ? "line-through text-text-muted" : "text-text-primary"
                }`}
              >
                {task.title}
              </span>
            )}

            {showExecutingIndicator && (
              <span
                className="inline-flex items-center gap-1 rounded-full border border-blue-400/30 bg-blue-500/10 px-1.5 py-0.5 text-[9px] font-semibold text-blue-300"
                title="Agent is currently working on this task"
              >
                <span className="h-1.5 w-1.5 rounded-full bg-blue-400 animate-pulse" />
                Live
              </span>
            )}
          </div>

          {/* Status selector */}
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-text-muted w-16">Status</span>
            <Select
              value={task.status}
              onValueChange={(value) => onUpdate({ status: value as Task["status"] })}
              disabled={isUpdating}
            >
              <SelectTrigger className="w-32 h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="bg-space-deep border-glass-border">
                {TASK_STATUS_OPTIONS.map((option) => {
                  const config = STATUS_CONFIG[option.value];
                  return (
                    <SelectItem key={option.value} value={option.value}>
                      <div className="flex items-center gap-2">
                        <span
                          className="w-2 h-2 rounded-full"
                          style={{ backgroundColor: config.color }}
                        />
                        {option.label}
                      </div>
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
          </div>

          {/* Priority selector */}
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-text-muted w-16">Priority</span>
            <Select
              value={task.priority}
              onValueChange={(v) => onUpdate({ priority: v as Task["priority"] })}
              disabled={isUpdating}
            >
              <SelectTrigger className="w-32 h-8">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="bg-space-deep border-glass-border">
                {Object.entries(PRIORITY_CONFIG).map(([value, config]) => (
                  <SelectItem key={value} value={value}>
                    <div className="flex items-center gap-2">
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: config.color }}
                      />
                      {config.label}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Assignee */}
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-text-muted w-16">Assignee</span>
            <Select
              value={task.assignee}
              onValueChange={(value) => onUpdate({ assignee: value })}
              disabled={isUpdating}
            >
              <SelectTrigger className="h-7 w-32 bg-space-deep border-glass-border text-[10px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {KNOWN_AGENTS.map((agent) => (
                  <SelectItem key={agent.id} value={agent.id}>
                    <div className="flex items-center gap-1.5">
                      {agent.id === "user" ? <User size={12} /> : <Bot size={12} />}
                      {agent.name}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {agentWorkBadge && (
            <div className="flex items-center gap-2">
              <span className="text-[10px] text-text-muted w-16">Agent</span>
              <div className="flex items-center gap-2 flex-wrap">
                <span
                  className="inline-flex items-center gap-1.5 rounded-full px-2 py-1 text-[10px] font-semibold"
                  style={{
                    backgroundColor: `${agentWorkBadge.color}20`,
                    color: agentWorkBadge.color,
                    border: `1px solid ${agentWorkBadge.color}40`,
                  }}
                >
                  <span
                    className={`h-1.5 w-1.5 rounded-full ${agentWorkBadge.animated ? "animate-pulse" : ""}`}
                    style={{ backgroundColor: agentWorkBadge.color }}
                  />
                  {agentWorkBadge.label}
                </span>
                {agentFailureDetail && (
                  <span className="text-[10px] text-text-muted">
                    {agentFailureDetail}
                  </span>
                )}
              </div>
            </div>
          )}

          {/* Labels */}
          <div className="flex items-start gap-2">
            <span className="text-[10px] text-text-muted w-16 mt-1">Labels</span>
            <div className="flex flex-wrap gap-1">
              {PREDEFINED_LABELS.map((label) => {
                const active = task.labels.includes(label.name);
                return (
                  <button
                    key={label.name}
                    onClick={() => {
                      const newLabels = active
                        ? task.labels.filter((l) => l !== label.name)
                        : [...task.labels, label.name];
                      onUpdate({ labels: newLabels });
                    }}
                    disabled={isUpdating}
                    className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium transition-all disabled:opacity-50 ${
                      active ? "opacity-100" : "opacity-40 hover:opacity-70"
                    }`}
                    style={{
                      backgroundColor: `${label.color}20`,
                      color: label.color,
                      border: `1px solid ${active ? label.color + "60" : "transparent"}`,
                    }}
                  >
                    {label.name}
                  </button>
                );
              })}
              <TaskLabelPills
                labels={task.labels.filter(
                  (l) => !PREDEFINED_LABELS.some((p) => p.name === l)
                )}
              />
            </div>
          </div>

          {/* Schedule + Repeat (combined collapsible) */}
          <div>
            <button
              onClick={() => setScheduleOpen((v) => !v)}
              className="flex items-center gap-2 w-full text-left group"
            >
              <Calendar size={14} className="text-text-muted shrink-0" />
              <span className="text-[10px] text-text-muted font-medium flex-1">
                {scheduleOpen
                  ? "Schedule"
                  : recurring
                  ? formatRecurrenceDisplay(
                      task.recurrence_rule!,
                      task.recurrence_time_of_day
                    )
                  : task.scheduled_start_at
                  ? "Scheduled"
                  : "Schedule"}
              </span>
              {recurring && !scheduleOpen && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onUpdate({ recurrence_rule: null, recurrence_time_of_day: null });
                  }}
                  className="text-[10px] text-text-muted hover:text-color-danger transition-colors"
                >
                  Clear
                </button>
              )}
            </button>

            {scheduleOpen && (
              <div className="space-y-3 mt-3 pl-6">
                {/* Recurrence row */}
                <div className="space-y-1">
                  <span className="text-[10px] text-text-muted">Repeat</span>
                  <Select
                    value={task.recurrence_rule ?? "__none__"}
                    onValueChange={handleRecurrenceChange}
                    disabled={isUpdating}
                  >
                    <SelectTrigger className="w-full h-9">
                      <SelectValue placeholder="Does not repeat" />
                    </SelectTrigger>
                    <SelectContent className="bg-space-deep border-glass-border">
                      {RECURRENCE_OPTIONS.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* Recurring mode: Time picker */}
                {recurring && (
                  <div className="space-y-1">
                    <span className="text-[10px] text-text-muted">Time</span>
                    <Select
                      value={task.recurrence_time_of_day || "09:00"}
                      onValueChange={handleTimeChange}
                      disabled={isUpdating}
                    >
                      <SelectTrigger className="w-full h-9">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent className="bg-space-deep border-glass-border">
                        {TIME_OPTIONS.map((t) => (
                          <SelectItem key={t} value={t}>
                            {t}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                )}

                {/* Non-recurring mode: Date/Time inputs */}
                {!recurring && (
                  <>
                    <div className="space-y-1">
                      <span className="text-[10px] text-text-muted">Start</span>
                      <div className="flex gap-2">
                        <input
                          type="date"
                          value={startDate || undefined}
                          onChange={(e) =>
                            onUpdate({
                              scheduled_start_at: fromDateTimeLocal(
                                e.target.value,
                                startTime
                              ),
                            })
                          }
                          disabled={isUpdating}
                          className="flex-1 h-9 px-2 text-xs bg-space-deep border border-glass-border rounded-md text-text-primary outline-none focus:border-[var(--glow-green)] transition-colors disabled:opacity-50"
                        />
                        <input
                          type="time"
                          value={startTime || undefined}
                          onChange={(e) =>
                            onUpdate({
                              scheduled_start_at: fromDateTimeLocal(
                                startDate,
                                e.target.value
                              ),
                            })
                          }
                          disabled={isUpdating}
                          className="w-24 h-9 px-2 text-xs bg-space-deep border border-glass-border rounded-md text-text-primary outline-none focus:border-[var(--glow-green)] transition-colors disabled:opacity-50"
                        />
                      </div>
                    </div>

                    <div className="space-y-1">
                      <span className="text-[10px] text-text-muted">End</span>
                      <div className="flex gap-2">
                        <input
                          type="date"
                          value={endDate || undefined}
                          onChange={(e) =>
                            onUpdate({
                              scheduled_end_at: fromDateTimeLocal(
                                e.target.value,
                                endTime
                              ),
                            })
                          }
                          disabled={isUpdating}
                          className="flex-1 h-9 px-2 text-xs bg-space-deep border border-glass-border rounded-md text-text-primary outline-none focus:border-[var(--glow-green)] transition-colors disabled:opacity-50"
                        />
                        <input
                          type="time"
                          value={endTime || undefined}
                          onChange={(e) =>
                            onUpdate({
                              scheduled_end_at: fromDateTimeLocal(
                                endDate,
                                e.target.value
                              ),
                            })
                          }
                          disabled={isUpdating}
                          className="w-24 h-9 px-2 text-xs bg-space-deep border border-glass-border rounded-md text-text-primary outline-none focus:border-[var(--glow-green)] transition-colors disabled:opacity-50"
                        />
                      </div>
                    </div>

                    <div className="space-y-1">
                      <span className="text-[10px] text-text-muted">Duration (optional)</span>
                      <div className="flex gap-2">
                        <input
                          type="number"
                          min="0"
                          step="5"
                          value={task.estimated_duration_min ?? ""}
                          onChange={(e) =>
                            onUpdate({
                              estimated_duration_min: e.target.value
                                ? parseInt(e.target.value, 10)
                                : null,
                            })
                          }
                          disabled={isUpdating}
                          placeholder="e.g. 30"
                          className="w-24 h-9 px-3 text-xs bg-space-deep border border-glass-border rounded-md text-text-primary outline-none focus:border-[var(--glow-green)] transition-colors placeholder:text-text-muted disabled:opacity-50"
                        />
                        <span className="text-[10px] text-text-muted self-center">minutes</span>
                      </div>
                    </div>
                  </>
                )}
              </div>
            )}
          </div>

          {/* Description */}
          <div className="space-y-1">
            <span className="text-[10px] text-text-muted">Description</span>
            <textarea
              value={descDraft}
              onChange={(e) => setDescDraft(e.target.value)}
              onBlur={handleDescBlur}
              disabled={isUpdating}
              placeholder="Add notes, details, or a sub-task list..."
              rows={6}
              className="w-full text-xs bg-space-deep border border-glass-border rounded-md px-3 py-2 text-text-primary outline-none focus:border-[var(--glow-green)] transition-colors resize-y placeholder:text-text-muted min-h-[80px] disabled:opacity-50"
            />
          </div>

          {/* Activity Section */}
          <TaskActivitySection taskId={task.id} />
        </div>
      </ScrollArea>

      <DeleteConfirmDialog
        isOpen={showDeleteDialog}
        taskTitle={task.title}
        onConfirm={() => {
          setShowDeleteDialog(false);
          onDelete();
        }}
        onCancel={() => setShowDeleteDialog(false)}
        isDeleting={isDeleting}
      />
    </div>
  );
}
