import { useMemo } from "react";
import { Button } from "@/components/ui/button";
import { SkillToggleList } from "./SkillToggleList";
import { useChatSettings } from "./useChatSettings";
import { useAgentProviders } from "@/hooks/useAgentProviders";

interface ChatSettingsPanelProps {
  onClose: () => void;
  activeRuntimeName?: string | null;
  configuredModelId?: string | null;
}

export function ChatSettingsPanel({
  onClose,
  activeRuntimeName,
  configuredModelId,
}: ChatSettingsPanelProps) {
  const {
    settings,
    catalog,
    isLoading,
    error,
    refresh,
    updateSettings,
  } = useChatSettings();

  const { defaultProvider } = useAgentProviders();

  const effectiveSkills = useMemo(() => {
    if (!settings || !catalog) return [];
    return settings.skills.length > 0 ? settings.skills : catalog.discoveredSkills;
  }, [catalog, settings]);

  if (isLoading && !settings) {
    return <div className="text-sm text-text-muted">Loading settings...</div>;
  }

  if (error) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">Failed to load settings.</p>
        <p className="text-xs text-text-muted">{error}</p>
        <Button size="sm" onClick={() => void refresh()}>
          Retry
        </Button>
      </div>
    );
  }

  if (!settings || !catalog) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-danger">Failed to load settings.</p>
        <Button size="sm" onClick={() => void refresh()}>
          Retry
        </Button>
      </div>
    );
  }

  if (catalog.providers.length === 0) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">No ACP runtimes installed.</p>
        <p className="text-xs text-text-secondary">
          Install a runtime from the Settings panel to get started.
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          Refresh
        </Button>
      </div>
    );
  }

  if (!defaultProvider) {
    return (
      <div className="space-y-2">
        <p className="text-sm text-text-muted">Selected runtime not found.</p>
        <p className="text-xs text-text-secondary">
          The previously selected runtime is no longer available.
        </p>
        <Button size="sm" onClick={() => void refresh()}>
          Refresh
        </Button>
      </div>
    );
  }

  return (
    <div className="max-h-[56vh] space-y-4 overflow-y-auto rounded-lg border border-glass-border bg-glass/50 p-3 pr-2">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-text-primary">Chat Settings</h3>
        <Button size="sm" variant="ghost" onClick={onClose}>
          Close
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-3">
        <div className="rounded-md border border-glass-border bg-space-surface/40 px-3 py-2 text-xs text-text-muted">
          <div>Active runtime: {activeRuntimeName ?? defaultProvider.displayName}</div>
        </div>

        <div className="rounded-md border border-glass-border bg-space-deep px-3 py-2">
          <div className="text-sm text-text-secondary">Model</div>
          <div className="mt-1 text-sm text-text-primary">
            {configuredModelId ?? "No global model configured"}
          </div>
          <div className="mt-1 text-xs text-text-muted">
            Change this in global runtime settings.
          </div>
        </div>
      </div>

      <div className="space-y-2">
        <p className="text-sm font-medium text-text-primary">Skills</p>
        <SkillToggleList
          skills={effectiveSkills}
          onToggle={(skillId, enabled) => {
            const skills = effectiveSkills.map((skill) =>
              skill.skillId === skillId ? { ...skill, enabled } : skill
            );
            void updateSettings({ skills });
          }}
        />
      </div>
    </div>
  );
}
