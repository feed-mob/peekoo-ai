import { PREDEFINED_LABELS } from "@/types/task";
import { getLabelColor } from "../utils/task-formatting";

interface TaskLabelPillsProps {
  labels: string[];
  maxVisible?: number;
}

export function TaskLabelPills({ labels, maxVisible = 3 }: TaskLabelPillsProps) {
  if (labels.length === 0) return null;

  const visibleLabels = labels.slice(0, maxVisible);
  const remainingCount = labels.length - maxVisible;

  return (
    <div className="flex flex-wrap gap-1">
      {visibleLabels.map((label) => {
        const predefined = PREDEFINED_LABELS.find((l) => l.name === label);
        const color = predefined?.color ?? getLabelColor(label);

        return (
          <span
            key={label}
            className="inline-flex items-center px-1.5 py-0.5 rounded-full text-[10px] font-medium leading-none"
            style={{
              backgroundColor: `${color}20`,
              color,
              border: `1px solid ${color}40`,
            }}
          >
            {label}
          </span>
        );
      })}
      {remainingCount > 0 && (
        <span className="text-[10px] text-text-muted">+{remainingCount}</span>
      )}
    </div>
  );
}
