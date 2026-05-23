# App and Window Management

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > macOS System Automation

---

## Key Findings

- The `open` CLI command is the simplest way to launch apps; AppleScript `activate` both launches and brings to front
- JXA (JavaScript for Automation) is the best bridge between Node.js and macOS window management -- it uses JavaScript syntax and runs via `osascript -l JavaScript`
- Window position/size control requires Accessibility permissions on macOS; use `AXPosition`/`AXSize` attributes or app-level `bounds` property
- `node-window-manager` is the primary npm package for native window control, but its macOS support is limited compared to Windows
- For workspace presets, the most practical approach is JXA scripts invoked from Node.js via `child_process` or the `run-jxa` / `@jxa/run` npm packages
- `yabai` and Hammerspoon are powerful third-party tools for tiling/layout, but add dependencies; raw AppleScript/JXA keeps things self-contained
- macOS Sequoia (15) added native window tiling, but no public API for programmatic control of it
- Mission Control / Spaces has no direct API; switching spaces requires simulating keyboard shortcuts

---

## 1. Launching Applications

### 1.1 The `open` Command (Simplest)

The `open` command is the most straightforward way to launch apps from a shell:

```bash
# Launch by app name
open -a "Safari"

# Launch by bundle identifier
open -b com.apple.Safari

# Launch with a specific file
open -a "Visual Studio Code" ~/project/file.ts

# Launch with arguments (most GUI apps ignore these)
open -a "Google Chrome" --args --incognito

# Launch and bring to foreground (default behavior)
open -a "Slack"

# Open a URL
open "https://github.com"
```

From Node.js:

```typescript
import { exec } from 'child_process';

function launchApp(appName: string): Promise<void> {
  return new Promise((resolve, reject) => {
    exec(`open -a "${appName}"`, (error) => {
      if (error) reject(error);
      else resolve();
    });
  });
}

await launchApp('Safari');
```

### 1.2 AppleScript via `osascript`

AppleScript provides richer control. The `activate` command both launches an app (if not running) and brings it to the front:

```bash
# Launch and bring to front
osascript -e 'tell application "Safari" to activate'

# Launch without bringing to front
osascript -e 'tell application "Safari" to launch'

# Open a specific URL in Safari
osascript -e 'tell application "Safari" to open location "https://github.com"'
```

### 1.3 JXA (JavaScript for Automation)

JXA is AppleScript's JavaScript equivalent, invoked with `osascript -l JavaScript`:

```bash
# Launch and activate
osascript -l JavaScript -e 'Application("Safari").activate()'

# Launch without bringing to front
osascript -l JavaScript -e 'Application("Safari").launch()'
```

From Node.js using `child_process`:

```typescript
import { execSync } from 'child_process';

function runJXA(script: string): string {
  return execSync(`osascript -l JavaScript -e '${script.replace(/'/g, "'\\''")}'`, {
    encoding: 'utf-8',
  }).trim();
}

runJXA('Application("Safari").activate()');
```

---

## 2. Switching to / Focusing an App

### 2.1 Activate (AppleScript/JXA)

`activate` is the primary method. It launches the app if not running and brings all its windows to the front:

```bash
# AppleScript
osascript -e 'tell application "Slack" to activate'

# JXA
osascript -l JavaScript -e 'Application("Slack").activate()'
```

### 2.2 Bring a Specific Window to Front

To bring a specific window (not just the app) to focus, use System Events:

```javascript
// JXA: bring first window of an app to front
const se = Application('System Events');
const proc = se.processes.byName('Safari');
proc.frontmost = true;
// Focus a specific window by index
proc.windows[0].actions.byName('AXRaise').perform();
```

### 2.3 From Node.js with node-window-manager

```typescript
import { windowManager } from 'node-window-manager';

// Required on macOS before window manipulation
windowManager.requestAccessibility();

