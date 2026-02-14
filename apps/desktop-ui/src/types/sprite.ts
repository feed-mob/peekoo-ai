export type AnimationType = 
  | "idle"      // Row 0: breathing + blinking
  | "happy"     // Row 1: happy/love
  | "excited"   // Row 2: excited/celebrate
  | "sleepy"    // Row 3: sleepy/snoring (eyes closed)
  | "working"   // Row 4: working
  | "angry"     // Row 5: angry/surprised/shy
  | "dragging"; // Row 6: dragging

export interface SpriteState {
  mood: "happy" | "sad" | "excited" | "thinking" | "idle" | "tired" | "surprised" | string;
  message: string;
  animation: string;
}
