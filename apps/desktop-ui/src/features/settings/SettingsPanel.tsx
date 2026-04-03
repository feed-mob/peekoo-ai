import { useGlobalSettings } from "./useGlobalSettings";
import { SpriteSelector } from "./SpriteSelector";
import { AgentProviderPanel } from "@/features/agent-runtimes/AgentProviderPanel";
import { Button } from "@/components/ui/button";
import { Sun, Moon, Monitor } from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";

export function SettingsPanel() {
  const {
    activeSpriteId,
    themeMode,
    logLevel,
    sprites,
    loading,
    error,
    setActiveSpriteId,
    setThemeMode,
    setLogLevel,
  } = useGlobalSettings();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-32 text-text-muted text-sm">
        Loading settings...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-32 text-danger text-sm">
        Failed to load settings: {error}
      </div>
    );
  }

  const themeOptions = [
    { id: "light", label: "Light", icon: Sun },
    { id: "dark", label: "Dark", icon: Moon },
    { id: "system", label: "System", icon: Monitor },
  ];

  const logLevelOptions = ["error", "warn", "info", "debug", "trace"] as const;

  async function handleLogLevelChange(nextLevel: string) {
    if (nextLevel === logLevel) {
      return;
    }

    await setLogLevel(nextLevel);
    const shouldRestart = await ask(
      "Log level updated. Restart Peekoo now to apply the new logging level?",
      {
        title: "Restart Required",
        kind: "info",
        okLabel: "Restart Now",
        cancelLabel: "Later",
      },
    );

    if (shouldRestart) {
      await relaunch();
    }
  }

  return (
    <div className="space-y-8">
      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">Appearance</h3>
        <div className="flex gap-2">
          {themeOptions.map((option) => {
            const Icon = option.icon;
            const isActive = themeMode === option.id;
            return (
              <Button
                key={option.id}
                variant={isActive ? "default" : "ghost"}
                size="sm"
                className={`flex-1 flex items-center gap-2 h-10 border ${
                  isActive 
                    ? "border-primary/50 shadow-lg shadow-primary/10" 
                    : "border-glass-border hover:bg-glass/30 text-text-muted"
                }`}
                onClick={() => void setThemeMode(option.id)}
              >
                <Icon size={16} />
                <span>{option.label}</span>
              </Button>
            );
          })}
        </div>
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">Active Pet</h3>
        <SpriteSelector
          sprites={sprites}
          activeSpriteId={activeSpriteId}
          onSelect={setActiveSpriteId}
        />
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">Logging</h3>
        <p className="text-xs text-text-muted">
          Set runtime log detail for troubleshooting. Restart required to apply changes.
        </p>
        <div className="flex flex-wrap gap-2">
          {logLevelOptions.map((option) => {
            const isActive = logLevel === option;
            return (
              <Button
                key={option}
                variant={isActive ? "default" : "ghost"}
                size="sm"
                className={`h-9 border ${
                  isActive
                    ? "border-primary/50 shadow-lg shadow-primary/10"
                    : "border-glass-border hover:bg-glass/30 text-text-muted"
                }`}
                onClick={() => void handleLogLevelChange(option)}
              >
                {option.toUpperCase()}
              </Button>
            );
          })}
        </div>
      </section>

      <section className="space-y-3">
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">
          ACP Runtimes
        </h3>
        <AgentProviderPanel />
      </section>
    </div>
  );
}
