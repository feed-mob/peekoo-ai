export type AnimationType =
  | "idle"      // Row 0: Idle/Peek - gentle breathing, peeking from bottom, occasional blinking
  | "happy"     // Row 1: Happy/Celebrate - joyful expression, celebration on task completion
  | "working"   // Row 2: Working/Focus - focused expression, working posture during pomodoro
  | "thinking"  // Row 3: Thinking - thinking expression and posture during AI processing
  | "reminder"  // Row 4: Reminder - reminder expression/action for deadlines and notifications
  | "sleepy"    // Row 5: Sleepy/Rest - tired expression, closed eyes breathing
  | "dragging"; // Row 6: Dragging - being dragged state, surprised or cooperative expression

export interface SpriteState {
  mood: "happy" | "sad" | "thinking" | "idle" | "tired" | "reminder" | string;
  message: string;
  animation: string;
}

export interface SpriteManifest {
  id: string;
  name: string;
  description: string;
  image: string;
  layout: {
    columns: number;
    rows: number;
  };
  scale?: number;
  frameRate?: number;
  chromaKey: {
    targetColor: readonly [number, number, number];
    minRbOverG: number;
    threshold: number;
    softness: number;
    spillSuppression: {
      enabled: boolean;
      threshold: number;
      strength: number;
    };
    stripDarkFringe?: boolean;
    pixelArt?: boolean;
  };
}

export interface ValidationIssue {
  field: string;
  message: string;
}

export type SpriteBackgroundMode = "transparent" | "flatColor" | "opaque";

export interface SpriteImageValidation {
  imageWidth: number;
  imageHeight: number;
  frameWidth: number | null;
  frameHeight: number | null;
  hasAlpha: boolean;
  backgroundMode: SpriteBackgroundMode;
  blankFrameCount: number;
  errors: ValidationIssue[];
  warnings: ValidationIssue[];
}

export interface SpriteManifestValidation {
  errors: ValidationIssue[];
  warnings: ValidationIssue[];
}

export interface GeneratedSpriteManifest {
  manifest: SpriteManifest;
  imageValidation: SpriteImageValidation;
  manifestValidation: SpriteManifestValidation;
}

export interface CustomSpriteManifestFile {
  manifest: SpriteManifest;
  imagePath: string;
}

export interface GenerateSpriteManifestInput {
  imagePath: string;
  name: string;
  description?: string | null;
  columns: number;
  rows: number;
  scale?: number | null;
  frameRate?: number | null;
  useChromaKey: boolean;
  pixelArt: boolean;
}

export interface ValidateSpriteManifestInput {
  imagePath: string;
  manifest: SpriteManifest;
}

export interface SaveCustomSpriteInput {
  imagePath: string;
  manifest: SpriteManifest;
}
