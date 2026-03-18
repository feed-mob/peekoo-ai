import { useGlobalSettings } from "./useGlobalSettings";
import { SpriteSelector } from "./SpriteSelector";

export function SettingsPanel() {
  const { activeSpriteId, sprites, loading, error, setActiveSpriteId } = useGlobalSettings();

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

  return (
    <div className="space-y-6">
      <SpriteSelector
        sprites={sprites}
        activeSpriteId={activeSpriteId}
        onSelect={setActiveSpriteId}
      />

      {/* Future sections (language, theme, etc.) can be added here */}
    </div>
  );
}
