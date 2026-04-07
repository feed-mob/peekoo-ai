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
        const backgroundColor = withAlpha(color, 0.18);
        const borderColor = withAlpha(color, 0.35);

        return (
          <span
            key={label}
            className="inline-flex items-center px-1.5 py-0.5 rounded-full text-[10px] font-medium leading-none"
            style={{
              backgroundColor,
              color: "var(--text-primary)",
              border: `1px solid ${borderColor}`,
            }}
          >
            <span
              className="mr-1 inline-block h-1.5 w-1.5 rounded-full"
              style={{ backgroundColor: color }}
            />
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

function withAlpha(color: string, alpha: number): string {
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    const normalized =
      hex.length === 3
        ? hex
            .split("")
            .map((part) => part + part)
            .join("")
        : hex;
    const r = Number.parseInt(normalized.slice(0, 2), 16);
    const g = Number.parseInt(normalized.slice(2, 4), 16);
    const b = Number.parseInt(normalized.slice(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  if (color.startsWith("hsl(") && color.endsWith(")")) {
    return color.replace(/^hsl\((.*)\)$/, `hsla($1, ${alpha})`);
  }

  if (color.startsWith("rgb(") && color.endsWith(")")) {
    return color.replace(/^rgb\((.*)\)$/, `rgba($1, ${alpha})`);
  }

  return color;
}
