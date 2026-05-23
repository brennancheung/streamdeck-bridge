# Keyboard and Mouse Simulation

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > macOS System Automation

---

## Key Findings

- **nut.js is the recommended high-level library** for Node.js keyboard/mouse simulation. It provides `keyboard.type(Key.LeftSuper, Key.Space)` ergonomics, is cross-platform, and actively maintained. However, prebuilt binaries require a paid subscription; the free fork `@nut-tree-fork/nut-js` is an alternative.
- **robotjs is simpler but effectively unmaintained.** The `@jitsi/robotjs` fork provides prebuilt binaries and tracks upstream. Media keys do not work on macOS.
- **AppleScript via `osascript` is the zero-dependency escape hatch.** Volume, mute, keystroke simulation, and app activation all work through `child_process.exec`. No native compilation needed.
- **Accessibility permission is mandatory.** Any app that posts synthetic keyboard or mouse events must be granted Accessibility access in System Settings > Privacy & Security > Accessibility. There is no way to programmatically grant this; you can only prompt the user to open the settings pane.
- **Secure Input blocks all synthetic input** in password fields. When an app enables Secure Event Input (e.g., password dialogs, 1Password), all event taps and CGEvent posting are blocked. This is by design and cannot be bypassed.
- **Global hotkey listening** (reacting to key combos when your app is not focused) requires either Electron's `globalShortcut` module or the `node-global-key-listener` package (archived but functional).

---

## 1. Approaches Overview

There are four tiers of keyboard/mouse simulation on macOS, from highest to lowest level:

| Approach | Ergonomics | Dependencies | Media Keys | Permissions |
|----------|-----------|--------------|------------|-------------|
| nut.js | Best | Native addon (paid binaries) | No | Accessibility |
| robotjs / @jitsi/robotjs | Good | Native addon (prebuilt) | No on macOS | Accessibility |
| AppleScript (osascript) | Moderate | None (built into macOS) | Volume/mute only | Accessibility |
| CGEvent (native addon) | Low | Custom C/Swift addon | Yes (via IOKit) | Accessibility |

---

## 2. nut.js -- The Recommended Library

nut.js (`@nut-tree/nut-js`) is the most ergonomic Node.js library for desktop automation. It provides async/await APIs for keyboard, mouse, and screen control.

### Installation

```bash
# Official (requires paid subscription for prebuilt binaries)
pnpm add @nut-tree/nut-js

# Community fork (free, v4.2.6)
pnpm add @nut-tree-fork/nut-js
```

### Keyboard Simulation

```typescript
import { keyboard, Key } from "@nut-tree/nut-js";

// Type a string
await keyboard.type("Hello, Stream Deck!");

// Press a keyboard shortcut (Cmd+C on macOS)
await keyboard.type(Key.LeftSuper, Key.C);

// Cmd+Shift+4 (screenshot region on macOS)
await keyboard.type(Key.LeftSuper, Key.LeftShift, Key.Num4);

// Cmd+Tab (app switcher)
await keyboard.type(Key.LeftSuper, Key.Tab);

// Cmd+Space (Spotlight)
await keyboard.type(Key.LeftSuper, Key.Space);

// For sequences that need hold-and-release (e.g., Cmd+Tab cycling)
await keyboard.pressKey(Key.LeftSuper);
await keyboard.pressKey(Key.Tab);
await keyboard.releaseKey(Key.Tab);
// Tab again to move to next app
await keyboard.pressKey(Key.Tab);
await keyboard.releaseKey(Key.Tab);
await keyboard.releaseKey(Key.LeftSuper);

// Configure delay between keystrokes
keyboard.config.autoDelayMs = 50;
```

### Available Modifier Keys

```typescript
Key.LeftSuper   // Command (macOS) / Windows key
Key.RightSuper
Key.LeftShift   // Shift
Key.RightShift
Key.LeftControl // Control
Key.RightControl
Key.LeftAlt     // Option (macOS) / Alt
Key.RightAlt
```

### Mouse Simulation