// Get all windows and find one by title
const windows = windowManager.getWindows();
const target = windows.find(w => w.getTitle().includes('My Project'));
if (target) {
  target.bringToTop();
}
```

---

## 3. Listing Running Applications and Windows

### 3.1 AppleScript: List Running Apps

```bash
osascript -e 'tell application "System Events" to get name of every process whose background only is false'
```

### 3.2 JXA: List Running Apps

```javascript
// Returns array of visible (non-background) app names
const se = Application('System Events');
const apps = se.processes.whose({ backgroundOnly: false });
const names = apps.name();
// names is now ["Finder", "Safari", "Slack", ...]
```

### 3.3 JXA: Get Frontmost App

```javascript
const se = Application('System Events');
const procs = se.processes.whose({ frontmost: true });
const frontApp = procs.name()[0];
// frontApp === "Safari" (or whatever is focused)
```

### 3.4 Core Graphics: List All Windows (Lower Level)

For exhaustive window enumeration, macOS provides `CGWindowListCopyWindowInfo`. This is a C/Swift API, but you can access it via JXA's Objective-C bridge:

```javascript
// JXA with ObjC bridge
ObjC.import('CoreGraphics');
ObjC.import('Foundation');

const windowList = $.CGWindowListCopyWindowInfo(
  $.kCGWindowListOptionOnScreenOnly,
  $.kCGNullWindowID
);
const count = $.CFArrayGetCount(windowList);

for (let i = 0; i < count; i++) {
  const dict = $.CFArrayGetValueAtIndex(windowList, i);
  const owner = $.CFDictionaryGetValue(dict, $('kCGWindowOwnerName'));
  const name = $.CFDictionaryGetValue(dict, $('kCGWindowName'));
  // Process window info...
}
```

### 3.5 Node.js: node-window-manager

```typescript
import { windowManager } from 'node-window-manager';

windowManager.requestAccessibility();

// Get all windows
const allWindows = windowManager.getWindows();
for (const win of allWindows) {
  console.log(`${win.path}: ${win.getTitle()} - ${JSON.stringify(win.getBounds())}`);
}

// Get currently focused window
const active = windowManager.getActiveWindow();
console.log(`Active: ${active.getTitle()}`);

// Listen for window focus changes
windowManager.on('window-activated', (window) => {
  console.log(`Switched to: ${window.getTitle()}`);
});
```

---

## 4. Window Position and Size Management

### 4.1 AppleScript: Set Window Bounds

The `bounds` property uses `{left, top, right, bottom}` format:

```applescript
tell application "Safari"
  activate
  -- Position window at (100, 50) with size 1200x800
  set bounds of front window to {100, 50, 1300, 850}
end tell
```

### 4.2 JXA: Set Window Bounds

JXA uses `{x, y, width, height}` format (different from AppleScript):

```javascript
const app = Application('Safari');
app.activate();

// Set position and size
app.windows[0].bounds = {
  x: 0,
  y: 25,     // 25 accounts for menu bar
  width: 960,
  height: 1080
};

// Read current bounds
const currentBounds = app.windows[0].bounds();
// { x: 0, y: 25, width: 960, height: 1080 }
```

### 4.3 JXA: Using Accessibility Attributes (More Reliable)

For apps that do not expose scriptable `bounds`, use System Events with AX attributes:

```javascript
const se = Application('System Events');
const proc = se.processes.byName('Safari');

// Get window position
const pos = proc.windows[0].position();   // [x, y]
const size = proc.windows[0].size();       // [width, height]

// Set window position and size
proc.windows[0].position = [100, 100];
proc.windows[0].size = [800, 600];
```

### 4.4 Node.js: node-window-manager

```typescript
import { windowManager } from 'node-window-manager';

windowManager.requestAccessibility();

const win = windowManager.getActiveWindow();

// Read bounds
const bounds = win.getBounds();
console.log(bounds); // { x: 0, y: 0, width: 1920, height: 1080 }

