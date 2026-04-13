import { invoke } from "@tauri-apps/api/core";
import type { SpriteInfo } from "@/types/global-settings";
import type { CustomSpriteManifestFile, SpriteManifest } from "@/types/sprite";

export interface LoadedSpriteAsset {
  manifest: SpriteManifest;
  imageSrc: string;
}

export async function loadLocalSpriteImage(imagePath: string): Promise<string> {
  return invoke<string>("app_sprites_get_image_data_url", { imagePath });
}

export async function loadSpriteAsset(sprite: SpriteInfo): Promise<LoadedSpriteAsset> {
  if (sprite.source === "builtin") {
    const response = await fetch(`/sprites/${sprite.id}/manifest.json`);
    const manifest = (await response.json()) as SpriteManifest;
    return {
      manifest,
      imageSrc: `/sprites/${sprite.id}/${manifest.image}`,
    };
  }

  const custom = await invoke<CustomSpriteManifestFile>("app_sprites_get_custom_manifest", {
    spriteId: sprite.id,
  });
  return {
    manifest: custom.manifest,
    imageSrc: await loadLocalSpriteImage(custom.imagePath),
  };
}
