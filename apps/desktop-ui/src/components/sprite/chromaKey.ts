export type RgbColor = readonly [number, number, number];

export interface SpillSuppressionOptions {
  enabled: boolean;
  threshold: number;
  strength: number;
}

export interface ChromaKeyOptions {
  targetColor: RgbColor;
  minRbOverG: number;
  threshold: number;
  softness: number;
  spillSuppression: SpillSuppressionOptions;
  stripDarkFringe?: boolean;
}

export const DEFAULT_CHROMA_KEY_OPTIONS: ChromaKeyOptions = {
  targetColor: [255, 0, 255],
  minRbOverG: 35,
  threshold: 78,
  softness: 36,
  spillSuppression: {
    enabled: true,
    threshold: 130,
    strength: 0.45,
  },
  stripDarkFringe: false,
};

function clampByte(value: number): number {
  return Math.min(255, Math.max(0, Math.round(value)));
}

function mergeOptions(options?: Partial<ChromaKeyOptions>): ChromaKeyOptions {
  if (!options) {
    return DEFAULT_CHROMA_KEY_OPTIONS;
  }

  return {
    targetColor: options.targetColor ?? DEFAULT_CHROMA_KEY_OPTIONS.targetColor,
    minRbOverG: options.minRbOverG ?? DEFAULT_CHROMA_KEY_OPTIONS.minRbOverG,
    threshold: options.threshold ?? DEFAULT_CHROMA_KEY_OPTIONS.threshold,
    softness: options.softness ?? DEFAULT_CHROMA_KEY_OPTIONS.softness,
    stripDarkFringe: options.stripDarkFringe ?? DEFAULT_CHROMA_KEY_OPTIONS.stripDarkFringe,
    spillSuppression: {
      enabled:
        options.spillSuppression?.enabled ??
        DEFAULT_CHROMA_KEY_OPTIONS.spillSuppression.enabled,
      threshold:
        options.spillSuppression?.threshold ??
        DEFAULT_CHROMA_KEY_OPTIONS.spillSuppression.threshold,
      strength:
        options.spillSuppression?.strength ??
        DEFAULT_CHROMA_KEY_OPTIONS.spillSuppression.strength,
    },
  };
}

function isMagentaCandidate(
  r: number,
  g: number,
  b: number,
  minRbOverG: number,
): boolean {
  return r - g >= minRbOverG && b - g >= minRbOverG;
}

function getAlphaFromDistance(
  distanceSq: number,
  thresholdSq: number,
  edgeThresholdSq: number,
): number {
  if (distanceSq <= thresholdSq) {
    return 0;
  }

  if (distanceSq >= edgeThresholdSq) {
    return 255;
  }

  const ratio =
    (distanceSq - thresholdSq) / (edgeThresholdSq - thresholdSq);
  return clampByte(ratio * 255);
}

function applySpillSuppression(
  r: number,
  g: number,
  b: number,
  distanceSq: number,
  options: SpillSuppressionOptions,
): [number, number, number] {
  if (!options.enabled || distanceSq > options.threshold * options.threshold) {
    return [r, g, b];
  }

  const thresholdSq = options.threshold * options.threshold;
  const edgeFactor = 1 - Math.min(1, distanceSq / thresholdSq);
  const magentaBias = Math.max(0, ((r + b) * 0.5 - g) / 255);
  const factor = options.strength * edgeFactor * (0.35 + magentaBias * 0.65);

  const nextR = clampByte(r * (1 - factor) + g * factor);
  const nextB = clampByte(b * (1 - factor) + g * factor);

  return [nextR, g, nextB];
}

export function applyChromaKeyToImageData(
  imageData: ImageData,
  partialOptions?: Partial<ChromaKeyOptions>,
): ImageData {
  const options = mergeOptions(partialOptions);
  const [targetR, targetG, targetB] = options.targetColor;
  const thresholdSq = options.threshold * options.threshold;
  const edgeThreshold = options.threshold + options.softness;
  const edgeThresholdSq = edgeThreshold * edgeThreshold;
  const data = imageData.data;

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i];
    const g = data[i + 1];
    const b = data[i + 2];

    if (!isMagentaCandidate(r, g, b, options.minRbOverG)) {
      continue;
    }

    if (options.stripDarkFringe) {
      // For this specific case, any "Magenta Candidate" is safe to remove
      // because the sprite (black cat) contains NO purple hues.
      data[i + 3] = 0;
      continue;
    }

    const dr = r - targetR;
    const dg = g - targetG;
    const db = b - targetB;
    const distanceSq = dr * dr + dg * dg + db * db;
    const alpha = getAlphaFromDistance(distanceSq, thresholdSq, edgeThresholdSq);

    const [nextR, nextG, nextB] = applySpillSuppression(
      r,
      g,
      b,
      distanceSq,
      options.spillSuppression,
    );

    data[i] = nextR;
    data[i + 1] = nextG;
    data[i + 2] = nextB;
    data[i + 3] = alpha;
  }

  return imageData;
}

export function buildKeyedSpriteSheet(
  image: HTMLImageElement,
  options?: Partial<ChromaKeyOptions>,
): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = image.width;
  canvas.height = image.height;

  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) {
    return canvas;
  }

  ctx.drawImage(image, 0, 0);
  const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const keyed = applyChromaKeyToImageData(imageData, options);
  ctx.putImageData(keyed, 0, 0);

  return canvas;
}
