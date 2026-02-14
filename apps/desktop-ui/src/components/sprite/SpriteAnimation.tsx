import { useEffect, useRef } from "react";
import type { AnimationType } from "@/types/sprite";
import {
  buildKeyedSpriteSheet,
  type ChromaKeyOptions,
} from "./chromaKey";

interface SpriteAnimationProps {
  animation?: AnimationType;
  frameRate?: number;
  scale?: number;
  chromaKey?: false | Partial<ChromaKeyOptions>;
  onFrameChange?: (frameIndex: number) => void;
  className?: string;
}

// Sprite sheet configuration (matches manifest.json)
const SPRITE_CONFIG = {
  columns: 8,
  rows: 7,
  frameWidth: 552,
  frameHeight: 548,
  imageSrc: "/sprite.jpg",
};

// Animation row mapping
const ANIMATION_ROWS: Record<AnimationType, number> = {
  idle: 0,
  happy: 1,
  excited: 2,
  sleepy: 3,
  working: 4,
  angry: 5,
  dragging: 6,
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

  // Load sprite sheet
  useEffect(() => {
    const img = new Image();
    img.src = SPRITE_CONFIG.imageSrc;
    img.onload = () => {
      sourceRef.current =
        chromaKey === false ? img : buildKeyedSpriteSheet(img, chromaKey);
      if (canvasRef.current) {
        canvasRef.current.width = SPRITE_CONFIG.frameWidth * scale;
        canvasRef.current.height = SPRITE_CONFIG.frameHeight * scale;
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
      if (!canvasRef.current || !sourceRef.current) {
        animationRef.current = requestAnimationFrame(animate);
        return;
      }

      const ctx = canvasRef.current.getContext("2d");
      if (!ctx) {
        animationRef.current = requestAnimationFrame(animate);
        return;
      }

      ctx.imageSmoothingEnabled = false;

      const frameInterval = 1000 / frameRate;
      
      if (currentTime - lastFrameTimeRef.current >= frameInterval) {
        const row = currentRowRef.current;
        const col = frameRef.current;
        const sx = col * SPRITE_CONFIG.frameWidth;
        const sy = row * SPRITE_CONFIG.frameHeight;

        ctx.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
        ctx.drawImage(
          sourceRef.current,
          sx,
          sy,
          SPRITE_CONFIG.frameWidth,
          SPRITE_CONFIG.frameHeight,
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
      }}
    />
  );
}