// Set bounds (any property can be omitted to keep current value)
win.setBounds({ x: 0, y: 0, width: 960, height: 1080 });

// Maximize / minimize / restore
win.maximize();
win.minimize();
win.restore();
```

**node-window-manager API coverage on macOS:**

| Method | macOS Support |
|--------|:---:|
| `getBounds()` | Yes |
| `setBounds()` | Yes |
| `getTitle()` | Yes |
| `minimize()` | Yes |
| `restore()` | Yes |
| `maximize()` | Yes |
| `bringToTop()` | Yes |
| `isWindow()` | Yes |
| `show()` | No (Windows only) |
| `hide()` | No (Windows only) |
| `setOpacity()` | No (Windows only) |
| `getMonitor()` | Returns empty |

### 4.5 Fullscreen Toggle

```bash
# AppleScript: toggle fullscreen for frontmost app
osascript -e '
tell application "System Events"
  tell (first process whose frontmost is true)
    set fullscreenState to value of attribute "AXFullScreen" of window 1
    set value of attribute "AXFullScreen" of window 1 to not fullscreenState
  end tell
end tell
'
```

JXA equivalent:

```javascript
const se = Application('System Events');
const proc = se.processes.whose({ frontmost: true })[0];
const win = proc.windows[0];
const isFS = win.attributes.byName('AXFullScreen').value();
win.attributes.byName('AXFullScreen').value = !isFS;
```

---

## 5. Workspace Layout Presets

This is the core use case for Stream Deck: pressing a button arranges all windows for a specific workflow.

### 5.1 Pure JXA Workspace Script

```javascript
// workspace-coding.jxa -- run with: osascript -l JavaScript workspace-coding.jxa

function setWindowBounds(appName, bounds) {
  try {
    const app = Application(appName);
    app.activate();
    delay(0.3);
    app.windows[0].bounds = bounds;
  } catch (e) {
    // App may not be running; optionally launch it
    const app = Application(appName);
    app.launch();
    delay(1);
    app.activate();
    delay(0.3);
    app.windows[0].bounds = bounds;
  }
}

function run() {
  // "Coding" workspace on a 2560x1440 display
  // Editor: left 60%
  setWindowBounds('Visual Studio Code', { x: 0, y: 25, width: 1536, height: 1415 });

  // Browser: right 40%
  setWindowBounds('Safari', { x: 1536, y: 25, width: 1024, height: 707 });

  // Terminal: bottom-right
  setWindowBounds('Terminal', { x: 1536, y: 732, width: 1024, height: 708 });

  // Focus back on editor
  Application('Visual Studio Code').activate();
}
```

### 5.2 Node.js Workspace Manager

```typescript
import { execSync } from 'child_process';

interface WindowLayout {
  app: string;
  x: number;
  y: number;
  width: number;
  height: number;
  focus?: boolean;
}

interface Workspace {
  name: string;
  windows: WindowLayout[];
}

const workspaces: Record<string, Workspace> = {
  coding: {
    name: 'Coding',
    windows: [
      { app: 'Visual Studio Code', x: 0, y: 25, width: 1536, height: 1415, focus: true },
      { app: 'Safari', x: 1536, y: 25, width: 1024, height: 707 },
      { app: 'Terminal', x: 1536, y: 732, width: 1024, height: 708 },
    ],
  },
  writing: {
    name: 'Writing',
    windows: [
      { app: 'Bear', x: 0, y: 25, width: 640, height: 1415 },
      { app: 'Safari', x: 640, y: 25, width: 1920, height: 1415, focus: true },
    ],
  },
  communication: {
    name: 'Communication',
    windows: [
      { app: 'Slack', x: 0, y: 25, width: 1280, height: 1415, focus: true },
      { app: 'Mail', x: 1280, y: 25, width: 1280, height: 1415 },
    ],
  },
};

