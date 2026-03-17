import { useState, useRef, useEffect, FormEvent } from "react";
import { Plus, ChevronDown } from "lucide-react";
import { Input } from "@/components/ui/input";
import type { Task } from "@/types/task";

interface TaskInputProps {
  onAdd: (title: string, priority: Task["priority"]) => void;
}

const PRIORITY_OPTIONS: { value: Task["priority"]; label: string; color: string }[] = [
  { value: "high",   label: "High",   color: "#E9762B" },
  { value: "medium", label: "Medium", color: "#F5C842" },
  { value: "low",    label: "Low",    color: "#7B9AC7" },
];

export function TaskInput({ onAdd }: TaskInputProps) {
  const [newTask, setNewTask] = useState("");
  const [priority, setPriority] = useState<Task["priority"]>("medium");
  const [open, setOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const selected = PRIORITY_OPTIONS.find((o) => o.value === priority)!;

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!newTask.trim()) return;
    onAdd(newTask, priority);
    setNewTask("");
    setPriority("medium");
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-3">
      {/* Task title input */}
      <div className="relative flex-1">
        <Plus className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" size={16} />
        <Input
          type="text"
          value={newTask}
          onChange={(e) => setNewTask(e.target.value)}
          placeholder="Add a new task..."
          className="pl-10 h-11 bg-space-deep border-glass-border text-text-primary placeholder:text-text-muted"
          aria-label="Task title"
        />
      </div>

      {/* Priority selector */}
      <div ref={dropdownRef} className="relative">
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          className="flex items-center gap-2 h-11 w-32 px-4 bg-space-deep text-sm font-medium text-text-primary focus:outline-none transition-all duration-200 cursor-pointer rounded-full border-0 relative overflow-hidden"
          style={{
            boxShadow: `0 0 0 1.5px ${selected.color}55, 0 2px 8px ${selected.color}22`,
          }}
          aria-label="Task priority"
          aria-expanded={open}
        >
          {/* Color dot */}
          <span
            className="w-2.5 h-2.5 rounded-full shrink-0"
            style={{ backgroundColor: selected.color }}
          />
          {/* Label colored */}
          <span className="flex-1 text-left" style={{ color: selected.color }}>
            {selected.label}
          </span>
          <ChevronDown
            size={14}
            className={`text-text-muted shrink-0 transition-transform duration-200 ${open ? "rotate-180" : ""}`}
          />
        </button>

        {/* Dropdown */}
        {open && (
          <div className="absolute top-full left-0 mt-1 w-32 bg-space-deep rounded-xl border border-glass-border shadow-lg z-50 overflow-hidden py-1">
            {PRIORITY_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => { setPriority(opt.value); setOpen(false); }}
                className={`flex items-center gap-2 w-full px-3 py-2.5 text-sm font-medium transition-colors hover:bg-space-surface ${
                  priority === opt.value ? "bg-space-surface/60" : ""
                }`}
              >
                {/* Left accent bar matching priority color */}
                <span
                  className="w-1 h-4 shrink-0 rounded-sm"
                  style={{ backgroundColor: opt.color }}
                />
                <span style={{ color: opt.color }}>{opt.label}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Submit */}
      <button
        type="submit"
        className="h-11 w-11 rounded-full shrink-0 flex items-center justify-center text-white bg-[var(--glow-green)] shadow-md hover:brightness-110 active:scale-95 transition-all duration-150"
        aria-label="Add task"
      >
        <Plus size={18} strokeWidth={2.5} />
      </button>
    </form>
  );
}