```typescript
import { mouse, Button, Point, straightTo } from "@nut-tree/nut-js";

// Move mouse to coordinates
await mouse.move(straightTo(new Point(500, 300)));

// Click
await mouse.click(Button.LEFT);
await mouse.click(Button.RIGHT);
await mouse.doubleClick(Button.LEFT);

// Drag from current position to target
await mouse.drag(straightTo(new Point(800, 600)));

// Scroll
await mouse.scrollDown(5);
await mouse.scrollUp(3);
await mouse.scrollLeft(2);
await mouse.scrollRight(2);

// Get current position
const pos = await mouse.getPosition();
console.log(`Mouse at: ${pos.x}, ${pos.y}`);

// Configure mouse speed (pixels per second)
mouse.config.mouseSpeed = 1500;
mouse.config.autoDelayMs = 100;
```

### Critical Warning

Always release keys pressed with `pressKey()`. Unreleased keys remain "stuck" and affect all subsequent input across the system until released.

---

## 3. robotjs / @jitsi/robotjs

robotjs has a simpler synchronous API. The original project is unmaintained; use the `@jitsi/robotjs` fork which provides prebuilt binaries and tracks upstream.

### Installation

```bash
pnpm add @jitsi/robotjs
```

### Keyboard Simulation

```typescript
import robot from "@jitsi/robotjs";

// Type a string
robot.typeString("Hello, Stream Deck!");

// Type with a delay (characters per minute)
robot.typeStringDelayed("Slow typing", 300);

// Press a single key
robot.keyTap("enter");
robot.keyTap("escape");
robot.keyTap("tab");

// Keyboard shortcut: Cmd+C
robot.keyTap("c", "command");

// Multiple modifiers: Cmd+Shift+4
robot.keyTap("4", ["command", "shift"]);

// Cmd+Tab
robot.keyTap("tab", "command");

// Hold and release a key
robot.keyToggle("shift", "down");
robot.keyTap("a"); // types "A"
robot.keyToggle("shift", "up");

// Set delay between key events (default: 10ms)
robot.setKeyboardDelay(50);
```

### Supported Key Names

Letters `a-z`, numbers `0-9`, plus:
`backspace`, `delete`, `enter`, `tab`, `escape`, `up`, `down`, `left`, `right`,
`home`, `end`, `pageup`, `pagedown`, `f1`-`f12`, `command`, `alt`, `control`,
`shift`, `right_shift`, `space`, `printscreen`, `insert`,
`audio_mute`, `audio_vol_down`, `audio_vol_up`, `audio_play`, `audio_stop`,
`audio_pause`, `audio_prev`, `audio_next`, `numpad_0`-`numpad_9`

