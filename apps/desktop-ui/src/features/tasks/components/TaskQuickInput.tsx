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
    <form onSubmit={handleSubmit} className="flex gap-2 mb-3">
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
          placeholder="Add a task... (Enter to submit)"
          disabled={isCreating}
          className="pl-10 h-11 text-sm bg-glass backdrop-blur-xl border-glass-border text-text-primary placeholder:text-text-muted disabled:opacity-50 focus:border-glow-green dark:focus:border-glow-olive transition-colors"
          aria-label="Task description"
        />
      </div>

      {/* Submit button - gradient variant */}
      <button
        type="submit"
        disabled={isCreating || !text.trim()}
        className="h-11 w-11 rounded-full shrink-0 flex items-center justify-center text-white bg-gradient-to-br from-glow-green to-glow-sage dark:from-glow-green dark:to-glow-olive shadow-md hover:shadow-lg hover:scale-105 active:scale-95 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
        aria-label="Add task"
      >
        {isCreating ? (
          <LoadingSpinner size="sm" className="text-white" />
        ) : (
          <Plus size={20} strokeWidth={2.5} />
        )}
      </button>
    </form>
  );
}
