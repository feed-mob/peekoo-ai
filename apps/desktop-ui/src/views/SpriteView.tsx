import { useState, useCallback } from "react";
import { Sprite } from "@/components/sprite/Sprite";
import { SpriteActionMenu } from "@/components/sprite/SpriteActionMenu";
import { useSpriteState } from "@/hooks/use-sprite-state";
import { usePanelWindows } from "@/hooks/use-panel-windows";
import { useSpriteReactions } from "@/hooks/use-sprite-reactions";
import type { SpriteState } from "@/types/sprite";

// Duration (ms) a reaction-triggered mood override stays active before reverting
const MOOD_OVERRIDE_DURATION_MS = 3000;

export default function SpriteView() {
  const spriteState = useSpriteState();
  const { panels, togglePanel, expandForMenu, shrinkToSprite } =
    usePanelWindows();
  const [menuOpen, setMenuOpen] = useState(false);
  const [randomTrigger, setRandomTrigger] = useState(0);
  const [moodOverride, setMoodOverride] = useState<string | null>(null);

  const handleMoodChange = useCallback((mood: string) => {
    setMoodOverride(mood);
    const timer = setTimeout(() => {
      setMoodOverride(null);
    }, MOOD_OVERRIDE_DURATION_MS);
    return () => clearTimeout(timer);
  }, []);

  useSpriteReactions({ onMoodChange: handleMoodChange });

  const effectiveSpriteState: SpriteState = moodOverride
    ? { ...spriteState, mood: moodOverride }
    : spriteState;

  // Left click: trigger random animation
  const handleSpriteClick = useCallback(() => {
    setRandomTrigger((prev) => prev + 1);
  }, []);

  // Right click: toggle menu
  const handleContextMenu = useCallback(
    async (e: React.MouseEvent) => {
      e.preventDefault();
      if (menuOpen) {
        setMenuOpen(false);
        await shrinkToSprite();
      } else {
        await expandForMenu();
        setMenuOpen(true);
      }
    },
    [menuOpen, expandForMenu, shrinkToSprite],
  );

  const handleTogglePanel = useCallback(
    async (label: Parameters<typeof togglePanel>[0]) => {
      await togglePanel(label);
      setMenuOpen(false);
      await shrinkToSprite();
    },
    [togglePanel, shrinkToSprite],
  );

  return (
    <div
      className="w-full h-full bg-transparent"
      data-tauri-drag-region
    >
      <div className="relative flex items-center justify-center w-full h-full">
        <div
          onClick={handleSpriteClick}
          onContextMenu={handleContextMenu}
          className="cursor-pointer"
        >
          <Sprite state={effectiveSpriteState} randomTrigger={randomTrigger} />
        </div>

        <SpriteActionMenu
          panels={panels}
          onTogglePanel={handleTogglePanel}
          isOpen={menuOpen}
        />
      </div>
    </div>
  );
}
