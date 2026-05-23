# Advanced Interaction Patterns -- Chords, Modes, and Layers

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Novel Use Cases & Ideas

---

## Key Findings

- The Stream Deck XL reports a full 32-byte state array on every button change, making simultaneous multi-button detection trivial at the hardware level -- no firmware hacks needed
- QMK/ZMK keyboard firmware and fighting game input buffers provide battle-tested architectures for chord detection, tap-hold discrimination, and layer switching
- Aircraft MFD bezel-key panels are the closest physical analog to the Stream Deck's "labeled button next to screen" paradigm, and their design rules (unambiguous label alignment, minimal head-down time) translate directly
- With 32 buttons, there are 4,294,967,296 possible button-state combinations; practical chord sets should stay under 50 defined chords to remain learnable
- Modal layer systems (inspired by Vim, QMK layers, and Ableton Push pages) are the highest-leverage pattern -- a 32-button deck with 8 mode layers gives 256 effective actions with clear visual feedback
- The 96x96 pixel per-key LCD is the killer feature for modal interfaces: every mode switch can relabel every button instantly, eliminating the memorization burden that plagues keyboard layers
- Long-press, double-tap, and hold-modifier patterns can coexist cleanly if timing thresholds are well-separated (tap < 200ms, double-tap window 300ms, long-press > 500ms, hold-modifier immediate on down)
- A "mode stack" architecture (push/pop modes like a call stack) enables arbitrarily deep navigation without losing context

---

## 1. Hardware Foundation: Why the Stream Deck Enables This

### 1.1 The Full-State Input Report

Every time any button changes state (pressed or released), the Stream Deck XL sends a 512-byte HID input report. After a 4-byte header, the next 32 bytes each represent one button:

```
Byte offset:  [0x00-0x03] [0x04] [0x05] [0x06] ... [0x23]
Content:       Header      Key 0  Key 1  Key 2      Key 31
Values:                    0x00   0x01   0x00        0x00
                          (up)  (down)  (up)        (up)
```

This means the host software always knows the complete pressed/released state of all 32 buttons simultaneously. Unlike keyboards that typically report up to 6 keys via USB HID (6KRO), the Stream Deck has no rollover limitation -- you can detect all 32 buttons pressed at once.

### 1.2 The Per-Key LCD

Each key has an independent 96x96 pixel LCD that accepts JPEG images. Images can be updated individually and in real time. This is what separates the Stream Deck from a MIDI pad controller or a keyboard: every button can show exactly what it does right now.

### 1.3 Physical Grid Layout

```
    Col 0   Col 1   Col 2   Col 3   Col 4   Col 5   Col 6   Col 7
   +-------+-------+-------+-------+-------+-------+-------+-------+
R0 | Key 0 | Key 1 | Key 2 | Key 3 | Key 4 | Key 5 | Key 6 | Key 7 |
   +-------+-------+-------+-------+-------+-------+-------+-------+
R1 | Key 8 | Key 9 |Key 10 |Key 11 |Key 12 |Key 13 |Key 14 |Key 15 |
   +-------+-------+-------+-------+-------+-------+-------+-------+
R2 |Key 16 |Key 17 |Key 18 |Key 19 |Key 20 |Key 21 |Key 22 |Key 23 |
   +-------+-------+-------+-------+-------+-------+-------+-------+
R3 |Key 24 |Key 25 |Key 26 |Key 27 |Key 28 |Key 29 |Key 30 |Key 31 |
   +-------+-------+-------+-------+-------+-------+-------+-------+
```

The 8x4 grid has a natural landscape orientation. The top row is easiest to scan visually, the bottom row is closest to the user's resting hand position. This spatial layout should inform where mode selectors, modifiers, and action keys are placed.

---

## 2. Chord Combinations

### 2.1 Simultaneous Chords (Press A+B Together)

Since the Stream Deck reports full state on every change, detecting simultaneous presses is straightforward. The challenge is defining what "simultaneous" means when humans cannot physically press two buttons at the exact same millisecond.

**Detection algorithm (borrowed from QMK combos):**

```
1. Button A goes down at time T1 -> start chord timer
2. Within CHORD_WINDOW (e.g., 50ms), button B goes down at T2
3. If |T2 - T1| < CHORD_WINDOW:
     -> Emit chord(A, B) instead of individual press(A), press(B)
4. If CHORD_WINDOW expires with only A down:
     -> Emit individual press(A)
```

**Key design decision: delayed vs immediate.**
- **Delayed mode** (QMK default): Wait for the chord window to expire before emitting anything. Adds 50ms latency to every single press. Better for chords that replace common actions.
- **Immediate mode**: Emit press(A) immediately, then if B arrives within the window, retract press(A) and emit chord(A, B). More responsive for single presses but requires undo capability.

For Stream Deck use, delayed mode with a short 40-50ms window is recommended. Users will not notice 50ms of visual latency on button images, and it avoids the complexity of retracting actions.

### 2.2 Sequential Chords (Press A, Then B Within a Window)

Sequential chords are like Vim's `gg` (press g, then g again) or Emacs's `C-x C-s` (Ctrl-X, then Ctrl-S). Press one button, release it, then press another within a time window.

```
1. Button A pressed and released -> start sequence timer (300ms)
2. Button B pressed within 300ms of A's release
   -> Emit sequence(A, B)
3. Timer expires without B
   -> Emit individual press(A)  [retroactively, after 300ms delay]
```

**The latency tradeoff is more severe here.** A 300ms delay on every single press is noticeable. Mitigation strategies:

- **Leader key pattern** (from Vim/QMK): Designate one specific button as the "leader." Only presses that start with the leader enter sequence mode. All other buttons fire immediately.
- **Prefix key with visual feedback**: When the leader is pressed, immediately update all other button images to show what sequence completions are available. This turns the latency into useful thinking time.

```
IDLE STATE:                         AFTER PRESSING LEADER (Key 0):

+------+------+------+---          +------+------+------+---
| LEAD |  Git | Term | ..          | LEAD | +Git |+Term | ..
| (*)  |      |      |             | hold | push |clear |
+------+------+------+---          +------+------+------+---
|      |      |      | ..          | +New |+Open |+Save | ..
|      |      |      |             | file | file | all  |
+------+------+------+---          +------+------+------+---
```

### 2.3 Combination Math

With 32 buttons, the theoretical combinatorics:

