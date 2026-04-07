import { Trash2, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { SkillSettings } from "@/types/agent-settings";

interface SkillListProps {
  skills: SkillSettings[];
  onUpload: () => void;
  onDelete: (skillMdPath: string) => void;
  isLoading: boolean;
  error: string | null;
}

export function SkillList({ skills, onUpload, onDelete, isLoading, error }: SkillListProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium text-text-primary">{t("chatSettings.skills")}</p>
        <Button
          size="sm"
          variant="ghost"
          onClick={onUpload}
          disabled={isLoading}
          className="h-7 gap-1.5 px-2 text-xs"
        >
          <Upload className="h-3.5 w-3.5" />
          {t("chatSettings.uploadSkill")}
        </Button>
      </div>

      <p className="text-xs text-text-muted">{t("chatSettings.skillsHelp")}</p>

      {error && (
        <p className="text-xs text-danger">{error}</p>
      )}

      {skills.length === 0 ? (
        <p className="text-xs text-text-muted">{t("chatSettings.noSkills")}</p>
      ) : (
        <div className="space-y-1">
          {skills.map((skill) => (
            <div
              key={`${skill.skillId}:${skill.path}`}
              className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 hover:bg-glass/30"
            >
              <div className="min-w-0 flex-1 space-y-0.5">
                <div className="truncate text-sm text-text-secondary">{skill.skillId}</div>
                <div className="truncate text-xs text-text-muted" title={skill.path}>
                  {skill.path}
                </div>
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => onDelete(skill.path)}
                disabled={isLoading || skill.locked}
                className="h-6 w-6 shrink-0 p-0 text-text-muted hover:text-danger"
                title={
                  skill.locked
                    ? t("chatSettings.lockedSkillCannotDelete")
                    : t("chatSettings.deleteSkill")
                }
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
