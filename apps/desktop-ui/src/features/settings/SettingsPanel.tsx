import { useGlobalSettings } from "./useGlobalSettings";
import { useLinearIntegrationStatus } from "./useLinearIntegrationStatus";
import { SpriteSelector } from "./SpriteSelector";
import { Button } from "@/components/ui/button";
import { Sun, Moon, Monitor } from "lucide-react";

export function SettingsPanel() {
  const { 
    activeSpriteId, 
    themeMode,
    sprites, 
    loading, 
    error, 
    setActiveSpriteId,
    setThemeMode
  } = useGlobalSettings();
  const {
    status: linearStatus,
    isLoading: linearLoading,
    error: linearError,
    refresh: refreshLinearStatus,
  } = useLinearIntegrationStatus();

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

  const linearStatusLabels: Record<string, string> = {
    uninstalled: "Not Installed",
    disabled: "Installed (Disabled)",
    disconnected: "Disconnected",
    connecting: "Connecting",
    connected: "Connected",
    syncing: "Syncing",
    error: "Error",
    unknown: "Unknown",
  };

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
        <h3 className="text-sm font-medium text-text-secondary uppercase tracking-wider">Integrations</h3>
        <div className="rounded-2xl border border-glass-border bg-glass/30 px-4 py-3 space-y-2">
          <div className="flex items-center justify-between gap-3">
            <p className="text-sm font-medium text-text-primary">Linear</p>
            <Button size="sm" variant="ghost" onClick={() => void refreshLinearStatus()}>
              Refresh
            </Button>
          </div>
          <p className="text-xs text-text-muted">
            Status: {linearLoading ? "Loading..." : linearStatusLabels[linearStatus.uiStatus] ?? "Unknown"}
          </p>
          {linearStatus.workspaceName ? (
            <p className="text-xs text-text-muted">Workspace: {linearStatus.workspaceName}</p>
          ) : null}
          {linearStatus.userEmail ? (
            <p className="text-xs text-text-muted">
              Account: {linearStatus.userName ? `${linearStatus.userName} · ` : ""}
              {linearStatus.userEmail}
            </p>
          ) : null}
          {linearStatus.lastSyncAt ? (
            <p className="text-xs text-text-muted">Last Sync: {linearStatus.lastSyncAt}</p>
          ) : null}
          {linearStatus.lastError ? (
            <p className="text-xs text-danger">Error: {linearStatus.lastError}</p>
          ) : null}
          {linearError ? (
            <p className="text-xs text-danger">Failed to fetch integration status: {linearError}</p>
          ) : null}
        </div>
      </section>
    </div>
  );
}
