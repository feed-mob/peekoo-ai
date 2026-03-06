import { useEffect, useRef } from "react";
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
}

// Sprite sheet configuration
const SPRITE_CONFIG = {
  columns: 8,
  rows: 7,
  imageSrc: "/sprite.png",
};

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
    img.src = SPRITE_CONFIG.imageSrc;
    img.onload = () => {
      const atlas = buildAtlasGrid(
        img.width,
        img.height,
        SPRITE_CONFIG.columns,
        SPRITE_CONFIG.rows,
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
        canvasRef.current.width = Math.max(1, Math.round(atlas.nominalFrameWidth * scale));
        canvasRef.current.height = Math.max(
          1,
          Math.round(atlas.nominalFrameHeight * scale),
        );
      }
    };
  }, [scale, chromaKey]);

  // Update row when animation changes
  useEffect(() => {
    currentRowRef.current = ANIMATION_ROWS[animation];
    frameRef.current = 0;
    lastFrameTimeRef.current = 0;
  }, [animation]);

  // Animation loop
  useEffect(() => {
    const animate = (currentTime: number) => {
      if (!canvasRef.current || !sourceRef.current || !atlasRef.current) {
        animationRef.current = requestAnimationFrame(animate);
        return;
      }

      const ctx = canvasRef.current.getContext("2d");
      if (!ctx) {
        animationRef.current = requestAnimationFrame(animate);
        return;
      }

      ctx.imageSmoothingEnabled = true;
      ctx.imageSmoothingQuality = "high";

      const frameInterval = 1000 / frameRate;
      
      if (currentTime - lastFrameTimeRef.current >= frameInterval) {
        const row = currentRowRef.current;
        const col = frameRef.current;
        const sourceFrame = trimFrameTop(
          getFrameRect(atlasRef.current, row, col),
          ROW_TOP_TRIM_PIXELS[row] ?? 0,
        );

        ctx.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
        ctx.drawImage(
          sourceRef.current,
          sourceFrame.sx,
          sourceFrame.sy,
          sourceFrame.sw,
          sourceFrame.sh,
          0,
          0,
          canvasRef.current.width,
          canvasRef.current.height
        );

        frameRef.current = (frameRef.current + 1) % SPRITE_CONFIG.columns;
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
  }, [frameRate, onFrameChange]);

  return (
    <canvas
      ref={canvasRef}
      className={`sprite-animation ${className}`}
      style={{
        imageRendering: "pixelated",
        pointerEvents: "none",
      }}
    />
  );
}