**macOS caveat:** Media keys (`audio_*`) do not work on macOS. This is a known, unresolved issue (#167).

### Mouse Simulation

```typescript
import robot from "@jitsi/robotjs";

// Move mouse instantly
robot.moveMouse(500, 300);

// Move mouse smoothly (human-like motion)
robot.moveMouseSmooth(500, 300);
robot.moveMouseSmooth(500, 300, 2.0); // custom speed

// Click
robot.mouseClick();              // left click
robot.mouseClick("right");       // right click
robot.mouseClick("left", true);  // double click

// Hold/release mouse button
robot.mouseToggle("down");
robot.moveMouse(800, 600);       // drag
robot.mouseToggle("up");

// Scroll
robot.scrollMouse(0, 5);   // scroll down
robot.scrollMouse(0, -5);  // scroll up

// Get mouse position
const pos = robot.getMousePos();
console.log(`Mouse at: ${pos.x}, ${pos.y}`);

// Set delay after mouse events
robot.setMouseDelay(100);
```

### Known Issues on macOS

- Modifier keys can remain "stuck" after `keyTap` or `keyToggle` calls (issue #219)
- Custom global hotkeys in other apps may not respond to robotjs-synthesized shortcuts on macOS
- Media keys throw errors or silently fail

---

## 4. AppleScript via osascript (Zero Dependencies)

AppleScript provides a zero-dependency path for keyboard simulation, volume/mute control, and app management. Call it from Node.js via `child_process`.

### Helper Function

```typescript
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

async function osascript(script: string): Promise<string> {
  const { stdout } = await execAsync(`osascript -e '${script}'`);
  return stdout.trim();
}
```

### Keyboard Shortcuts

```typescript
// Cmd+C (copy)
await osascript(
  'tell application "System Events" to keystroke "c" using command down'
);

// Cmd+Shift+4 (screenshot region)
await osascript(
  'tell application "System Events" to keystroke "4" using {command down, shift down}'
);

// Cmd+Tab (app switcher)
await osascript(
  'tell application "System Events" to key code 48 using command down'
);

// Press Enter
await osascript(
  'tell application "System Events" to key code 36'
);

// Press Escape
await osascript(
  'tell application "System Events" to key code 53'
);

// Press F11 (show desktop)
await osascript(
  'tell application "System Events" to key code 103'
);

// Type text
await osascript(
  'tell application "System Events" to keystroke "Hello World"'
);
```

### Common AppleScript Key Codes

```
Return: 36       Tab: 48          Space: 49
Escape: 53       Delete: 51       Forward Delete: 117
Up: 126          Down: 125        Left: 123        Right: 124
Home: 115        End: 119         Page Up: 116      Page Down: 121
F1: 122   F2: 120   F3: 99    F4: 118   F5: 96    F6: 97
F7: 98    F8: 100   F9: 101   F10: 109  F11: 103  F12: 111
```

### Volume and Mute Control

```typescript
// Set volume (0-100)
await osascript('set volume output volume 50');

// Get current volume
const vol = await osascript('output volume of (get volume settings)');

// Mute / unmute
await osascript('set volume output muted true');
await osascript('set volume output muted false');

// Check mute status
const muted = await osascript('output muted of (get volume settings)');

// Set volume to zero (different from mute -- resets the level)
await osascript('set volume output volume 0');
```

### App Management

```typescript
// Activate (focus) an application
await osascript('tell application "Safari" to activate');

// Launch an application
await osascript('tell application "Calculator" to launch');

// Quit an application
await osascript('tell application "TextEdit" to quit');

// Get name of frontmost application
const app = await osascript(
  'tell application "System Events" to get name of first application process whose frontmost is true'
);
```

### Limitations

- AppleScript keystroke simulation has a noticeable latency (~100-200ms per call) due to process spawning
- Complex sequences require multiple `osascript` invocations
- Still requires Accessibility permissions for keystroke/key code commands
- Cannot simulate media hardware keys (volume/brightness buttons), only the AppleScript volume commands

---

## 5. Media Keys and Hardware Keys

Simulating media keys (play/pause, brightness, volume hardware buttons) on macOS is more complex than regular keyboard shortcuts because they use the IOKit HID subsystem rather than CGEvent.

### What Works

| Method | Volume | Mute | Brightness | Play/Pause | Prev/Next |
|--------|--------|------|------------|------------|-----------|
| AppleScript `set volume` | Yes | Yes | No | No | No |
| robotjs `audio_*` keys | No (macOS) | No (macOS) | No | No | No |
| nut.js | No | No | No | No | No |
| IOKit NX_KEYTYPE (native) | Yes | Yes | Yes | Yes | Yes |
| osascript + Spotify/Music | N/A | N/A | N/A | Yes | Yes |

### App-Specific Media Control via AppleScript

```typescript
// Control Apple Music
await osascript('tell application "Music" to playpause');
await osascript('tell application "Music" to next track');
await osascript('tell application "Music" to previous track');

// Control Spotify
await osascript('tell application "Spotify" to playpause');
await osascript('tell application "Spotify" to next track');
await osascript('tell application "Spotify" to previous track');
```

### Brightness via Command Line

Brightness control is not available through standard AppleScript. Options:

```bash
# Using the brightness CLI tool (install separately)
brightness -l          # list current brightness
brightness 0.7         # set to 70%

# Or via AppleScript with System Events
osascript -e 'tell application "System Events" to key code 107'  # brightness down (F14)
osascript -e 'tell application "System Events" to key code 113'  # brightness up (F15)
```

### Native IOKit Approach (for a Custom Addon)

For true hardware media key simulation, a native Node.js addon would use IOKit:

```c
// Conceptual -- this is what a native addon would wrap
#include <IOKit/hidsystem/IOHIDLib.h>

void simulateMediaKey(int keyType, bool keyDown) {
    // keyType: NX_KEYTYPE_PLAY (16), NX_KEYTYPE_SOUND_UP (0), etc.
    NSEvent *event = [NSEvent otherEventWithType:NSEventTypeSystemDefined
                                        location:NSZeroPoint
                                   modifierFlags:(keyDown ? 0xa00 : 0xb00)
                                       timestamp:0
                                    windowNumber:0
                                         context:nil
                                         subtype:8
                                           data1:((keyType << 16) | ((keyDown ? 0xa : 0xb) << 8))
                                           data2:-1];
    CGEventPost(kCGHIDEventTap, [event CGEvent]);
}
```

NX_KEYTYPE constants:
```
NX_KEYTYPE_SOUND_UP          = 0    // Volume up
NX_KEYTYPE_SOUND_DOWN        = 1    // Volume down
NX_KEYTYPE_MUTE              = 7    // Mute toggle
NX_KEYTYPE_BRIGHTNESS_UP     = 2    // Screen brightness up
NX_KEYTYPE_BRIGHTNESS_DOWN   = 3    // Screen brightness down
NX_KEYTYPE_PLAY              = 16   // Play/pause
NX_KEYTYPE_NEXT              = 17   // Next track
NX_KEYTYPE_PREVIOUS          = 18   // Previous track
NX_KEYTYPE_FAST              = 19   // Fast forward
NX_KEYTYPE_REWIND            = 20   // Rewind
NX_KEYTYPE_ILLUMINATION_UP   = 21   // Keyboard backlight up
NX_KEYTYPE_ILLUMINATION_DOWN = 22   // Keyboard backlight down
```

---

## 6. Permissions and Security

### Accessibility Permission (Required)

Every approach that posts synthetic keyboard or mouse events requires the app to be granted Accessibility access.

**Checking permission programmatically:**

```typescript
// Using node-mac-permissions
import { getAuthStatus } from "node-mac-permissions";

const status = getAuthStatus("accessibility");
// Returns: "not determined" | "denied" | "authorized" | "restricted"
```

```bash
pnpm add node-mac-permissions
```

**Prompting the user:**

```typescript
import { askForAccessibilityAccess } from "node-mac-permissions";

// Opens System Settings > Accessibility pane
// There is NO way to programmatically grant access
askForAccessibilityAccess();
```

**What the user sees:** A system dialog stating the application wants to control the computer using Accessibility features. The user must authenticate as admin and toggle the app on in System Settings > Privacy & Security > Accessibility.

### Secure Event Input (Blocking Restriction)

When a macOS application enables Secure Event Input (activated automatically by NSSecureTextField for password fields), all event taps and CGEvent posting are blocked system-wide. This means:

- No keyboard simulation works in password dialogs
- No key logging is possible during secure input
- Tools like TextExpander, Keyboard Maestro, and 1Password auto-type all stop working
- This is intentional and cannot be bypassed

**Workaround:** Use the clipboard. Copy text to the clipboard and simulate Cmd+V. This works in some (but not all) secure input fields.

```typescript
import { exec } from "child_process";

function pasteText(text: string) {
  // Copy to clipboard
  exec(`echo "${text}" | pbcopy`);
  // Simulate Cmd+V
  exec(`osascript -e 'tell application "System Events" to keystroke "v" using command down'`);
}
```

### System Integrity Protection (SIP)

SIP does not directly block CGEvent posting or keyboard simulation for non-sandboxed apps. However:

- Apps distributed through the Mac App Store are sandboxed and CGEventPost is ignored inside the sandbox
- Non-sandboxed apps (like an Electron app distributed directly) work fine with Accessibility permission
- SIP protects system files but does not restrict synthetic input for permitted apps

### Input Monitoring Permission

Separate from Accessibility, macOS also has an Input Monitoring permission (System Settings > Privacy & Security > Input Monitoring). This is required for apps that want to observe keyboard and mouse events without necessarily controlling them. For a Stream Deck controller, you typically need Accessibility (for posting events) rather than Input Monitoring (for observing events).

---

## 7. Global Hotkey Registration

Global hotkeys let your app respond to key combinations even when it does not have focus. This is useful for Stream Deck scenarios where a physical button press should trigger an action regardless of which app is in the foreground.

### Electron globalShortcut

```typescript
import { app, globalShortcut } from "electron";

app.whenReady().then(() => {
  // Register a global shortcut
  globalShortcut.register("CommandOrControl+Shift+P", () => {
    console.log("Global shortcut triggered!");
    // Perform action
  });

  // Register multiple shortcuts
  globalShortcut.register("CommandOrControl+Shift+1", () => toggleMute());
  globalShortcut.register("CommandOrControl+Shift+2", () => switchScene());

  // Check if a shortcut is registered
  const isRegistered = globalShortcut.isRegistered("CommandOrControl+Shift+P");

  // Unregister when done
  // globalShortcut.unregister("CommandOrControl+Shift+P");
  // globalShortcut.unregisterAll();
});

app.on("will-quit", () => {
  globalShortcut.unregisterAll();
});
```

**Accelerator format:** `CommandOrControl+Shift+Alt+Key` where Key can be letters, numbers, F1-F24, Plus, Space, Tab, Backspace, Delete, Insert, Return, Up, Down, Left, Right, Home, End, PageUp, PageDown, Escape, VolumeUp, VolumeDown, VolumeMute, MediaNextTrack, MediaPreviousTrack, MediaStop, MediaPlayPause, PrintScreen.

### node-global-key-listener (Non-Electron)

For non-Electron Node.js apps:

```typescript
import { GlobalKeyboardListener } from "node-global-key-listener";

const listener = new GlobalKeyboardListener();

// Listen for all key events
listener.addListener((event, down) => {
  console.log(`${event.name} ${event.state}`);  // e.g., "A DOWN"

  // Check for Cmd+Shift+P
  if (
    event.state === "DOWN" &&
    event.name === "P" &&
    (down["LEFT META"] || down["RIGHT META"]) &&
    (down["LEFT SHIFT"] || down["RIGHT SHIFT"])
  ) {
    console.log("Custom hotkey triggered!");
    return true; // suppress the event from reaching other apps
  }
});
```

**Important notes:**
- The project is archived (July 2024) but still functional
- On macOS, requires Accessibility permission
- Uses a separate key server process communicating over stdio
- No node-gyp compilation required (pre-compiled binaries)
- Returning `true` from the listener suppresses the event on macOS

---

## 8. Text Input and Typing Simulation

### Fast Typing with nut.js

```typescript
import { keyboard } from "@nut-tree/nut-js";

// Type a string character by character
await keyboard.type("Hello, this is typed text!");

// Control typing speed
keyboard.config.autoDelayMs = 30; // 30ms between characters
await keyboard.type("Faster typing with short delay");
```

### Fast Typing with robotjs

```typescript
import robot from "@jitsi/robotjs";

// Instant typing
robot.typeString("Hello World");

// Controlled speed (characters per minute)
robot.typeStringDelayed("Slower typing", 500);
```

### Clipboard-Based Fast Paste (Fastest Method)

For long text blocks, simulating individual keystrokes is slow. The clipboard approach is much faster:

```typescript
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

async function typeViaPaste(text: string): Promise<void> {
  // Save current clipboard
  const { stdout: savedClipboard } = await execAsync("pbpaste");

  // Set new clipboard content
  await execAsync(`echo -n ${JSON.stringify(text)} | pbcopy`);

  // Paste it
  await execAsync(
    `osascript -e 'tell application "System Events" to keystroke "v" using command down'`
  );

  // Restore original clipboard after a short delay
  setTimeout(async () => {
    await execAsync(`echo -n ${JSON.stringify(savedClipboard)} | pbcopy`);
  }, 500);
}
```

---

## 9. Practical Patterns for Stream Deck

### High-Level Wrapper

A convenience wrapper that picks the best method for each action type:

```typescript
import { keyboard, mouse, Key, Button, Point, straightTo } from "@nut-tree/nut-js";
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);
const osascript = (s: string) => execAsync(`osascript -e '${s}'`);

// --- Keyboard Shortcuts ---

async function pressShortcut(...keys: Key[]): Promise<void> {
  await keyboard.type(...keys);
}

// Cmd+C
await pressShortcut(Key.LeftSuper, Key.C);

// Cmd+Shift+4
await pressShortcut(Key.LeftSuper, Key.LeftShift, Key.Num4);

// --- Volume Control ---

async function setVolume(level: number): Promise<void> {
  await osascript(`set volume output volume ${Math.round(level)}`);
}

async function toggleMute(): Promise<void> {
  const { stdout } = await execAsync(
    `osascript -e 'output muted of (get volume settings)'`
  );
  const isMuted = stdout.trim() === "true";
  await osascript(`set volume output muted ${!isMuted}`);
}

// --- Media Control ---

async function mediaPlayPause(app = "Music"): Promise<void> {
  await osascript(`tell application "${app}" to playpause`);
}

async function mediaNext(app = "Music"): Promise<void> {
  await osascript(`tell application "${app}" to next track`);
}

// --- App Management ---

async function focusApp(name: string): Promise<void> {
  await osascript(`tell application "${name}" to activate`);
}

async function launchApp(name: string): Promise<void> {
  await execAsync(`open -a "${name}"`);
}

// --- Mouse ---

async function clickAt(x: number, y: number): Promise<void> {
  await mouse.move(straightTo(new Point(x, y)));
  await mouse.click(Button.LEFT);
}
```

### Permission Check on Startup

```typescript
import { getAuthStatus, askForAccessibilityAccess } from "node-mac-permissions";

function ensureAccessibility(): boolean {
  const status = getAuthStatus("accessibility");
  if (status === "authorized") {
    return true;
  }
  console.log("Accessibility permission required. Opening System Settings...");
  askForAccessibilityAccess();
  return false;
}
```

---

## 10. Library Comparison Matrix

| Feature | nut.js | @jitsi/robotjs | AppleScript | Native CGEvent |
|---------|--------|---------------|-------------|----------------|
| Keyboard shortcuts | Yes | Yes | Yes | Yes |
| Type text | Yes | Yes | Yes | Yes (complex) |
| Mouse move | Yes | Yes | No | Yes |
| Mouse click | Yes | Yes | Yes (via key code) | Yes |
| Mouse scroll | Yes | Yes | No | Yes |
| Media keys (macOS) | No | No | Volume/mute only | Yes (IOKit) |
| Brightness | No | No | No | Yes (IOKit) |
| Global hotkey listen | No | No | No | No (use Electron) |
| Async API | Yes | No (sync) | Via exec | Via exec |
| Node.js version support | Modern (N-API) | Older (node-gyp) | Any | Custom build |
| Active maintenance | Yes | Jitsi fork only | N/A (OS built-in) | Custom |
| Installation complexity | Medium | Low | None | High |
| Latency | Low (<10ms) | Low (<10ms) | High (~100-200ms) | Lowest |

---

## Sources

1. [nut.js - Desktop Automation for Node.js](https://nutjs.dev/)
2. [nut.js Keyboard Input Documentation](https://nutjs.dev/docs/keyboard)
3. [nut.js Mouse Control Documentation](https://nutjs.dev/docs/mouse)
4. [RobotJS - GitHub](https://github.com/octalmage/robotjs)
5. [RobotJS API Syntax Documentation](https://github.com/octalmage/robotjs.dev/blob/master/_posts/docs/2016-10-12-syntax.md)
6. [@jitsi/robotjs - npm](https://www.npmjs.com/package/@jitsi/robotjs)
7. [RobotJS Media Keys macOS Issue #167](https://github.com/octalmage/robotjs/issues/167)
8. [node-global-key-listener - GitHub](https://github.com/LaunchMenu/node-global-key-listener)
9. [Electron globalShortcut API](https://www.electronjs.org/docs/latest/api/global-shortcut)
10. [node-mac-permissions - GitHub](https://github.com/codebytere/node-mac-permissions)
11. [Apple Developer - CGKeyCode](https://developer.apple.com/documentation/coregraphics/cgkeycode)
12. [Apple Developer Forums - Simulate Keyboard Events](https://developer.apple.com/forums/thread/28605)
13. [Apple Developer - CGEventPost Sandbox Restriction](https://developer.apple.com/forums/thread/103992)
14. [macOS TCC - HackTricks](https://hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc.html)
15. [Accessibility Permission on macOS](https://jano.dev/apple/macos/swift/2025/01/08/Accessibility-Permission.html)
16. [Implementing Auto-Type on macOS](https://blog.kulman.sk/implementing-auto-type-on-macos/)
17. [Complete List of AppleScript Key Codes](https://eastmanreference.com/complete-list-of-applescript-key-codes)
18. [macOS Key Codes Gist](https://gist.github.com/usagimaru/2c918779d68aa0899f281357cfec62db)
19. [Adjusting Mac Volume from Command Line](https://excessivelyadequate.com/posts/vol.html)
20. [TextExpander and Secure Input](https://textexpander.com/secure-input)
21. [Apple TN2150 - Using Secure Event Input](https://developer.apple.com/library/archive/technotes/tn2150/_index.html)
22. [@nut-tree-fork/nut-js - npm](https://www.npmjs.com/package/@nut-tree-fork/nut-js)
23. [Doug's AppleScripts - Key Codes](https://dougscripts.com/itunes/itinfo/keycodes.php)
24. [Top 12 Alternatives to RobotJS](https://testdriver.ai/articles/top-12-alternatives-to-robotjs-for-windows-macos-linux-testing)