| Chord Size | Combinations         | Practical? |
|------------|----------------------|------------|
| 2-button   | C(32,2) = 496        | Yes -- but only adjacent pairs are ergonomic |
| 3-button   | C(32,3) = 4,960      | Marginal -- hard to press precisely |
| 4-button   | C(32,4) = 35,960     | No -- too error-prone |
| Any combo  | 2^32 = 4,294,967,296 | Absurd |

**Practical recommendation:** Define at most 20-30 two-button chords, limited to buttons that are physically adjacent or in the same row. The ergonomics of a flat button grid make non-adjacent chords awkward.

### 2.4 Ergonomic Chord Zones

```
   Col 0   Col 1   Col 2   Col 3   Col 4   Col 5   Col 6   Col 7
  +-------+-------+-------+-------+-------+-------+-------+-------+
  |       |       |       |       |       |       |       |       |
  | Zone A (left hand)    :       : Zone B (right hand)           |
  |       |       |       |       |       |       |       |       |
  +-------+-------+-------+-------+-------+-------+-------+-------+
  |       |       |       |       |       |       |       |       |
  |   Comfortable chord   :       :   Comfortable chord           |
  |   pairs: horizontal   :       :   pairs: horizontal           |
  |   neighbors only      :       :   neighbors only              |
  +-------+-------+-------+-------+-------+-------+-------+-------+

  Good chords:  [Key 0 + Key 1]  (adjacent horizontal)
                [Key 0 + Key 8]  (adjacent vertical)
  Bad chords:   [Key 0 + Key 7]  (opposite ends of row)
                [Key 0 + Key 31] (opposite corners)
```

---

## 3. Mode/Layer Switching

This is the highest-leverage interaction pattern for the Stream Deck. The per-key LCD eliminates the biggest problem with keyboard layers: you can always see what every button does.

### 3.1 Architecture: Layers as a Stack

Borrowing from QMK's layer model and extending it:

```
                    +-------------------+
                    |   Active Layer    |  <- top of stack, receives input
                    +-------------------+
                    |   Parent Layer    |  <- transparent keys fall through
                    +-------------------+
                    |   Base Layer      |  <- always present, default fallback
                    +-------------------+
```

**Key concepts:**

- **Base layer**: Always at the bottom. Contains global actions (volume, mic mute, screen lock) that should be accessible from any mode.
- **Transparent keys**: A key in an upper layer can be marked "transparent" (`KC_TRNS` in QMK terminology), meaning it falls through to the layer below. This lets parent-layer actions remain visible and functional.
- **Push/Pop**: Activating a mode pushes a new layer. Pressing back (or the same mode key again) pops it. This naturally supports nested navigation.
- **Maximum depth**: Recommend limiting to 3-4 layers deep. Deeper stacks become disorienting.

### 3.2 Mode Activation Patterns

There are four distinct patterns for how a button activates a layer, each suited to different use cases:

#### Toggle Mode (Press to Enter, Press Again to Exit)

```
Press [DEV] -> Enter dev mode (all buttons show dev actions)
Press [DEV] -> Return to base layer

State machine:
  BASE --press[DEV]--> DEV_MODE --press[DEV]--> BASE
```

Best for: Long-duration modes where you will perform multiple actions before leaving (e.g., "Git mode" where you will do several git operations in a row).

#### Momentary Mode (Hold to Activate, Release to Return)

```
Hold [SHIFT] -> While held, all buttons show alternate actions
Release [SHIFT] -> Instantly return to previous layer

State machine:
  BASE --down[SHIFT]--> SHIFT_MODE --up[SHIFT]--> BASE
```

Best for: Quick one-off access to alternate functions. Analogous to holding Shift on a keyboard. The key advantage is zero-latency return to the base layer -- you always end up back where you started.

This is the Ableton Push pattern: hold a mode button to temporarily switch pad function, release to snap back.

#### One-Shot Mode (Press to Activate for Next Action Only)

```
Press [ONESHOT] -> Enter mode, next button press executes in that mode, then auto-return

State machine:
  BASE --press[OS]--> ONESHOT_MODE --press[any]--> execute(any, in ONESHOT) -> BASE
```

Best for: Infrequent modifiers. Like QMK one-shot mods -- press Shift, then press a letter, and the shift applies only to that one letter. On the Stream Deck: press a "danger" modifier, then press an action to confirm a destructive operation.

#### Layer Lock (Toggle with Visual Indicator)

```
Press [LOCK] -> Lock current layer (prevent accidental exits)
Press [LOCK] -> Unlock (normal exit behavior resumes)
```

Best for: Preventing accidental mode switches during critical workflows. The lock button's image should show a clear locked/unlocked icon.

### 3.3 Nested Modes / Mode Stack Example

Here is a concrete example of a mode stack for a software developer:

```
BASE LAYER (always accessible):
+--------+--------+--------+--------+--------+--------+--------+--------+
|  DEV   |  MEDIA |  HOME  |  COMMS |  SYS   | Vol Dn | Vol Up |  Mute  |
| mode   | mode   | mode   | mode   | mode   |   -    |   +    |  Mic   |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
|  App 1 |  App 2 |  App 3 |  App 4 |  App 5 |  App 6 |  App 7 |  App 8 |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
| Scrn 1 | Scrn 2 | Scrn 3 |        |        |        |        |  Lock  |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
|  Prev  |  Play  |  Next  |        |        |        |  Brt-  |  Brt+  |
+--------+--------+--------+--------+--------+--------+--------+--------+

                            |
                      Press [DEV]
                            v

DEV LAYER (pushed on top of base):
+--------+--------+--------+--------+--------+--------+--------+--------+
|< BACK >|  GIT   |  TERM  |  DEBUG | DOCKER |  DB    |  API   |  LOGS  |
| (base) | mode   | mode   | mode   | mode   | mode   | mode   | mode   |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
| VS Code| Chrome | iTerm  | Figma  |Postman | Slack  | Notes  |        |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
| Build  | Test   |  Lint  | Format |Type Chk|        |        |        |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
|  Run   |  Stop  |Restart | Deploy |        |        |        |        |
+--------+--------+--------+--------+--------+--------+--------+--------+

                            |
                      Press [GIT]
                            v

GIT LAYER (pushed on top of dev):
+--------+--------+--------+--------+--------+--------+--------+--------+
|< BACK >|        |        |        |        |        |        |        |
| (dev)  | Status | Diff   |  Log   | Stash  |Branches| Remote | Blame  |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
|  Add   |  Add   | Commit |Commit  |  Amend |  Push  | Pull   | Fetch  |
|  All   |  File  |        | -m     |        |        |        |        |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
|Checkout|  New   | Switch | Merge  | Rebase |  Tag   | Cherry |  Bisect|
| branch | branch | branch |        |        |        |  Pick  |        |
+--------+--------+--------+--------+--------+--------+--------+--------+
|        |        |        |        |        |        |        |        |
| Reset  | Reset  | Stash  | Stash  | Clean  |        | GitHub | GitHub |
| soft   | hard   | push   | pop    |        |        |  PR    | Issues |
+--------+--------+--------+--------+--------+--------+--------+--------+

Mode stack: [BASE] -> [DEV] -> [GIT]
Press BACK from GIT -> returns to DEV
Press BACK from DEV -> returns to BASE
```

