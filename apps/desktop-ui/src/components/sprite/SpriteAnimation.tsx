import { useEffect, useRef, useLayoutEffect } from "react";
import type { AnimationType } from "@/types/sprite";
import {
  buildKeyedSpriteSheet,
  type ChromaKeyOptions,
} from "./chromaKey";
import {
  buildAtlasGrid,
  getFrameRect,
  trimFrameTop,
  validateAtlas,
  type SpriteAtlasGrid,
} from "./spriteAtlas";

interface SpriteAnimationProps {
  animation?: AnimationType;
  frameRate?: number;
  scale?: number;
  chromaKey?: false | Partial<ChromaKeyOptions>;
  onFrameChange?: (frameIndex: number) => void;
  className?: string;
  imageSrc: string;
  columns: number;
  rows: number;
  pixelArt?: boolean;
}

// Animation row mapping (matches new sprite sheet layout)
const ANIMATION_ROWS: Record<AnimationType, number> = {
  idle: 0,      // Row 0: Idle/Peek
  happy: 1,     // Row 1: Happy/Celebrate
  working: 2,   // Row 2: Working/Focus
  thinking: 3,  // Row 3: Thinking
  reminder: 4,  // Row 4: Reminder
  sleepy: 5,    // Row 5: Sleepy/Rest
  dragging: 6,  // Row 6: Dragging
};

const ROW_TOP_TRIM_PIXELS: Partial<Record<number, number>> = {
  4: 9,
  5: 12,
};

export default function SpriteAnimation({
  animation = "idle",
  frameRate = 8,
  scale = 0.3,
  chromaKey,
  onFrameChange,
  className = "",
  imageSrc,
  columns,
  rows,
  pixelArt = false, // Default to false (smooth illustration)
}: SpriteAnimationProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sourceRef = useRef<CanvasImageSource | null>(null);
  const frameRef = useRef(0);
  const animationRef = useRef<number>();
  const currentRowRef = useRef(ANIMATION_ROWS[animation]);
  const lastFrameTimeRef = useRef<number>(0);
  const atlasRef = useRef<SpriteAtlasGrid | null>(null);

  // Load sprite sheet
  useEffect(() => {
    const img = new Image();
    img.src = imageSrc;
    img.onload = () => {
      const atlas = buildAtlasGrid(
        img.width,
        img.height,
        columns,
        rows,
      );
      atlasRef.current = atlas;

      if (import.meta.env.DEV) {
        const validation = validateAtlas(atlas, img.width, img.height);
        if (validation.warnings.length > 0 || validation.errors.length > 0) {
          console.warn("[SpriteAnimation] atlas validation", validation);
        }
      }

      sourceRef.current =
        chromaKey === false ? img : buildKeyedSpriteSheet(img, chromaKey);

      if (canvasRef.current) {
        // Use window.devicePixelRatio for crisp rendering on high-DPI displays
        const dpr = window.devicePixelRatio || 1;
        const displayWidth = Math.max(1, Math.round(atlas.nominalFrameWidth * scale));
        const displayHeight = Math.max(1, Math.round(atlas.nominalFrameHeight * scale));

        canvasRef.current.width = displayWidth * dpr;
        canvasRef.current.height = displayHeight * dpr;
        
        // Scale context to match dpr
        const ctx = canvasRef.current.getContext("2d");
        if (ctx) {
           ctx.scale(dpr, dpr);
        }
        
        // Set CSS size
        canvasRef.current.style.width = `${displayWidth}px`;
        canvasRef.current.style.height = `${displayHeight}px`;
      }
    };
  }, [scale, chromaKey, imageSrc, columns, rows]);


  // Draw a single frame. This is extracted so we can call it immediately on animation change.
  const drawFrame = (_currentTime: number, forceRow?: number) => {
    if (!canvasRef.current || !sourceRef.current || !atlasRef.current) return;
    
    const ctx = canvasRef.current.getContext("2d");
    if (!ctx) return;
    
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0); 

    const row = forceRow !== undefined ? forceRow : currentRowRef.current;
    const col = forceRow !== undefined ? 0 : frameRef.current;
    
    const sourceFrame = trimFrameTop(
      getFrameRect(atlasRef.current, row, col),
      ROW_TOP_TRIM_PIXELS[row] ?? 0,
    );

    const displayWidth = parseFloat(canvasRef.current.style.width || "0");
    const displayHeight = parseFloat(canvasRef.current.style.height || "0");
    
    // Calculate targeted height based on the actual (trimmed) source height to prevent stretching
    const targetHeight = sourceFrame.sh * scale;
    // Calculate horizontal offset if we wanted centering, but currently we stay top-aligned or left-aligned
    // Typically these sprites are centered in their frame
    const targetWidth = sourceFrame.sw * scale;

    ctx.clearRect(0, 0, displayWidth, displayHeight);
    ctx.drawImage(
      sourceRef.current,
      sourceFrame.sx,
      sourceFrame.sy,
      sourceFrame.sw,
      sourceFrame.sh,
      (displayWidth - targetWidth) / 2, // Center horizontally in canvas
      0, // Draw from top
      targetWidth,
      targetHeight
    );
  };

  // Update row when animation changes and force an immediate draw
  useLayoutEffect(() => {
    const targetRow = ANIMATION_ROWS[animation];
    currentRowRef.current = targetRow;
    frameRef.current = 0;
    lastFrameTimeRef.current = 0;
    
    // Force an immediate draw of the first frame of the new animation
    // This is critical for dragging because the thread will soon be blocked by OS
    drawFrame(performance.now(), targetRow);
  }, [animation]);

  // Animation loop
  useEffect(() => {
    const animate = (currentTime: number) => {
      const frameInterval = 1000 / frameRate;
      
      if (currentTime - lastFrameTimeRef.current >= frameInterval) {
        drawFrame(currentTime);
        frameRef.current = (frameRef.current + 1) % columns;
        lastFrameTimeRef.current = currentTime;
        onFrameChange?.(frameRef.current);
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    animationRef.current = requestAnimationFrame(animate);
    
    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [frameRate, onFrameChange, columns]);

  return (
    <canvas
      ref={canvasRef}
      className={`sprite-animation ${className}`}
      style={{
        imageRendering: pixelArt ? "pixelated" : "auto",
        pointerEvents: "none",
      }}
    />
  );
}
