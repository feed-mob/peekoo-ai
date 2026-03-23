import { PREDEFINED_LABELS } from "@/types/task";

interface TaskLabelsProps {
  labels: string[];
}

function getLabelColor(label: string): string {
  const predefined = PREDEFINED_LABELS.find((l) => l.name === label);
  if (predefined) return predefined.color;
  // Hash-based color for custom labels
  let hash = 0;
  for (let i = 0; i < label.length; i++) {
    hash = label.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 60%, 55%)`;
}

export function TaskLabels({ labels }: TaskLabelsProps) {
  if (labels.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-1 mt-1">
      {labels.map((label) => {
        const color = getLabelColor(label);
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
    </div>
  );
}