function activateWorkspace(workspaceId: string): void {
  const workspace = workspaces[workspaceId];
  if (!workspace) throw new Error(`Unknown workspace: ${workspaceId}`);

  let focusApp: string | undefined;

  for (const win of workspace.windows) {
    if (win.focus) focusApp = win.app;

    const jxa = `
      (() => {
        const app = Application("${win.app}");
        try { app.activate(); } catch(e) { app.launch(); delay(1); app.activate(); }
        delay(0.3);
        app.windows[0].bounds = {
          x: ${win.x}, y: ${win.y},
          width: ${win.width}, height: ${win.height}
        };
      })()
    `;
    execSync(`osascript -l JavaScript -e '${jxa.replace(/'/g, "'\\''")}'`);
  }

  if (focusApp) {
    execSync(`osascript -l JavaScript -e 'Application("${focusApp}").activate()'`);
  }
}

// Stream Deck button press handler
activateWorkspace('coding');
```

### 5.3 Hammerspoon Layouts (Alternative Approach)

If Hammerspoon is installed, it provides a dedicated layout system in Lua:

```lua
-- ~/.hammerspoon/init.lua

local layouts = {
  coding = {
    {"Code", nil, nil, hs.layout.left60, nil, nil},
    {"Safari", nil, nil, hs.layout.right40, nil, nil},
    {"Terminal", nil, nil, {x=0.6, y=0.5, w=0.4, h=0.5}, nil, nil},
  },
  writing = {
    {"Bear", nil, nil, hs.layout.left25, nil, nil},
    {"Safari", nil, nil, hs.layout.right75, nil, nil},
  },
}

function applyLayout(name)
  hs.layout.apply(layouts[name])
end

-- Bind to hotkeys (Stream Deck can trigger these)
hs.hotkey.bind({"cmd", "alt"}, "1", function() applyLayout("coding") end)
hs.hotkey.bind({"cmd", "alt"}, "2", function() applyLayout("writing") end)
```

Trigger from Node.js:

```typescript
// Hammerspoon exposes an IPC interface
execSync('open -g "hammerspoon://applyLayout?name=coding"');
```

---

## 6. Mission Control and Spaces

Apple provides no public API for Spaces. All programmatic control relies on simulating keyboard shortcuts.

### 6.1 Switch Spaces via Keyboard Simulation

```bash
# Move to space to the right (Ctrl + Right Arrow)
osascript -e 'tell application "System Events" to key code 124 using control down'

# Move to space to the left (Ctrl + Left Arrow)
osascript -e 'tell application "System Events" to key code 123 using control down'

# Switch to specific space (Ctrl + 1-9, must be enabled in System Settings)
# Space 1: key code 18 = "1"
osascript -e 'tell application "System Events" to key code 18 using control down'
# Space 2: key code 19 = "2"
osascript -e 'tell application "System Events" to key code 19 using control down'
# Space 3: key code 20 = "3"
osascript -e 'tell application "System Events" to key code 20 using control down'
```

### 6.2 Open Mission Control

```bash
# Trigger Mission Control (Ctrl + Up Arrow)
osascript -e 'tell application "System Events" to key code 126 using control down'

# Or use the open command
open -a "Mission Control"
```

### 6.3 Show Application Windows (App Expose)

```bash
# Ctrl + Down Arrow
osascript -e 'tell application "System Events" to key code 125 using control down'
```

### 6.4 Yabai for Advanced Space Management

If `yabai` is installed, it provides real CLI commands for Spaces:

```bash
# Focus a specific space
yabai -m space --focus 2

# Move focused window to a space
yabai -m window --space 3

# Create a new space
yabai -m space --create

# Destroy current space
yabai -m space --destroy
```

---

## 7. System Settings Control

### 7.1 URL Scheme for System Settings

macOS supports deep-linking into System Settings via the `x-apple.systempreferences` URL scheme:

```bash
# Open Wi-Fi settings
open "x-apple.systempreferences:com.apple.Network-Settings.extension"

