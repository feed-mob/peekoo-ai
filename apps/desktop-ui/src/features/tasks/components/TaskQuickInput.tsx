import { useState, useRef, FormEvent, KeyboardEvent } from "react";
import { Plus } from "lucide-react";
import { Input } from "@/components/ui/input";
import { LoadingSpinner } from "./LoadingSpinner";

interface TaskQuickInputProps {
  onAdd: (text: string) => Promise<unknown>;
  isCreating?: boolean;
}

export function TaskQuickInput({ onAdd, isCreating = false }: TaskQuickInputProps) {
  const [text, setText] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  // Keyboard shortcut: Cmd/Ctrl + Enter to submit
  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && text.trim()) {
      handleSubmit();
    }
  };

  const handleSubmit = async (e?: FormEvent) => {
    e?.preventDefault();
    if (!text.trim() || isCreating) return;

    await onAdd(text.trim());

    // Reset form
    setText("");
    inputRef.current?.focus();
  };

  return (
    <form onSubmit={handleSubmit} className="flex gap-2">
      {/* Task text input - AI will parse natural language */}
      <div className="relative flex-1">
        <Plus
          className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted"
          size={16}
        />
        <Input
          ref={inputRef}
          type="text"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Add a task... e.g., 'Meeting tomorrow at 3pm for 1 hour high priority'"
          disabled={isCreating}
          className="pl-10 h-10 bg-space-deep border-glass-border text-text-primary placeholder:text-text-muted disabled:opacity-50"
          aria-label="Task description"
        />
      </div>

      {/* Submit button */}
      <button
        type="submit"
        disabled={isCreating || !text.trim()}
        className="h-10 w-10 rounded-full shrink-0 flex items-center justify-center text-white bg-[var(--glow-green)] shadow-md hover:brightness-110 active:scale-95 transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed"
        aria-label="Add task"
      >
        {isCreating ? (
          <LoadingSpinner size="sm" className="text-white" />
        ) : (
          <Plus size={18} strokeWidth={2.5} />
        )}
      </button>
    </form>
  );
}