### 3.4 Mode Indicator Design

Every mode needs a clear visual indicator so the user always knows where they are. Strategies:

1. **Color-coded backgrounds**: Base layer = dark gray, Dev = blue, Git = orange, Media = green. Every button image in that layer uses the same background tint.

2. **Breadcrumb trail on the BACK button**: The back button shows the name of the parent mode, so you know where "back" goes.

3. **Active mode highlight on top row**: If the top row contains mode selectors, the currently active mode button should have a highlighted/inverted appearance.

4. **Dim unavailable actions**: Buttons with no action in the current mode should show a dimmed or empty state, not misleading labels.

---

## 4. Gesture Patterns

### 4.1 Timing-Based Gestures

The Stream Deck reports button down and button up events. By tracking timestamps, the host software can distinguish several gesture types from a single button:

```
          PRESS              RELEASE
            |                  |
            v                  v
Time: ------[====HELD====]------->

         <-- duration -->

  Tap:        duration < 200ms
  Long press: duration > 500ms
  Hold:       continuous action while held (emitted every N ms)
```

**Recommended timing thresholds:**

| Gesture      | Threshold          | Notes                                  |
|--------------|--------------------|----------------------------------------|
| Tap          | < 200ms            | The "normal" press. Fire on release.   |
| Double-tap   | Two taps < 300ms apart | Fire on second release.            |
| Long press   | > 500ms held       | Fire once when threshold is crossed.   |
| Hold-repeat  | > 500ms, then every 100ms | For continuous actions like volume. |

**Critical: Tap must fire on release, not on press.** If tap fires on press (key-down), you cannot distinguish it from the start of a long press. The 200ms latency on taps is acceptable for most actions.

Exception: For latency-critical actions (push-to-talk, media play/pause), fire on key-down and ignore long-press detection for that button.

### 4.2 Double-Tap Detection State Machine

```
                     press
        IDLE -----------------> FIRST_DOWN
          ^                        |
          | timeout                | release (< 200ms)
          |                        v
          |                    FIRST_UP
          |                        |
          +--- timeout (300ms) ----+
          |                        |
          |                        | press (< 300ms after first release)
          |                        v
          |                    SECOND_DOWN
          |                        |
          |                        | release
          |                        v
          +---- emit single ---- EMIT_DOUBLE_TAP
                  tap (on
                  timeout)

  If the second press does NOT arrive within 300ms of the first release,
  emit a single tap retroactively.

  If the second press DOES arrive, emit a double-tap on second release.
```

**The retroactive single-tap problem:** Double-tap detection adds 300ms latency to single taps (you have to wait to see if a second tap is coming). Solutions:

- Only enable double-tap on specific buttons that need it. Most buttons should just be instant single-tap.
- Use the first tap immediately for a "preview" action (like highlighting something) and the double-tap for the "confirm" action. This follows the file-manager convention (single-click selects, double-click opens).

### 4.3 Press-and-Hold for Continuous Actions

Useful for volume control, brightness adjustment, scrolling, or any incremental action.

```
Algorithm:
1. Key down -> start hold timer
2. After HOLD_THRESHOLD (500ms) -> emit first repeat
3. Every REPEAT_INTERVAL (100ms) -> emit subsequent repeats
4. Key up -> stop repeating

Optional acceleration:
- After 2 seconds of holding, reduce REPEAT_INTERVAL to 50ms
- After 5 seconds, reduce to 25ms
- This mimics keyboard repeat acceleration for rapid traversal
```

Visual feedback: While held, the button image could show a progress indicator or the current value (e.g., "Vol: 73%").

### 4.4 Swipe-Like Patterns Across Adjacent Buttons

The Stream Deck is a flat grid, not a touchscreen, so true swipes are not possible. But you can detect a "button sweep" -- pressing a sequence of adjacent buttons in rapid succession:

```
Algorithm:
1. Track press timestamps for all buttons
2. If buttons B1, B2, B3 are pressed in order, each within 100ms of
   the previous, and B1-B2-B3 are physically adjacent:
   -> Emit swipe(direction, length)

Example - horizontal right swipe:
  Key 0 down at T=0ms
  Key 1 down at T=60ms
  Key 2 down at T=120ms
  -> swipe(RIGHT, 3)

Example - vertical down swipe:
  Key 1 down at T=0ms
  Key 9 down at T=80ms
  Key 17 down at T=150ms
  -> swipe(DOWN, 3)
```

This is exotic and probably not worth implementing in a first version, but it opens up interesting possibilities for page scrolling, list navigation, or scrubbing through a timeline.

---

## 5. State Machine Architecture

### 5.1 Per-Button State Machine

Each button can be modeled as an independent state machine that feeds into a global event system:

```
                        +----------+
               press    |          | release
        +-------------->|  PRESSED |-------------+
        |               |          |              |
        |               +----+-----+              |
        |                    |                    |
        |                    | held > HOLD_THRESHOLD
        |                    v                    |
   +----+-----+        +----------+         +----+-----+
   |          |        |          |         |          |
   |   IDLE   |        |   HELD   |-------->| RELEASED |
   |          |        |          | release |          |
   +----+-----+        +----------+         +----+-----+
        ^                                        |
        |                                        |
        +---- after emit / timeout --------------+
```

### 5.2 Global Chord Detector (Layered on Top)

```
                    Per-button state machines
                    ________________________
                   |  Btn0  Btn1  ... Btn31  |
                   |  SM    SM        SM     |
                   |________________________|
                              |
                    Button events (down, up, held)
                              |
                              v
                   +---------------------+
                   |   Chord Detector    |  Buffers events, detects
                   |   (50ms window)     |  simultaneous combos
                   +---------------------+
                              |
                    Resolved events (tap, chord, long-press, etc.)
                              |
                              v
                   +---------------------+
                   |   Layer Manager     |  Routes event to the
                   |   (mode stack)      |  active layer's handler
                   +---------------------+
                              |
                              v
                   +---------------------+
                   |   Action Executor   |  Runs the bound action
                   |                     |  (shell cmd, keypress, etc.)
                   +---------------------+
                              |
                              v
                   +---------------------+
                   |   Display Manager   |  Updates button images
                   |                     |  based on new state
                   +---------------------+
```