# Open Bluetooth settings
open "x-apple.systempreferences:com.apple.preferences.Bluetooth"

# Open Sound settings
open "x-apple.systempreferences:com.apple.Sound-Settings.extension"

# Open Display settings
open "x-apple.systempreferences:com.apple.Displays-Settings.extension"

# Open Keyboard settings
open "x-apple.systempreferences:com.apple.Keyboard-Settings.extension"

# Open Accessibility
open "x-apple.systempreferences:com.apple.Accessibility-Settings.extension"

# Open Notifications
open "x-apple.systempreferences:com.apple.Notifications-Settings.extension"

# Open Privacy & Security > Camera
open "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Camera"
```

### 7.2 AppleScript for System Settings Navigation

```applescript
tell application "System Settings"
  activate
  delay 0.5
  -- Navigate to a specific pane via its anchor
  reveal anchor "Privacy_Accessibility" of pane id "com.apple.settings.PrivacySecurity.extension"
end tell
```

---

## 8. Node.js Integration Summary

### 8.1 Recommended npm Packages

| Package | Purpose | macOS Support | Notes |
|---------|---------|:---:|-------|
| `node-window-manager` | Native window control | Partial | setBounds, getBounds, minimize, maximize work |
| `run-jxa` | Execute JXA from Node.js | Full | By sindresorhus; clean async API |
| `@jxa/run` | Execute JXA from Node.js | Full | Alternative to run-jxa |
| `node-osascript` | Execute AppleScript from Node.js | Full | Supports variable passing |
| `osascript` | Execute OSA scripts | Full | Supports both AS and JXA |

### 8.2 Recommended Architecture for Stream Deck

The most reliable approach for a Stream Deck plugin is a hybrid:

1. **App launching**: Use `child_process.exec('open -a "AppName"')` -- simple and reliable
2. **Window management**: Use JXA via `run-jxa` or raw `osascript -l JavaScript` -- full access to bounds, position, size
3. **App switching**: Use JXA `Application("name").activate()` -- launches if needed, brings to front
4. **Workspace presets**: Define layouts as JSON configs, execute via JXA scripts
5. **Spaces**: Simulate keyboard shortcuts via `osascript` System Events key codes

```typescript
// Minimal Stream Deck workspace handler
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

