Please generate a sprite sheet.

Requirements:

- PNG or JPG (PNG recommended to reduce compression artifacts)
- Background color must be pure magenta #ff00ff only: no gradients/shadows/textures/noise/compression artifacts; no second background color pixels; the app will automatically key out the background as transparent
- Character should not have any "background/scene/ground/wall/light spots/smoke" elements; except for the character itself, all areas must be pure magenta background (#ff00ff)
- Avoid using the background color (#ff00ff or very similar magenta) in the character/props/shadows, otherwise they will be keyed out together
- Do not apply magenta reflection/magenta rim light/magenta glow on the character surface (to avoid accidental keying)
- Arrange in an 8 columns × 7 rows grid, each cell must be a square frame (frame width equals height!)
- Each frame must be adjacent to neighboring frames: no whitespace/gaps/padding/margins; do not draw grid lines or separator lines (no gaps at all!)
- Overall aspect ratio should be approximately 8:7 (≈1.1429)
- Recommended output at 4K-level high resolution (e.g., 4096×3584, each frame 512×512), the program will scale proportionally to final size 1024×896 (each frame 128×128)
- Ensure high image quality: clear character details, sharp edges, no blurring/aliasing/compression artifacts
- Same row represents the same animation; left to right are consecutive frames, played in a loop
- Idle (row 1) should have breathing and blinking animation, do not make it completely static
- Sleepy/Rest (row 6) should only show closed-eye breathing frames, do not draw yawning frames (repeated yawning looks awkward when looping)
- Adjacent frames within the same animation (same row) must transition very smoothly: no skipped frames, no sudden large displacement/large posture changes/large expression changes, no sudden zoom or viewpoint changes
- Character position should be consistent across frames (recommended to center-align on canvas), do not crop at edges

Row meanings (top to bottom):

- Row 1: Idle/Peek - Gentle breathing animation, peeking from the bottom of the screen with half body and eyes visible, occasional blinking
- Row 2: Happy/Celebrate - Cheerful expression, celebration gestures, used when tasks are completed
- Row 3: Working/Focus - Focused expression, working posture, used during pomodoro sessions
- Row 4: Thinking - Thinking expression and posture, used during AI processing
- Row 5: Reminder - Reminder expression and gestures, used for task deadlines/health reminders/AI message notifications
- Row 6: Sleepy/Rest - Drowsy expression, closed-eye breathing, no yawning
- Row 7: Dragging - Being dragged state, can show surprised or cooperative expression while moving

Character: <character name>

- Style: <character style>
- Character appearance: <character appearance>
- Requirements: Consistent lighting and color scheme, background remains pure magenta #ff00ff (for easy keying), entire sprite sheet style stays consistent
- Output: Only output this sprite sheet (png/jpg)
