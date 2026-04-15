import { useEffect, useMemo, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Copy, ImagePlus, FileJson, CheckCircle2, AlertTriangle, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import SpriteAnimation from "@/components/sprite/SpriteAnimation";
import { loadLocalSpriteImage } from "@/components/sprite/spriteAsset";
import { NotificationToast } from "@/features/tasks/components/ErrorToast";
import { useToast } from "@/features/tasks/hooks/use-toast";
import type { SpriteInfo } from "@/types/global-settings";
import type {
  GenerateSpriteManifestInput,
  SpriteManifest,
  SpriteManifestValidation,
  ValidationIssue,
} from "@/types/sprite";

interface CustomSpriteManagerProps {
  sprites: SpriteInfo[];
  deleteCustomSprite: (spriteId: string) => Promise<void>;
  getSpritePrompt: () => Promise<string>;
  getSpriteManifestTemplate: () => Promise<SpriteManifest>;
  loadSpriteManifestFile: (manifestPath: string) => Promise<SpriteManifest>;
  generateSpriteManifestDraft: (input: GenerateSpriteManifestInput) => Promise<{
    manifest: SpriteManifest;
    manifestValidation: SpriteManifestValidation;
  }>;
  generateSpriteManifestWithAgent: (input: GenerateSpriteManifestInput) => Promise<{
    manifest: SpriteManifest;
    manifestValidation: SpriteManifestValidation;
  }>;
  validateSpriteManifest: (input: {
    imagePath: string;
    manifest: SpriteManifest;
  }) => Promise<SpriteManifestValidation>;
  saveCustomSprite: (input: { imagePath: string; manifest: SpriteManifest }) => Promise<SpriteInfo>;
}

function renderIssues(issues: ValidationIssue[], tone: "error" | "warning") {
  if (issues.length === 0) {
    return null;
  }

  return (
    <div className="space-y-1">
      {issues.map((issue) => (
        <div
          key={`${tone}-${issue.field}-${issue.message}`}
          className={tone === "error" ? "text-xs text-red-300" : "text-xs text-amber-200"}
        >
          {issue.message}
        </div>
      ))}
    </div>
  );
}

export function CustomSpriteManager({
  sprites,
  deleteCustomSprite,
  getSpritePrompt,
  getSpriteManifestTemplate,
  loadSpriteManifestFile,
  generateSpriteManifestDraft,
  generateSpriteManifestWithAgent,
  validateSpriteManifest,
  saveCustomSprite,
}: CustomSpriteManagerProps) {
  const { t } = useTranslation();
  const { toasts, removeToast, success } = useToast();
  const [template, setTemplate] = useState<SpriteManifest | null>(null);
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);
  const [manifest, setManifest] = useState<SpriteManifest | null>(null);
  const [manifestJson, setManifestJson] = useState("");
  const [manifestJsonError, setManifestJsonError] = useState<string | null>(null);
  const [validation, setValidation] = useState<SpriteManifestValidation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [isGeneratingWithAgent, setIsGeneratingWithAgent] = useState(false);

  useEffect(() => {
    void Promise.all([getSpritePrompt(), getSpriteManifestTemplate()])
      .then(([, nextTemplate]) => {
        setTemplate(nextTemplate);
      })
      .catch((err) => setError(String(err)));
  }, [getSpriteManifestTemplate, getSpritePrompt]);

  const prompt = t("settings.customSprite.promptText");

  const customSprites = useMemo(
    () => sprites.filter((sprite) => sprite.source === "custom"),
    [sprites],
  );

  const setManifestAndJson = (nextManifest: SpriteManifest) => {
    setManifest(nextManifest);
    setManifestJson(JSON.stringify(nextManifest, null, 2));
    setManifestJsonError(null);
  };

  useEffect(() => {
    if (!imagePath) {
      setPreviewSrc(null);
      return;
    }

    let cancelled = false;
    void loadLocalSpriteImage(imagePath)
      .then((src) => {
        if (!cancelled) {
          setPreviewSrc(src);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
          setPreviewSrc(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [imagePath]);

  const updateManifest = (patch: Partial<SpriteManifest>) => {
    setManifest((prev) => {
      if (!prev) {
        return prev;
      }

      const next = { ...prev, ...patch };
      setManifestJson(JSON.stringify(next, null, 2));
      setManifestJsonError(null);
      return next;
    });
  };

  const updateLayout = (field: "columns" | "rows", value: number) => {
    setManifest((prev) => {
      if (!prev) return prev;
      const next = {
        ...prev,
        layout: {
          ...prev.layout,
          [field]: value,
        },
      };
      setManifestJson(JSON.stringify(next, null, 2));
      setManifestJsonError(null);
      return next;
    });
  };

  const handleManifestJsonChange = (value: string) => {
    setManifestJson(value);
    try {
      const parsed = JSON.parse(value) as SpriteManifest;
      setManifest(parsed);
      setManifestJsonError(null);
    } catch {
      setManifestJsonError(t("settings.customSprite.json.invalidJson"));
    }
  };

  const handleCopyPrompt = async () => {
    try {
      await navigator.clipboard.writeText(prompt);
      success(t("settings.customSprite.toast.promptCopied"));
    } catch (err) {
      setError(String(err));
    }
  };

  const handleCopyTemplate = async () => {
    if (!template) {
      return;
    }
    try {
      await navigator.clipboard.writeText(JSON.stringify(template, null, 2));
      success(t("settings.customSprite.toast.manifestTemplateCopied"));
    } catch (err) {
      setError(String(err));
    }
  };

  const handleSelectImage = async () => {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "Sprite image", extensions: ["png", "webp", "jpg", "jpeg"] }],
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }

    setImagePath(selected);
    setError(null);
    setValidation(null);

    const baseName = selected.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") || "custom-sprite";
    const name = baseName
      .split(/[-_]/)
      .filter(Boolean)
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join(" ");

    if (!template) {
      return;
    }

    setIsBusy(true);
    setIsGeneratingWithAgent(true);
    try {
      const generated = await generateSpriteManifestWithAgent({
        imagePath: selected,
        name,
        description: template.description,
        columns: template.layout.columns,
        rows: template.layout.rows,
        scale: template.scale ?? 0.35,
        frameRate: template.frameRate ?? 6,
        useChromaKey: true,
        pixelArt: template.chromaKey.pixelArt ?? false,
      });
      setManifestAndJson(generated.manifest);
      setValidation(null);
      success(t("settings.customSprite.toast.generatedByAgent"));
    } catch (err) {
      try {
        const fallback = await generateSpriteManifestDraft({
          imagePath: selected,
          name,
          description: template.description,
          columns: template.layout.columns,
          rows: template.layout.rows,
          scale: template.scale ?? 0.35,
          frameRate: template.frameRate ?? 6,
          useChromaKey: true,
          pixelArt: template.chromaKey.pixelArt ?? false,
        });
        setManifestAndJson(fallback.manifest);
        setValidation(null);
        setError(null);
        success(t("settings.customSprite.toast.generatedByFallback"));
      } catch (fallbackErr) {
        setError(String(fallbackErr ?? err));
      }
    } finally {
      setIsGeneratingWithAgent(false);
      setIsBusy(false);
    }
  };

  const handleLoadManifest = async () => {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "Manifest JSON", extensions: ["json"] }],
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }

    try {
      const nextManifest = await loadSpriteManifestFile(selected);
      setManifestAndJson(nextManifest);
      setValidation(null);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleValidate = async () => {
    if (!imagePath || !manifest) {
      return;
    }
    setIsBusy(true);
    try {
      setValidation(await validateSpriteManifest({ imagePath, manifest }));
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsBusy(false);
    }
  };

  const handleSave = async () => {
    if (!imagePath || !manifest) {
      return;
    }
    if (manifestJsonError) {
      setError(manifestJsonError);
      return;
    }
    setIsBusy(true);
    try {
      const nextValidation = await validateSpriteManifest({ imagePath, manifest });
      setValidation(nextValidation);
      if (nextValidation.errors.length > 0) {
        return;
      }
      await saveCustomSprite({ imagePath, manifest });
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsBusy(false);
    }
  };

  return (
    <>
      <NotificationToast toasts={toasts} onRemove={removeToast} />
      <section className="space-y-4 rounded-xl border border-glass-border bg-glass/20 p-4">
        <div className="space-y-2">
          <div className="flex items-center justify-between gap-2">
            <div>
              <h4 className="text-sm font-semibold text-text-primary">{t("settings.customSprite.title")}</h4>
              <p className="text-xs text-text-muted">
                {t("settings.customSprite.description")}
              </p>
            </div>
            <Button type="button" variant="ghost" size="sm" onClick={() => void handleCopyPrompt()}>
              <Copy size={14} />
              {t("settings.customSprite.copyPrompt")}
            </Button>
          </div>
          <div className="flex justify-end">
            <Button type="button" variant="ghost" size="sm" disabled={!template} onClick={() => void handleCopyTemplate()}>
              <FileJson size={14} />
              {t("settings.customSprite.copyManifestTemplate")}
            </Button>
          </div>
          <textarea
            value={prompt}
            readOnly
            className="min-h-28 w-full rounded-xl border border-glass-border bg-space-surface px-3 py-2 text-xs text-text-secondary"
          />
        </div>

      <div className="flex flex-wrap gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={() => void handleSelectImage()}>
          <ImagePlus size={14} />
          {t("settings.customSprite.uploadImage")}
        </Button>
        <Button type="button" variant="ghost" size="sm" onClick={() => void handleLoadManifest()}>
          <FileJson size={14} />
          {t("settings.customSprite.uploadManifest")}
        </Button>
        <Button type="button" variant="ghost" size="sm" disabled={!imagePath || !manifest || isBusy} onClick={() => void handleValidate()}>
          <CheckCircle2 size={14} />
          {t("settings.customSprite.validateDraft")}
        </Button>
      </div>

      {imagePath && (
        <div className="text-xs text-text-muted break-all">{t("settings.customSprite.imagePath", { path: imagePath })}</div>
      )}

      {isGeneratingWithAgent && (
        <div className="rounded-lg border border-accent-teal/30 bg-accent-teal/10 p-3 text-xs text-text-secondary">
          {t("settings.customSprite.status.generatingWithAgent")}
        </div>
      )}

      {manifest && (
        <div className="grid gap-3 md:grid-cols-2">
          <div className="space-y-3">
            <div className="grid gap-3 md:grid-cols-2">
              <div className="space-y-1">
                <label className="text-xs text-text-muted">{t("settings.customSprite.fields.name")}</label>
                <Input value={manifest.name} onChange={(event) => updateManifest({ name: event.target.value })} />
              </div>
              <div className="space-y-1">
                <label className="text-xs text-text-muted">{t("settings.customSprite.fields.id")}</label>
                <Input value={manifest.id} onChange={(event) => updateManifest({ id: event.target.value })} />
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-xs text-text-muted">{t("settings.customSprite.fields.description")}</label>
              <Input value={manifest.description} onChange={(event) => updateManifest({ description: event.target.value })} />
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="space-y-1">
                <label className="text-xs text-text-muted">{t("settings.customSprite.fields.columns")}</label>
                <Input type="number" min={1} value={manifest.layout.columns} onChange={(event) => updateLayout("columns", Number(event.target.value))} />
              </div>
              <div className="space-y-1">
                <label className="text-xs text-text-muted">{t("settings.customSprite.fields.rows")}</label>
                <Input type="number" min={1} value={manifest.layout.rows} onChange={(event) => updateLayout("rows", Number(event.target.value))} />
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="space-y-1">
                <label className="text-xs text-text-muted">{t("settings.customSprite.fields.scale")}</label>
                <Input
                  type="number"
                  step="0.01"
                  min={0.05}
                  value={manifest.scale ?? 0.35}
                  onChange={(event) => updateManifest({ scale: Number(event.target.value) })}
                />
              </div>
              <div className="space-y-1">
                <label className="text-xs text-text-muted">{t("settings.customSprite.fields.frameRate")}</label>
                <Input
                  type="number"
                  min={1}
                  value={manifest.frameRate ?? 6}
                  onChange={(event) => updateManifest({ frameRate: Number(event.target.value) })}
                />
              </div>
            </div>

            <div className="flex items-center gap-2 text-sm text-text-secondary">
              <Checkbox
                checked={manifest.chromaKey.pixelArt ?? false}
                onCheckedChange={(checked) => updateManifest({
                  chromaKey: {
                    ...manifest.chromaKey,
                    pixelArt: checked === true,
                  },
                })}
              />
              {t("settings.customSprite.fields.pixelArt")}
            </div>

            <div className="space-y-1">
              <label className="text-xs text-text-muted">{t("settings.customSprite.json.title")}</label>
              <textarea
                value={manifestJson}
                onChange={(event) => handleManifestJsonChange(event.target.value)}
                className="min-h-56 w-full rounded-xl border border-glass-border bg-space-surface px-3 py-2 text-xs font-mono text-text-secondary"
              />
              {manifestJsonError && <div className="text-xs text-red-300">{manifestJsonError}</div>}
            </div>

            <div className="flex flex-wrap gap-2">
              <Button type="button" size="sm" disabled={isBusy} onClick={() => void handleSave()}>
                {t("settings.customSprite.saveCustomSprite")}
              </Button>
            </div>
          </div>

          <div className="space-y-3 rounded-xl border border-glass-border bg-space-surface/60 p-3">
            <div>
              <div className="text-xs font-medium uppercase tracking-wider text-text-muted">{t("settings.customSprite.preview")}</div>
            </div>
            <div className="flex min-h-32 items-center justify-center rounded-xl bg-space-overlay/60">
              {previewSrc ? (
                <SpriteAnimation
                  animation="idle"
                  frameRate={manifest.frameRate || 6}
                  scale={manifest.scale ?? 0.35}
                  chromaKey={manifest.chromaKey}
                  imageSrc={previewSrc}
                  columns={manifest.layout.columns}
                  rows={manifest.layout.rows}
                  pixelArt={manifest.chromaKey.pixelArt}
                />
              ) : null}
            </div>

            {validation && (
              <div className="space-y-3">
                {validation.errors.length > 0 && (
                  <div className="space-y-1 rounded-lg border border-red-400/30 bg-red-500/10 p-3">
                    <div className="flex items-center gap-2 text-xs font-medium text-red-200">
                      <AlertTriangle size={14} />
                      {t("settings.customSprite.blockingIssues")}
                    </div>
                    {renderIssues(validation.errors, "error")}
                  </div>
                )}

              </div>
            )}

          </div>
        </div>
      )}

      {error && <div className="text-sm text-red-300">{error}</div>}

        {customSprites.length > 0 && (
          <div className="space-y-2">
            <div className="text-xs font-medium uppercase tracking-wider text-text-muted">{t("settings.customSprite.savedSprites")}</div>
            <div className="space-y-2">
              {customSprites.map((sprite) => (
                <div key={sprite.id} className="flex items-center justify-between rounded-lg border border-glass-border bg-space-surface/50 px-3 py-2">
                  <div>
                    <div className="text-sm text-text-primary">{sprite.name}</div>
                    <div className="text-xs text-text-muted">{sprite.id}</div>
                  </div>
                  <Button type="button" variant="ghost" size="sm" onClick={() => void deleteCustomSprite(sprite.id)}>
                    <Trash2 size={14} />
                    {t("common.delete")}
                  </Button>
                </div>
              ))}
            </div>
          </div>
        )}
      </section>
    </>
  );
}