### 5.3 Lessons from Fighting Game Input Buffers

Fighting games solve a harder version of this problem: detecting complex multi-directional sequences (quarter-circle forward + punch = fireball) within tight timing windows while buffering inputs across animation frames.

**Applicable patterns:**

1. **Input buffer with timestamps**: Store the last N button events with their timestamps. Pattern matchers scan the buffer for matching sequences. Buffer size of 16-32 events and a 500ms window covers all practical Stream Deck patterns.

2. **Priority resolution**: When multiple patterns match (a chord and a sequence could both trigger), use priority ordering. More specific patterns win (a 3-button chord beats a 2-button chord that is a subset).

3. **Rollback**: Fighting games famously "roll back" game state when input arrives late. For Stream Deck, if a chord is detected after a single press was already emitted, the action can be cancelled if it has not yet completed. This is feasible for actions with confirmation steps.

### 5.4 Lessons from QMK/ZMK Firmware

QMK and ZMK have spent years refining input handling for keyboards. Their relevant patterns:

**Tap-Hold decision (QMK `TAPPING_TERM`):**
- Default: 200ms. If held longer, it is a hold. If released sooner, it is a tap.
- `PERMISSIVE_HOLD`: If another key is pressed while holding, immediately decide "hold" regardless of elapsed time. Good for modifier keys.
- `HOLD_ON_OTHER_KEY_PRESS`: Even more aggressive -- any other key press during the hold immediately commits to "hold."

**Layer access patterns (QMK):**
- `MO(layer)`: Momentary. Layer is active while key is held.
- `TG(layer)`: Toggle. Press to activate, press again to deactivate.
- `TO(layer)`: Go to layer. Deactivates all other layers and activates this one.
- `LT(layer, kc)`: Layer-tap. Hold for layer, tap for keycode. The dual-purpose key.
- `OSL(layer)`: One-shot layer. Next keypress is in the layer, then auto-return.

All of these map directly to Stream Deck mode patterns.

---

## 6. Spatial Layout Patterns

### 6.1 The "Control Strip + Action Grid" Layout

Reserve the top row for mode selection and persistent controls. The remaining 24 buttons (3 rows x 8 columns) are the action area that changes per mode.

```
   CONTROL STRIP (persistent across all modes):
  +--------+--------+--------+--------+--------+--------+--------+--------+
  | Mode 1 | Mode 2 | Mode 3 | Mode 4 |        | Vol Dn | Vol Up |  Mute  |
  | active |        |        |        |        |   -    |   +    |  Mic   |
  +--------+--------+--------+--------+--------+--------+--------+--------+

   ACTION GRID (changes based on active mode):
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
```

This is the aircraft MFD pattern: bezel keys around the perimeter of the screen, with the display content (here, button images) changing based on the active page/mode.

### 6.2 The "Modifier Column" Layout

Reserve the left column as modifier keys (like Shift, Ctrl, Alt on a keyboard). The remaining 28 buttons (4 rows x 7 columns) are actions. Holding a modifier changes what the action buttons do.

```
   MODIFIERS    ACTION AREA
  +--------+   +--------+--------+--------+--------+--------+--------+--------+
  | SHIFT  |   |  Fn 1  |  Fn 2  |  Fn 3  |  Fn 4  |  Fn 5  |  Fn 6  |  Fn 7  |
  | (hold) |   |        |        |        |        |        |        |        |
  +--------+   +--------+--------+--------+--------+--------+--------+--------+
  | CTRL   |   |  Fn 8  |  Fn 9  | Fn 10  | Fn 11  | Fn 12  | Fn 13  | Fn 14  |
  | (hold) |   |        |        |        |        |        |        |        |
  +--------+   +--------+--------+--------+--------+--------+--------+--------+
  | ALT    |   | Fn 15  | Fn 16  | Fn 17  | Fn 18  | Fn 19  | Fn 20  | Fn 21  |
  | (hold) |   |        |        |        |        |        |        |        |
  +--------+   +--------+--------+--------+--------+--------+--------+--------+
  | META   |   | Fn 22  | Fn 23  | Fn 24  | Fn 25  | Fn 26  | Fn 27  | Fn 28  |
  | (hold) |   |        |        |        |        |        |        |        |
  +--------+   +--------+--------+--------+--------+--------+--------+--------+

  4 modifiers x 28 actions = 112 effective bindings (plus 28 unmodified)
  Total: 140 actions from 32 buttons
```

When SHIFT is held, all 28 action button images update instantly to show the shifted variants. This is the "laptop Fn key" pattern but with visual feedback.

### 6.3 The "Quadrant" Layout

Divide the 8x4 grid into four quadrants, each dedicated to a domain:

```
   QUADRANT 1: MEDIA/AUDIO        QUADRANT 2: COMMUNICATIONS
  +--------+--------+--------+---+--------+--------+--------+--------+
  | Prev   | Play   | Next   | | | Slack  | Email  | Zoom   | Teams  |
  +--------+--------+--------+ | +--------+--------+--------+--------+
  | Vol-   | Vol+   | Mute   | | | DND    | Status | React  | Reply  |
  +--------+--------+--------+---+--------+--------+--------+--------+

   QUADRANT 3: DEV TOOLS          QUADRANT 4: SYSTEM
  +--------+--------+--------+---+--------+--------+--------+--------+
  | Build  | Test   | Lint   | | | Lock   | Sleep  | Brt-   | Brt+   |
  +--------+--------+--------+ | +--------+--------+--------+--------+
  | Git    | Deploy | Logs   | | | Scrn1  | Scrn2  | Scrn3  |Capture |
  +--------+--------+--------+---+--------+--------+--------+--------+
```

No mode switching needed for the most common actions. Each quadrant can independently support sub-modes (press "Git" to replace the dev quadrant with git actions while other quadrants remain unchanged).

### 6.4 The "Numpad + Context" Layout

Use the right side as a numpad (for data entry, IP addresses, port numbers) while the left side provides context:

```
  +--------+--------+--------+--------++--------+--------+--------+--------+
  | Target |  SSH   |  HTTP  |  HTTPS ||   7    |   8    |   9    |  Bksp  |
  +--------+--------+--------+--------++--------+--------+--------+--------+
  | prod   |  stg   |  dev   |  local ||   4    |   5    |   6    |   .    |
  +--------+--------+--------+--------++--------+--------+--------+--------+
  | web-01 | web-02 | api-01 | api-02 ||   1    |   2    |   3    |   :    |
  +--------+--------+--------+--------++--------+--------+--------+--------+
  |  ping  | trace  |  curl  |  logs  ||   0    |  00    | Enter  |  Clr   |
  +--------+--------+--------+--------++--------+--------+--------+--------+

  Press [prod] + [web-01] + [SSH] -> ssh user@prod-web-01.example.com
  Or: press [curl] then numpad 8080 then [Enter] -> curl localhost:8080
```

### 6.5 The "Ableton-Style Page Grid"

Treat the entire 32-button surface as a paged workspace. A small navigation area switches between pages:

```
  PAGE INDICATOR (bottom-right corner):
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  |                    28-button action area                              |
  |                    (changes per page)                                 |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |  Pg 1  |  Pg 2  |  Pg 3  |
  |        |        |        |        |        | active |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+

  29 actions per page x 8 pages = 232 actions
  (using 3 bottom-right buttons for page navigation)
```

---

## 7. Dynamic Labeling Design

### 7.1 The 96x96 Pixel Canvas

Each Stream Deck XL key has a 96x96 pixel LCD. At normal viewing distance (~50cm), this is roughly equivalent to a 1cm x 1cm physical area. Design constraints:

- **Text**: Maximum ~8 characters at a readable size (12px font). Two lines of text are comfortable. Three lines are possible but tight.
- **Icons**: A centered 48x48 or 64x64 icon with a short label below works well.
- **Color**: Full RGB. Use color as the primary differentiator between modes, not text.

### 7.2 Visual Hierarchy for Modal Interfaces

When modes change, all button images update. The user needs to instantly parse "what mode am I in?" and "what does each button do now?" Design rules:

**Rule 1: Mode color = background color.**

```
  Base mode:     Dark gray (#2D2D2D) background
  Dev mode:      Deep blue (#1A237E) background
  Git mode:      Orange (#E65100) background
  Media mode:    Green (#1B5E20) background
  Danger mode:   Red (#B71C1C) background
```

A full-deck color change is the most salient visual signal possible. The user can tell which mode they are in from peripheral vision.

**Rule 2: Icon first, label second.**

```
  +------------------+
  |                  |
  |     [ICON]       |    <- 48x48 centered icon
  |                  |
  |    git push      |    <- 10px text label below
  +------------------+
```

**Rule 3: State-dependent appearance.**

```
  IDLE button:          ACTIVE/PRESSED button:     DISABLED button:
  +----------------+   +----------------+          +----------------+
  | Dark bg        |   | Bright bg      |          | Very dark bg   |
  |   [icon]       |   |   [icon]       |          |   [dim icon]   |
  |   label        |   |   label        |          |   dim label    |
  +----------------+   +----------------+          +----------------+
```

**Rule 4: Modifier state shown on every button.**

When a modifier (SHIFT, etc.) is held, every button image should update to show its modified action. Do not force users to memorize what "SHIFT + Build" does -- show it.

**Rule 5: Breadcrumb on the back button.**

```
  +----------------+
  |    < BACK      |
  |                |
  |   to: DEV      |    <- shows where "back" goes
  +----------------+
```

### 7.3 Animation and Transitions

The Stream Deck can update button images fast enough for simple animations (10-15 fps per button is achievable). Uses:

- **Mode transition**: Brief flash or color sweep when switching modes, confirming the switch happened.
- **Loading indicator**: Spinner or progress bar on a button whose action is in progress (e.g., "deploying...").
- **Attention pulse**: Gentle brightness pulse on a button that needs attention (unread messages, failed build).
- **Countdown**: Numeric countdown on a button before an auto-action (e.g., screen lock in 10... 9... 8...).

Caution: Constant animation is distracting. Use it sparingly and purposefully.

---

## 8. Analogies from Other Domains

### 8.1 Vim Modal Editing

Vim's design philosophy is directly applicable: most time is spent navigating and executing commands, not typing. The Stream Deck, similarly, is about executing pre-defined actions efficiently.

| Vim Concept          | Stream Deck Equivalent                                    |
|----------------------|----------------------------------------------------------|
| Normal mode          | Base layer -- most buttons are actions                    |
| Insert mode          | Text-input layer -- buttons type characters or fill forms |
| Visual mode          | Selection layer -- buttons select targets before action   |
| Command mode         | Leader-key sequence -- press leader, then command         |
| `.` (repeat)         | A button that repeats the last-executed action            |
| `u` (undo)           | A button that undoes the last action                      |
| Counts (`5dd`)       | Numpad entry before an action (delete 5 lines)           |
| Registers (`"ay`)    | Clipboard/paste-buffer selector                           |

**The "verb + noun" pattern from Vim is powerful:**

```
  Step 1: Press an ACTION verb button (e.g., [DEPLOY])
  Step 2: All other buttons update to show valid TARGETS (e.g., [staging], [prod], [canary])
  Step 3: Press a TARGET -> action executes (deploy to staging)

  This is the one-shot-layer pattern with semantic naming.
```

### 8.2 Aircraft MFD Bezel Keys

Military and commercial aircraft MFDs have been refining the "labeled buttons around a screen" pattern since the 1980s. Key lessons:

1. **OSB (Option Select Button) alignment**: Each bezel button must be unambiguously aligned with its on-screen label. On the Stream Deck, the label IS on the button, which is even better -- no alignment ambiguity.

2. **Page hierarchy**: MFDs use a hierarchical page structure (top-level -> sub-page -> detail). Each page has a "return" button in a consistent position. This maps exactly to the mode stack pattern.

3. **Fixed-function keys**: Some bezel positions always do the same thing regardless of page (brightness, contrast, power). The Stream Deck equivalent: certain button positions (e.g., top-right corner) always mean the same thing across all modes.

4. **Feedback latency**: MFDs require that button presses produce visible feedback within 100ms. The Stream Deck should follow the same rule -- update the pressed button's image immediately on press, even if the action takes longer to complete.

5. **Minimal head-down time**: Pilots need to glance at the MFD, press a button, and return eyes to the outside. The Stream Deck user similarly should not need to study the buttons -- mode colors and consistent spatial layout enable muscle memory.

### 8.3 Ableton Push / Novation Launchpad

These MIDI controllers have the closest interaction model to what we are designing:

