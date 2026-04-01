import type { SkillSettings } from "@/types/agent-settings";

interface SkillListProps {
  skills: SkillSettings[];
}

export function SkillList({ skills }: SkillListProps) {
  if (skills.length === 0) {
    return <p className="text-xs text-text-muted">No skills discovered yet.</p>;
  }

  return (
    <div className="space-y-2">
      {skills.map((skill) => (
        <div key={`${skill.skillId}:${skill.path}`} className="space-y-0.5">
          <div className="truncate text-sm text-text-secondary">{skill.skillId}</div>
          <div className="truncate text-xs text-text-muted" title={skill.path}>
            {skill.path}
          </div>
        </div>
      ))}
    </div>
  );
}
