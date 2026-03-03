import { Checkbox } from "@/components/ui/checkbox";
import type { SkillSettings } from "@/types/agent-settings";

interface SkillToggleListProps {
  skills: SkillSettings[];
  onToggle: (skillId: string, enabled: boolean) => void;
}

export function SkillToggleList({ skills, onToggle }: SkillToggleListProps) {
  if (skills.length === 0) {
    return <p className="text-xs text-text-muted">No skills discovered yet.</p>;
  }

  return (
    <div className="space-y-2">
      {skills.map((skill) => (
        <label key={skill.skillId} className="flex items-center gap-2 text-sm text-text-secondary">
          <Checkbox
            checked={skill.enabled}
            onCheckedChange={(checked) => onToggle(skill.skillId, checked === true)}
          />
          <span className="truncate" title={skill.path}>
            {skill.skillId}
          </span>
        </label>
      ))}
    </div>
  );
}
