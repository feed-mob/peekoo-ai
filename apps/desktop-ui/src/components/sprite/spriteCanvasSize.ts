interface SpriteCanvasSizeInput {
  nominalFrameWidth: number;
  nominalFrameHeight: number;
  scale: number;
  devicePixelRatio: number;
}

interface SpriteCanvasSize {
  displayWidth: number;
  displayHeight: number;
  canvasWidth: number;
  canvasHeight: number;
  devicePixelRatio: number;
}

export function getSpriteCanvasSize({
  nominalFrameWidth,
  nominalFrameHeight,
  scale,
  devicePixelRatio,
}: SpriteCanvasSizeInput): SpriteCanvasSize {
  const normalizedDpr = Number.isFinite(devicePixelRatio) && devicePixelRatio > 0
    ? devicePixelRatio
    : 1;
  const displayWidth = Math.max(1, Math.round(nominalFrameWidth * scale));
  const displayHeight = Math.max(1, Math.round(nominalFrameHeight * scale));

  return {
    displayWidth,
    displayHeight,
    canvasWidth: Math.max(1, Math.round(displayWidth * normalizedDpr)),
    canvasHeight: Math.max(1, Math.round(displayHeight * normalizedDpr)),
    devicePixelRatio: normalizedDpr,
  };
}
