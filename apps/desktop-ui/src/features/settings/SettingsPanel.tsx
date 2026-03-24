import { useGlobalSettings } from "./useGlobalSettings";
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
    </div>
  );
}