async function jxa(script: string): Promise<string> {
  const escaped = script.replace(/\\/g, '\\\\').replace(/'/g, "'\\''");
  const { stdout } = await execAsync(`osascript -l JavaScript -e '${escaped}'`);
  return stdout.trim();
}

// Launch and position
async function openAndPosition(app: string, x: number, y: number, w: number, h: number) {
  await jxa(`
    (() => {
      const app = Application("${app}");
      app.activate();
      delay(0.5);
      app.windows[0].bounds = { x: ${x}, y: ${y}, width: ${w}, height: ${h} };
    })()
  `);
}

// Get frontmost app name
async function getFrontmostApp(): Promise<string> {
  return jxa(`
    (() => {
      const se = Application("System Events");
      return se.processes.whose({ frontmost: true })[0].name();
    })()
  `);
}

// List all visible apps
async function listRunningApps(): Promise<string[]> {
  const result = await jxa(`
    (() => {
      const se = Application("System Events");
      return JSON.stringify(
        se.processes.whose({ backgroundOnly: false }).name()
      );
    })()
  `);
  return JSON.parse(result);
}
```

---

## 9. Accessibility Permissions

All window management on macOS requires Accessibility permissions. Without them, `setBounds`, AX attributes, and System Events window manipulation will fail silently or throw errors.

### 9.1 Checking Permission Programmatically

```bash
# Check if Terminal/your app has accessibility access (returns 0 or 1)
osascript -l JavaScript -e '
  ObjC.import("ApplicationServices");
  $.AXIsProcessTrusted();
'
```

### 9.2 Prompting for Permission

```bash
# Open the Accessibility preferences pane
open "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility"
```

With `node-window-manager`, calling `windowManager.requestAccessibility()` will prompt the dialog if permission is not yet granted.

### 9.3 What Needs Permission

| Action | Requires Accessibility |
|--------|:---:|
| `open -a "App"` | No |
| `Application("App").activate()` | No |
| Reading/setting `bounds` via JXA | Yes |
| System Events `position`/`size` | Yes |
| `AXFullScreen` toggle | Yes |
| `node-window-manager` setBounds | Yes |
| Keystroke simulation | Yes |

---

## Sources

1. [node-window-manager - npm](https://www.npmjs.com/package/node-window-manager)
2. [node-window-manager - GitHub](https://github.com/sentialx/node-window-manager)
3. [node-window-manager Window API docs](https://github.com/sentialx/node-window-manager/blob/master/docs/window.md)
4. [node-window-manager WindowManager API docs](https://github.com/sentialx/node-window-manager/blob/master/docs/window-manager.md)
5. [Swindler - macOS window management for Swift](https://github.com/tmandry/Swindler)
6. [Rectangle - macOS window manager](https://github.com/rxhanson/Rectangle)
7. [JXA Cookbook - System Events](https://github.com/JXA-Cookbook/JXA-Cookbook/wiki/System-Events)
8. [JXA Cookbook Wiki](https://github.com/JXA-Cookbook/JXA-Cookbook/wiki)
9. [Managing macOS windows with JXA - DEV Community](https://dev.to/pragli/how-to-manage-macos-windows-using-javascript-for-automation-jxa-428o)
10. [Automating macOS with JXA presentation](https://github.com/josh-/automating-macOS-with-JXA-presentation)
11. [run-jxa - npm (sindresorhus)](https://github.com/sindresorhus/run-jxa)
12. [@jxa/run - npm](https://www.npmjs.com/package/@jxa/run)
13. [node-applescript - GitHub](https://github.com/TooTallNate/node-applescript)
14. [node-osascript - npm](https://www.npmjs.com/package/node-osascript)
15. [Hammerspoon Grid Layouts](https://michaelheap.com/hammerspoon-layout/)
16. [Hammerspoon Sample Configurations](https://github.com/Hammerspoon/hammerspoon/wiki/Sample-Configurations)
17. [Yabai Commands Wiki](https://github.com/koekeishiya/yabai/wiki/Commands)
18. [Yabai configuration guide](https://evantravers.com/articles/2024/02/15/yabai-tiling-window-management-for-osx/)
19. [AppleScript fullscreen toggle gist](https://gist.github.com/dsummersl/4175461)
20. [AppleScript window positioning](https://alvinalexander.com/source-code/mac-os-x/how-size-or-resize-application-windows-applescript/)
21. [Apple System Preferences URL Schemes](https://gist.github.com/rmcdongit/f66ff91e0dad78d4d6346a75ded4b751)
22. [macOS Settings deep-link list](https://macmost.com/mac-settings-links)
23. [CGWindowListCopyWindowInfo - Apple Developer](https://developer.apple.com/documentation/coregraphics/1455137-cgwindowlistcopywindowinfo)
24. [NSRunningApplication - Apple Developer](https://developer.apple.com/documentation/appkit/nsrunningapplication)
25. [frontmostApplication - Apple Developer](https://developer.apple.com/documentation/appkit/nsworkspace/frontmostapplication)
26. [macOS Sequoia window tiling - MacRumors](https://www.macrumors.com/2024/06/12/macos-sequoia-window-tiling/)
27. [ArrangeWindows with AppleScript - cybercafe.dev](https://cybercafe.dev/arrange-multiple-terminal-windows-using-applescript-and-automator/)
28. [DFAXUIElement - Accessibility API wrapper](https://github.com/DevilFinger/DFAXUIElement)