1. **Momentary mode switching**: Hold a mode button to temporarily view that mode's pad layout. Release to return. The Stream Deck should support this.

2. **Color-coded grid regions**: Launchpad uses color to show which pads are in-scale (blue), root notes (purple), and out-of-scale (unlit). The Stream Deck can use color similarly to group related buttons.

3. **Session vs Note vs Custom**: Launchpad has distinct "personality" modes where the entire grid changes meaning. Session mode = clip launcher. Note mode = instrument. Custom mode = user-defined. The Stream Deck should support similar wholesale mode changes.

4. **Scene/page scrolling**: Launchpad lets you scroll through 8-track x N-scene grids using arrow buttons. The Stream Deck could implement similar pagination for large action sets.

### 8.4 QMK/ZMK Layer System

The keyboard firmware community has solved many of these problems already:

1. **Layer precedence**: Layers are checked top-down. The first non-transparent key found is used. This is the right model for Stream Deck mode stacks.

2. **Tri-layer**: In QMK, holding two different layer keys simultaneously activates a third layer (e.g., hold Lower + Raise = Adjust). This is a chord that activates a mode.

3. **Layer indicators**: QMK keyboards with RGB LEDs change LED color based on active layer. The Stream Deck has per-key LCDs -- far more expressive. Each layer should define a complete set of 32 button images.

4. **Sticky keys / one-shot**: QMK one-shot mods activate for exactly the next keypress. Perfect for "modifier + action" patterns on Stream Deck.

5. **Tap dance**: In QMK, a single key can do different things based on how many times it is tapped (1 tap = a, 2 taps = b, 3 taps = c). With Stream Deck's LCD feedback, you could show a countdown or menu of options as taps accumulate.

### 8.5 Bloomberg Terminal Keyboard

The Bloomberg Starboard keyboard uses color-coded physical keys:

- **Green keys**: Action/execute (GO, MENU)
- **Yellow keys**: Market sector navigation (GOVT, CORP, MTGE, EQUITY, etc.)
- **Red keys**: Cancel/stop

This color language is immediately learnable. The Stream Deck should adopt a similar scheme:

```
  Suggested color language:
    Green   = Execute / confirm / go
    Red     = Stop / cancel / danger
    Blue    = Navigation / mode switch
    Yellow  = Caution / requires attention
    Gray    = Neutral / standard action
    Purple  = System / settings
```

---

## 9. Proposed Interaction Architecture

Bringing all the patterns together into a concrete architecture for a Stream Deck XL control application.

### 9.1 Core Components

```
+-------------------------------------------------------------------+
|                        USER'S STREAM DECK XL                       |
|                                                                    |
|   HID Input Reports (button state)                                 |
|   HID Output Reports (button images)                               |
+----------------------------+--------------------------------------+
                             |
                             | USB HID
                             v
+-------------------------------------------------------------------+
|                      INPUT PIPELINE                                |
|                                                                    |
|  1. Raw HID Reader                                                 |
|     - Polls device at ~50ms intervals                              |
|     - Emits raw ButtonState(index, pressed) events                 |
|                                                                    |
|  2. Gesture Recognizer (per-button state machine)                  |
|     - Classifies: tap, double-tap, long-press, hold-repeat         |
|     - Configurable timing per button                               |
|                                                                    |
|  3. Chord Detector                                                 |
|     - 50ms window for simultaneous chord detection                 |
|     - Sequential chord detection with leader-key gating            |
|     - Priority resolution (most specific match wins)               |
|                                                                    |
|  4. Layer Router                                                   |
|     - Maintains mode stack                                         |
|     - Routes resolved gestures to active layer's key bindings      |
|     - Handles momentary/toggle/one-shot layer activation           |
|                                                                    |
+----------------------------+--------------------------------------+
                             |
                             | Resolved Action Events
                             v
+-------------------------------------------------------------------+
|                      ACTION SYSTEM                                 |
|                                                                    |
|  Action types:                                                     |
|    - ShellCommand(cmd: String)                                     |
|    - Keystroke(modifiers: [], key: String)                          |
|    - OpenApp(bundle_id: String)                                    |
|    - OpenURL(url: String)                                          |
|    - LayerPush(layer_id: String)                                   |
|    - LayerPop                                                      |
|    - LayerToggle(layer_id: String)                                 |
|    - LayerMomentary(layer_id: String)  -- on key down/up           |
|    - Sequence(actions: [Action])       -- run multiple in order     |
|    - Conditional(check, then, else)    -- branch on state          |
|                                                                    |
+----------------------------+--------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|                      DISPLAY MANAGER                               |
|                                                                    |
|  - Maintains a 32-slot image buffer (current displayed state)      |
|  - On layer change: batch-updates all 32 button images             |
|  - On action feedback: updates individual button images            |
|  - Image generation:                                               |
|      - Pre-rendered JPEG cache per layer per button                |
|      - Dynamic rendering for state-dependent buttons               |
|        (volume level, build status, unread count, clock)           |
|  - Transition animations (optional)                                |
|                                                                    |
+-------------------------------------------------------------------+
```

### 9.2 Configuration Format

A declarative configuration that defines layers, key bindings, and images:

```yaml
# Example: streamdeck-config.yaml

settings:
  chord_window_ms: 50
  tap_threshold_ms: 200
  double_tap_window_ms: 300
  long_press_threshold_ms: 500
  hold_repeat_interval_ms: 100

layers:
  - id: base
    name: "Base"
    color: "#2D2D2D"
    keys:
      0:
        image: "icons/dev-mode.png"
        label: "DEV"
        tap: { action: "layer_toggle", layer: "dev" }
        hold: { action: "layer_momentary", layer: "dev" }
      1:
        image: "icons/media-mode.png"
        label: "MEDIA"
        tap: { action: "layer_toggle", layer: "media" }
      6:
        image: "icons/vol-up.png"
        label: "Vol+"
        tap: { action: "keystroke", key: "VolumeUp" }
        hold: { action: "keystroke", key: "VolumeUp", repeat: true }
      7:
        image: "icons/mic-mute.png"
        label: "Mute"
        tap: { action: "keystroke", key: "F13" }
        double_tap: { action: "shell", cmd: "toggle-dnd" }

  - id: dev
    name: "Dev Tools"
    color: "#1A237E"
    parent: base
    keys:
      0:
        image: "icons/back.png"
        label: "< BACK"
        sublabel: "to: Base"
        tap: { action: "layer_pop" }
      1:
        image: "icons/git.png"
        label: "Git"
        tap: { action: "layer_push", layer: "git" }
      16:
        image: "icons/build.png"
        label: "Build"
        tap: { action: "shell", cmd: "pnpm ai" }
        long_press: { action: "shell", cmd: "pnpm ai --clean" }

chords:
  - keys: [6, 7]          # Vol+ and Mute pressed together
    action: { action: "shell", cmd: "toggle-audio-output" }
  - keys: [24, 25]        # Bottom-left two buttons
    action: { action: "layer_push", layer: "emergency" }

sequences:
  - leader: 0             # Key 0 is the leader key
    timeout_ms: 500
    bindings:
      - sequence: [1, 2]  # Leader -> Key 1 -> Key 2
        action: { action: "shell", cmd: "git stash && git pull && git stash pop" }
```

