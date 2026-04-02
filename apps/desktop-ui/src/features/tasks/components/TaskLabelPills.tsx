import { PREDEFINED_LABELS } from "@/types/task";
import { getLabelColor } from "../utils/task-formatting";

interface TaskLabelPillsProps {
  labels: string[];
  maxVisible?: number;
}

function toSoftBackground(color: string): string {
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    const normalized = hex.length === 3
      ? hex.split("").map((char) => `${char}${char}`).join("")
      : hex;
    if (normalized.length === 6) {
      const r = Number.parseInt(normalized.slice(0, 2), 16);
      const g = Number.parseInt(normalized.slice(2, 4), 16);
      const b = Number.parseInt(normalized.slice(4, 6), 16);
      if (Number.isFinite(r) && Number.isFinite(g) && Number.isFinite(b)) {
        return `rgba(${r}, ${g}, ${b}, 0.18)`;
      }
    }
  }
  return "rgba(120, 130, 120, 0.18)";
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
              backgroundColor: toSoftBackground(color),
              color: "#33413A",
              border: `1px solid ${toSoftBackground(color)}`,
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
