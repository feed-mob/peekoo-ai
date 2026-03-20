import { useState, useRef, useEffect, FormEvent } from "react";
import { Plus, ChevronDown, User, Bot } from "lucide-react";
import { Input } from "@/components/ui/input";
import type { Task } from "@/types/task";
import { PREDEFINED_LABELS } from "@/types/task";

interface TaskInputProps {
  onAdd: (title: string, priority: Task["priority"], assignee: Task["assignee"], labels: string[]) => void;
}

const PRIORITY_OPTIONS: { value: Task["priority"]; label: string; color: string }[] = [
  { value: "high",   label: "High",   color: "#E9762B" },
  { value: "medium", label: "Medium", color: "#F5C842" },
  { value: "low",    label: "Low",    color: "#7B9AC7" },
];

export function TaskInput({ onAdd }: TaskInputProps) {
  const [newTask, setNewTask] = useState("");
  const [priority, setPriority] = useState<Task["priority"]>("medium");
  const [assignee, setAssignee] = useState<Task["assignee"]>("user");
  const [selectedLabels, setSelectedLabels] = useState<string[]>([]);
  const [priorityOpen, setPriorityOpen] = useState(false);
  const [labelOpen, setLabelOpen] = useState(false);
  const priorityRef = useRef<HTMLDivElement>(null);
  const labelRef = useRef<HTMLDivElement>(null);

  const selected = PRIORITY_OPTIONS.find((o) => o.value === priority)!;

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (priorityRef.current && !priorityRef.current.contains(e.target as Node)) {
        setPriorityOpen(false);
      }
      if (labelRef.current && !labelRef.current.contains(e.target as Node)) {
        setLabelOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const toggleLabel = (name: string) => {
    setSelectedLabels((prev) =>
      prev.includes(name) ? prev.filter((l) => l !== name) : [...prev, name],
    );
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!newTask.trim()) return;
    onAdd(newTask, priority, assignee, selectedLabels);
    setNewTask("");
    setPriority("medium");
    setAssignee("user");
    setSelectedLabels([]);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-2">
      <div className="flex gap-2">
        {/* Task title input */}
        <div className="relative flex-1">
          <Plus className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" size={16} />
          <Input
            type="text"
            value={newTask}
            onChange={(e) => setNewTask(e.target.value)}
            placeholder="Add a new task..."
            className="pl-10 h-10 bg-space-deep border-glass-border text-text-primary placeholder:text-text-muted"
            aria-label="Task title"
          />
        </div>

        {/* Priority selector */}
        <div ref={priorityRef} className="relative">
          <button
            type="button"
            onClick={() => setPriorityOpen((v) => !v)}
            className="flex items-center gap-1.5 h-10 w-24 px-3 bg-space-deep text-xs font-medium text-text-primary focus:outline-none transition-all duration-200 cursor-pointer rounded-full border-0 relative overflow-hidden"
            style={{
              boxShadow: `0 0 0 1.5px ${selected.color}55, 0 2px 8px ${selected.color}22`,
            }}
            aria-label="Task priority"
          >
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: selected.color }}
            />
            <span className="flex-1 text-left" style={{ color: selected.color }}>
              {selected.label}
            </span>
            <ChevronDown size={12} className={`text-text-muted shrink-0 transition-transform ${priorityOpen ? "rotate-180" : ""}`} />
          </button>

          {priorityOpen && (
            <div className="absolute top-full left-0 mt-1 w-28 bg-space-deep rounded-xl border border-glass-border shadow-lg z-50 overflow-hidden py-1">
              {PRIORITY_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => { setPriority(opt.value); setPriorityOpen(false); }}
                  className={`flex items-center gap-2 w-full px-3 py-2 text-xs font-medium transition-colors hover:bg-space-surface ${
                    priority === opt.value ? "bg-space-surface/60" : ""
                  }`}
                >
                  <span className="w-1 h-3 shrink-0 rounded-sm" style={{ backgroundColor: opt.color }} />
                  <span style={{ color: opt.color }}>{opt.label}</span>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Submit */}
        <button
          type="submit"
          className="h-10 w-10 rounded-full shrink-0 flex items-center justify-center text-white bg-[var(--glow-green)] shadow-md hover:brightness-110 active:scale-95 transition-all duration-150"
          aria-label="Add task"
        >
          <Plus size={18} strokeWidth={2.5} />
        </button>
      </div>

      {/* Assignee + Labels row */}
      <div className="flex items-center gap-2">
        {/* Assignee toggle */}
        <button
          type="button"
          onClick={() => setAssignee(assignee === "user" ? "agent" : "user")}
          className={`flex items-center gap-1.5 h-7 px-2.5 rounded-full text-xs font-medium transition-all ${
            assignee === "agent"
              ? "bg-purple-500/20 text-purple-400 border border-purple-500/40"
              : "bg-space-deep text-text-muted border border-glass-border"
          }`}
          title={`Assigned to ${assignee}`}
        >
          {assignee === "agent" ? <Bot size={12} /> : <User size={12} />}
          {assignee === "agent" ? "Agent" : "Me"}
        </button>

        {/* Label picker */}
        <div ref={labelRef} className="relative">
          <button
            type="button"
            onClick={() => setLabelOpen((v) => !v)}
            className="flex items-center gap-1 h-7 px-2.5 rounded-full text-xs font-medium bg-space-deep text-text-muted border border-glass-border transition-colors hover:text-text-primary"
          >
            Labels{selectedLabels.length > 0 && ` (${selectedLabels.length})`}
            <ChevronDown size={10} className={`transition-transform ${labelOpen ? "rotate-180" : ""}`} />
          </button>

          {labelOpen && (
            <div className="absolute top-full left-0 mt-1 w-36 bg-space-deep rounded-xl border border-glass-border shadow-lg z-50 overflow-hidden py-1">
              {PREDEFINED_LABELS.map((label) => (
                <button
                  key={label.name}
                  type="button"
                  onClick={() => toggleLabel(label.name)}
                  className={`flex items-center gap-2 w-full px-3 py-2 text-xs font-medium transition-colors hover:bg-space-surface ${
                    selectedLabels.includes(label.name) ? "bg-space-surface/60" : ""
                  }`}
                >
                  <span
                    className="w-2 h-2 rounded-full shrink-0"
                    style={{ backgroundColor: label.color }}
                  />
                  <span style={{ color: label.color }}>{label.name}</span>
                  {selectedLabels.includes(label.name) && (
                    <span className="ml-auto text-text-muted">✓</span>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Selected labels preview */}
        {selectedLabels.length > 0 && (
          <div className="flex gap-1 overflow-hidden">
            {selectedLabels.map((name) => {
              const label = PREDEFINED_LABELS.find((l) => l.name === name);
              const color = label?.color ?? "#888";
              return (
                <span
                  key={name}
                  className="inline-flex items-center px-1.5 py-0.5 rounded-full text-[10px] font-medium cursor-pointer hover:opacity-70"
                  style={{ backgroundColor: `${color}20`, color }}
                  onClick={() => toggleLabel(name)}
                >
                  {name} ×
                </span>
              );
            })}
          </div>
        )}
      </div>
    </form>
  );
}