### 9.3 Layer Transition Logic

```
fn handle_key_event(event: KeyEvent, state: &mut AppState):
    // 1. Run through gesture recognizer
    let gesture = state.gesture_recognizers[event.key_index].process(event)
    if gesture.is_none(): return  // still accumulating (e.g., waiting for chord window)

    // 2. Check chord detector
    let resolved = state.chord_detector.process(gesture)
    if resolved.is_none(): return  // still in chord window

    // 3. Route through layer stack
    let active_layer = state.layer_stack.top()
    let binding = active_layer.get_binding(resolved.key, resolved.gesture_type)

    // 4. Fall through transparent keys
    if binding.is_none():
        for layer in state.layer_stack.iter_from_top():
            binding = layer.get_binding(resolved.key, resolved.gesture_type)
            if binding.is_some(): break

    // 5. Execute action
    if let Some(action) = binding:
        match action:
            LayerPush(id)      => state.layer_stack.push(id); refresh_all_images()
            LayerPop           => state.layer_stack.pop(); refresh_all_images()
            LayerToggle(id)    => if active == id: pop() else: push(id); refresh_all_images()
            LayerMomentary(id) => if event.is_down: push(id) else: pop(); refresh_all_images()
            other              => execute_action(other)
```

---

## 10. Creative / Speculative Ideas

These are ideas that push beyond conventional Stream Deck usage. Some are practical; some are provocative thought experiments.

### 10.1 The "Vim Deck" -- A Physical Vim Mode Indicator

```
  NORMAL MODE (green tint):
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  ESC   |  :w    |  :q    |  :wq   | :qa!   |  u     | Ctrl-R |  .     |
  |        | save   | quit   |save+q  |force q | undo   | redo   | repeat |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  dd    |  yy    |  p     |  P     |  >>    |  <<    |  J     |  ~     |
  |del line|cp line |paste a |paste b |indent  |dedent  | join   |  case  |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  /     |  n     |  N     |  *     | %      |  gd    |  gr    |  K     |
  | search | next   | prev   | word*  | match  |go def  |go ref  | hover  |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  v     |  V     | Ctrl-V |  o     |  O     |  i     |  a     |  A     |
  |visual  |V-line  |V-block |open bl | open ab|insert  |append  |apnd EOL|
  +--------+--------+--------+--------+--------+--------+--------+--------+

  Pressing [i] switches to INSERT MODE (blue tint):
  - All buttons update to show insert-mode-relevant shortcuts
  - ESC button pulses gently to remind user how to exit

  Pressing [v] switches to VISUAL MODE (purple tint):
  - Buttons show visual-mode operations (d, y, >, <, u, U, etc.)
```

This is not about replacing Vim keybindings -- it is about providing a physical dashboard that shows the current Vim mode and available commands, reducing the learning curve and serving as a persistent cheat sheet.

### 10.2 Context-Aware Layers (Automatic Mode Switching)

The host software monitors the active macOS application and automatically switches layers:

```
  Active app: VS Code    -> Dev layer auto-activates
  Active app: Chrome     -> Browser layer (bookmarks, tab management)
  Active app: Slack      -> Comms layer (channel switching, reactions, status)
  Active app: Terminal   -> Terminal layer (common commands, SSH targets)
  Active app: Zoom       -> Meeting layer (mute, camera, screen share, raise hand)
  Active app: Figma      -> Design layer (zoom, export, component shortcuts)
```

Combined with manual mode switching: the auto-selected base changes, but the user can still manually push sub-layers on top.

### 10.3 The "Dashboard Deck" -- Live Status Display

Use some buttons purely as status indicators, not as pressable actions:

```
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  CPU   |  MEM   |  DISK  |  NET   | CI/CD  |  PROD  | ALERTS |  TIME  |
  | 23%    | 8.2/16 | 72%    | 12Mbps | green  |  OK    |   0    | 14:32  |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  |    ... action buttons below ...                                       |
```

The top row continuously updates with system metrics. Pressing a status button could open a detail view (push a layer showing more detailed info about that metric).

### 10.4 The "Confirmation Deck" -- Dangerous Actions

For destructive actions, use a two-stage confirmation pattern:

```
  Step 1: Press [Deploy Prod]

  All buttons turn RED, showing:
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  | CANCEL | CANCEL | CANCEL |CONFIRM | CANCEL | CANCEL | CANCEL | CANCEL |
  |        |        |        |deploy  |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL | CANCEL |
  +--------+--------+--------+--------+--------+--------+--------+--------+

  The CONFIRM button is placed at a RANDOM position each time.
  Auto-cancels after 5 seconds if no button is pressed.
```

The random placement of the confirm button prevents muscle memory from accidentally confirming. This is genuinely useful for "never accidentally do this" actions like production deployments or database drops.

### 10.5 The "Macro Recorder" Button

One button enters "record mode":

```
  Press [REC] -> button turns red, starts recording
  Press [action1], [action2], [action3] -> each action is recorded
  Press [REC] again -> recording stops
  Prompt: "Assign macro to which button?" -> press target button
  Now that button replays the sequence action1, action2, action3
```

Like Vim's `q` (macro record), this lets users create custom sequences without editing configuration files.

### 10.6 The "Teaching Deck" -- Progressive Disclosure

For new users, start with only a few buttons lit. As the user becomes comfortable, gradually reveal more:

```
  Week 1 (beginner):
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  Play  |  Mute  |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  (unlit buttons are ignored)

  Week 4 (intermediate):
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |  DEV   | MEDIA  | COMMS  |        |        | Vol-   | Vol+   |  Mute  |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  | App 1  | App 2  | App 3  | App 4  |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  |        |        |        |        |        |        |        |        |
  +--------+--------+--------+--------+--------+--------+--------+--------+
  | Prev   | Play   | Next   |        |        |        |        | Lock   |
  +--------+--------+--------+--------+--------+--------+--------+--------+
```

