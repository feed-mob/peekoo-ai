import { Button } from "@/components/ui/button";
import { SkillList } from "./SkillList";
import { SkillReplaceDialog } from "./SkillReplaceDialog";
import { useChatSettings } from "./useChatSettings";
import { useTranslation } from "react-i18next";
import { useAgentProviders } from "@/hooks/useAgentProviders";

interface ChatSettingsPanelProps {
  onClose: () => void;
}

export function ChatSettingsPanel({ onClose }: ChatSettingsPanelProps) {
  const { t } = useTranslation();
  const {
    settings,
    catalog,
    isLoading,
    error,
    refresh,
    isSkillLoading,
    skillError,
    pendingReplaceSkillId,
    uploadSkill,
    confirmReplaceSkill,
    cancelReplaceSkill,
    deleteSkill,
  } = useChatSettings();

  const { defaultProvider } = useAgentProviders();

  if (isLoading && !settings) {
    return <div className="text-sm text-text-muted">{t("chatSettings.loading")}</div>;
  }

  if (error) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">{t("chatSettings.failedLoad")}</p>
        <p className="text-xs text-text-muted">{error}</p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.retry")}
        </Button>
      </div>
    );
  }

  if (!settings || !catalog) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">{t("chatSettings.failedLoad")}</p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.retry")}
        </Button>
      </div>
    );
  }

  if (catalog.providers.length === 0) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">{t("chatSettings.noRuntimes")}</p>
        <p className="text-xs text-text-secondary">
          {t("chatSettings.installRuntimeHelp")}
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.refresh")}
        </Button>
      </div>
    );
  }

  if (!defaultProvider) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">{t("chatSettings.runtimeNotFound")}</p>
        <p className="text-xs text-text-secondary">
          {t("chatSettings.runtimeUnavailable")}
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          {t("common.refresh")}
        </Button>
      </div>
    );
  }

  return (
    <>
      <div className="max-h-[56vh] space-y-4 overflow-y-auto rounded-lg border border-glass-border bg-glass/50 p-3 pr-2">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-text-primary">{t("chatSettings.title")}</h3>
          <Button size="sm" variant="ghost" onClick={onClose}>
            {t("common.close")}
          </Button>
        </div>

        <div className="grid grid-cols-1 gap-3">
          <div className="rounded-md border border-glass-border bg-space-deep px-3 py-2">
            <div className="text-sm text-text-secondary">{t("chatSettings.modelLabel")}</div>
            <div className="mt-1 text-sm text-text-primary">
              {defaultProvider.config.defaultModel ?? t("chatSettings.noModelConfigured")}
            </div>
            <div className="mt-1 text-xs text-text-muted">
              {t("chatSettings.modelHelp")}
            </div>
          </div>
        </div>

        <SkillList
          skills={catalog.discoveredSkills}
          onUpload={() => void uploadSkill()}
          onDelete={(path) => void deleteSkill(path)}
          isLoading={isSkillLoading}
          error={skillError}
        />
      </div>

      <SkillReplaceDialog
        skillId={pendingReplaceSkillId}
        onConfirm={() => void confirmReplaceSkill()}
        onCancel={cancelReplaceSkill}
      />
    </>
  );
}
