import { useState, useEffect } from "react";
import { Check } from "lucide-react";
import SpriteAnimation from "@/components/sprite/SpriteAnimation";
import type { SpriteInfo } from "@/types/global-settings";
import type { SpriteManifest } from "@/types/sprite";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";

interface SpriteSelectorProps {
  sprites: SpriteInfo[];
  activeSpriteId: string | null;
  onSelect: (spriteId: string) => void;
}

function SpritePreview({ spriteId }: { spriteId: string }) {
  const { t } = useTranslation();
  const [manifest, setManifest] = useState<SpriteManifest | null>(null);

  useEffect(() => {
    fetch(`/sprites/${spriteId}/manifest.json`)
      .then((res) => res.json())
      .then((data: SpriteManifest) => setManifest(data))
      .catch((err) => console.error(`Failed to load manifest for ${spriteId}`, err));
  }, [spriteId]);

  if (!manifest) {
    return (
      <div className="w-full h-24 flex items-center justify-center text-text-muted text-xs">
        {t("settings.sprite.loadingPreview")}
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center h-24">
      <SpriteAnimation
        animation="idle"
        frameRate={manifest.frameRate || 8}
        scale={manifest.scale ?? 0.40}
        chromaKey={manifest.chromaKey}
        imageSrc={`/sprites/${spriteId}/${manifest.image}`}
        columns={manifest.layout.columns}
        rows={manifest.layout.rows}
        pixelArt={manifest.chromaKey.pixelArt}
      />
    </div>
  );
}

export function SpriteSelector({ sprites, activeSpriteId, onSelect }: SpriteSelectorProps) {
  const { t } = useTranslation();
  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold text-text-primary">{t("settings.sprite.title")}</h3>
      <div className="grid grid-cols-2 gap-3">
        {sprites.map((sprite) => {
          const isActive = sprite.id === activeSpriteId;
          return (
            <button
              key={sprite.id}
              onClick={() => onSelect(sprite.id)}
              className={cn(
                "relative flex flex-col items-center gap-2 p-3 rounded-xl border transition-all cursor-pointer",
                isActive
                  ? "border-glow-green bg-muted-green/50 shadow-glow-green"
                  : "border-glass-border bg-glass hover:bg-space-overlay hover:border-text-muted",
              )}
            >
              {isActive && (
                <div className="absolute top-2 right-2 w-5 h-5 rounded-full bg-glow-green flex items-center justify-center">
                  <Check size={12} className="text-white" />
                </div>
              )}
              <SpritePreview spriteId={sprite.id} />
              <div className="text-center">
                <p className="text-xs font-medium text-text-primary">{sprite.name}</p>
                <p className="text-[10px] text-text-muted leading-tight mt-0.5">{sprite.description}</p>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