### 10.7 Spatial Encoding: Position Conveys Meaning

A consistent spatial grammar that holds across all layers:

```
  TOP ROW:     Always navigation / mode selection
  LEFT SIDE:   Primary / most-used actions
  RIGHT SIDE:  Secondary / settings / system
  BOTTOM ROW:  Persistent controls (media, lock, brightness)
  CENTER:      Context-specific actions (change per mode)

  TOP-LEFT:    Always "BACK" or "HOME" (consistent escape hatch)
  TOP-RIGHT:   Always "MUTE" or "DND" (consistent safety)
  BOTTOM-LEFT: Always "PREV" (consistent media control)
  BOTTOM-RIGHT:Always "LOCK" (consistent security)
```

If these positions are sacred across all modes, users build muscle memory for the most critical actions regardless of which mode they are in.

---

## 11. Implementation Priorities

For a first implementation, ordered by effort-to-value ratio:

### Tier 1: Build First (High Value, Moderate Effort)

1. **Toggle mode switching** with full image updates. This is the single highest-leverage feature. Even with just 4 modes (base, dev, media, comms), you go from 32 actions to 128 actions.

2. **Top-row control strip** layout. Reserve row 0 for mode selectors and persistent controls. The remaining 24 buttons are the action area.

3. **Mode-colored backgrounds** on all button images. Instant visual feedback for mode state.

4. **Tap detection** (fire on release). The basic gesture that everything else builds on.

### Tier 2: Build Next (High Value, Higher Effort)

5. **Momentary mode** (hold to activate). Enables quick one-off actions in another mode without fully switching.

6. **Long-press detection**. One additional gesture per button with minimal UI complexity.

7. **Mode stack** with back/pop navigation. Enables the nested DEV -> GIT -> (back) -> DEV -> (back) -> BASE flow.

8. **Context-aware auto-switching** based on active macOS application.

### Tier 3: Build Later (Moderate Value, High Effort)

9. **Chord detection** (simultaneous two-button combos). Useful but requires careful timing calibration and user training.

10. **Double-tap detection**. Adds latency to single taps; only worth it for specific buttons.

11. **Sequential chords** with leader key. Powerful for experts, confusing for beginners.

12. **Hold-repeat** for continuous actions. Nice for volume but not essential.

### Tier 4: Explore (Speculative)

13. **Dashboard / live status buttons**. Requires background data polling infrastructure.

14. **Confirmation deck** for dangerous actions. Fun, but niche.

15. **Macro recorder**. Complex to implement well; configuration files may be sufficient.

16. **Swipe detection**. Novel but possibly too unreliable with physical buttons.

---

## 12. Summary: Interaction Pattern Reference

| Pattern             | Activation             | Latency Impact     | Best For                        |
|---------------------|------------------------|--------------------|---------------------------------|
| Tap                 | Press + release < 200ms| +200ms (fire on up)| Standard actions                |
| Long press          | Hold > 500ms           | None (fire on threshold) | Alternate actions, settings |
| Double tap          | Two taps < 300ms apart | +300ms on single taps | Confirmations, toggles      |
| Hold-repeat         | Hold > 500ms, repeat   | None               | Volume, brightness, scrolling   |
| Simultaneous chord  | Two keys < 50ms apart  | +50ms on all presses | Power-user shortcuts          |
| Sequential chord    | Key A then key B < 300ms| +300ms on first key | Command sequences (with leader)|
| Toggle mode         | Tap a mode button      | None               | Entering a work context         |
| Momentary mode      | Hold a mode button     | None               | Quick look at another layer     |
| One-shot mode       | Tap, then next key     | None               | Modifier + action               |
| Mode stack push/pop | Tap into sub-mode      | None               | Hierarchical navigation         |

---

## Sources

1. [QMK Firmware -- Combos](https://docs.qmk.fm/features/combo)
2. [QMK Firmware -- Tap-Hold Configuration](https://docs.qmk.fm/tap_hold)
3. [QMK Firmware -- Layers](https://docs.qmk.fm/feature_layers)
4. [ZMK Firmware -- Hold-Tap Behavior](https://zmk.dev/docs/keymaps/behaviors/hold-tap)
5. [Elgato Stream Deck -- Key Logic](https://www.elgato.com/us/en/explorer/products/stream-deck/key-logic-stream-deck/)
6. [Elgato Stream Deck -- Multi Actions](https://help.elgato.com/hc/en-us/articles/360027960912-Elgato-Stream-Deck-Multi-Actions)
7. [Elgato Stream Deck HID API -- General Reference](https://docs.elgato.com/streamdeck/hid/general/)
8. [Elgato Stream Deck HID API -- Module 15 and 32 Keys](https://docs.elgato.com/streamdeck/hid/module-15_32/)
9. [Notes on the Stream Deck HID Protocol (Cliff Rowley)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0)
10. [Reverse Engineering the Stream Deck Plus (Den Delimarsky)](https://den.dev/blog/reverse-engineer-stream-deck-plus/)
11. [Novation Launchpad X -- Session Mode User Guide](https://userguides.novationmusic.com/hc/en-gb/articles/23731420256018-Using-Launchpad-X-s-Session-mode)
12. [Novation Launchpad X -- Note Mode User Guide](https://userguides.novationmusic.com/hc/en-gb/articles/23731440690706-Using-Launchpad-X-s-Note-mode)
13. [Multi-function Display -- Wikipedia](https://en.wikipedia.org/wiki/Multi-function_display)
14. [Multi-Function Displays: A Guide for Human Factors Evaluation (DTIC)](https://apps.dtic.mil/sti/tr/pdf/ADA601926.pdf)
15. [SoundFlow -- Modifier Key Implementation for Stream Deck](https://forum.soundflow.org/-10818/deck-modifier-key-implementation)
16. [Fighting Game Input Manager -- GameDev.net](https://www.gamedev.net/forums/topic/335496-fighting-game-input-manager/)
17. [How to Code Fighting Game Motion Inputs -- CritPoints](https://critpoints.net/2025/02/05/how-to-code-fighting-game-motion-inputs/)
18. [Bloomberg Keyboard 4 (Starboard) Guide (PDF)](https://data.bloomberglp.com/professional/sites/20/bloomberg_keyboard_4_guide.pdf)
19. [Vim Philosophy -- MIT Missing Semester](https://missing.csail.mit.edu/2020/editors/)
20. [urob/zmk-config -- Advanced ZMK Configuration](https://github.com/urob/zmk-config)
