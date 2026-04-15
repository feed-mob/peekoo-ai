import type { SpriteManifest } from "@/types/sprite";

export function getActiveSpriteManifest(
  manifests: Record<string, SpriteManifest>,
  activeSpriteId: string,
): SpriteManifest | null {
  return manifests[activeSpriteId] ?? null;
}
