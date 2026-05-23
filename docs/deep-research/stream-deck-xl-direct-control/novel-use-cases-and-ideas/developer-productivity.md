# Developer Productivity

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Novel Use Cases & Ideas

---

## Key Findings

- The Stream Deck XL's 32 buttons, combined with profiles, pages, and folders, provide enough surface area to dedicate entire button groups to IDE control, git workflows, CI/CD monitoring, Docker management, meeting controls, and AI agent interaction -- all without switching contexts.
- Smart Profiles automatically swap button layouts when the foreground app changes (e.g., VS Code, terminal, Zoom), making the hardware contextually aware of what you are doing.
- The official Elgato SDK (Node.js / TypeScript) and the community `python-elgato-streamdeck` library both allow building fully custom plugins -- meaning anything with an API can be wired to a button.
- CI/CD status monitoring is production-ready today: the DevOps for StreamDeck plugin shows green/red/yellow indicators for GitHub Actions, GitLab CI, Vercel, and Netlify pipelines.
- The Claude Code + Stream Deck integration space has exploded: AgentDeck, TerminalDeck, streamdeck-mcp, and DeckMate all ship purpose-built bridges between AI coding agents and physical buttons.
- Meeting controls (Zoom, Teams, Google Meet) via MuteDeck or native Zoom plugin eliminate fumbling for mute buttons during calls -- a universally cited productivity win for remote developers.
- Custom shell-script buttons are the most versatile primitive: one button can run `git add . && git commit -m "wip" && git push`, start a dev server, or trigger a deploy webhook.

---

## 1. IDE Integration

### VS Code

The primary integration is the **vscode-deck** extension (by nicollasr), available both as a VS Code extension and a Stream Deck plugin on the Elgato Marketplace. It establishes a message server between VS Code and the Stream Deck hardware, allowing buttons to trigger any VS Code command by its command ID.

**Setup process:**
1. Install the VS Code extension from the marketplace
2. Install the paired Stream Deck plugin
3. Open VS Code keyboard shortcuts (File > Preferences > Keyboard Shortcuts)
4. Right-click any command, copy its command ID
5. Paste the command ID into the Stream Deck button configuration

**High-value button mappings for VS Code:**
- Toggle terminal panel (`workbench.action.terminal.toggleTerminal`)
- Toggle sidebar (`workbench.action.toggleSidebarVisibility`)
- Go to definition (`editor.action.revealDefinition`)
- Find in files (`workbench.action.findInFiles`)
- Toggle Zen Mode (`workbench.action.toggleZenMode`)
- Split editor (`workbench.action.splitEditor`)
- Quick open file (`workbench.action.quickOpen`)
- Format document (`editor.action.formatDocument`)
- Toggle word wrap (`editor.action.toggleWordWrap`)
- Run task (`workbench.action.tasks.runTask`)

**NPM Scripts Deck** is a more advanced framework that dynamically discovers and displays NPM scripts from the active VS Code workspace on Stream Deck buttons. When your `package.json` changes, the buttons update. The author designed it as an extensible framework supporting additional providers for Git, Docker, dotnet, and more.

### JetBrains (IntelliJ, WebStorm, GoLand, etc.)

JetBrains ships an **official Stream Deck plugin** (`intellij-streamdeck-plugin`) that works with all JetBrains IDEs (IDEA Community/Ultimate, WebStorm, Rider, Android Studio, PhpStorm, RubyMine, GoLand). It follows a zero-config design -- install and it works.

**Key capabilities:**
- Controls up to 10 simultaneously running local IDEs from one Stream Deck
- Supports remote IDE connections
- Uses the IDE's Action Browser (Help > Open Action Browser) to discover action IDs

**Documented action examples:**

| Action | ID | Default Shortcut |
|---|---|---|
| Toggle Project panel | `ActivateProjectToolWindow` | Cmd+1 |
| Toggle Structure panel | `ActivateStructureToolWindow` | Cmd+7 |
| Toggle Terminal | `ActivateTerminalToolWindow` | Opt+F12 |
| Toggle Gradle panel | `ActivateGradleToolWindow` | -- |
| Close tab | `CloseContent` | Cmd+W |
| Next tab | `NextTab` | Shift+Cmd+] |
| Open file | `OpenFile` | -- |

Any action registered in the IDE's action system can be bound to a Stream Deck button, making this extremely powerful for refactoring workflows, debugging controls (step over, step into, evaluate expression), and test execution.

### General IDE Strategy

For IDEs without native plugins, the hotkey approach works universally:
1. Assign a unique keyboard shortcut in the IDE
2. Map the Stream Deck button to send that hotkey
3. Use Smart Profiles to auto-switch the button layout when the IDE gains focus

---

## 2. Git Workflow Buttons

Git operations are among the most natural fits for Stream Deck because they are frequent, repetitive, and dangerous enough to want visual confirmation.

### Basic Git Operations

Map shell commands via the System > Open action (or a shell-script plugin):

| Button | Command |
|---|---|
| Quick Commit (WIP) | `cd ~/project && git add -A && git commit -m "wip"` |
| Push | `cd ~/project && git push` |
| Pull & Rebase | `cd ~/project && git pull --rebase` |
| Stash | `cd ~/project && git stash` |
| Pop Stash | `cd ~/project && git stash pop` |
| Status | `cd ~/project && git status` (display in notification) |

### GitHub Integration (Marketplace Plugins)

Five GitHub-related plugins exist on the Elgato Marketplace:

1. **GitHub API** -- Uses GitHub's GraphQL API to display arbitrary data on buttons. You write a GraphQL query and specify the JSON path to the value you want displayed. Examples:
   - Total contributions this year via `user.contributionsCollection.contributionCalendar.totalContributions`
   - Open PR count via `search(query: "type:pr author:@me is:open") { issueCount }`
   - Open issue count with a similar search query
   - Mergeable status of specific PRs

2. **GitHub Utilities** -- Pre-built actions for common GitHub operations (v2.3, Mac/Windows).

3. **GitHub Contributions** -- Displays your contribution graph/count on a button.

4. **GitHub CI** -- Shows CI pipeline status for specific repos.

5. **DMS GitHub Plugin** -- Additional GitHub workflow shortcuts.

### Branch Status Indicator Pattern

Using the GitHub API plugin with conditional background colors:
- **Green background**: All checks passing on main branch
- **Red background**: Failing checks
- **Yellow background**: Checks running

This provides at-a-glance pipeline health without opening a browser.

### Custom Git Workflow Ideas

- **Branch switcher page**: One button per feature branch, showing which is checked out
- **PR creation button**: Multi-action that pushes the current branch, then opens `gh pr create --web` in the browser
- **Conflict indicator**: Script that runs `git status` and changes the button color if merge conflicts are detected
- **Diff reviewer**: Opens `git diff` in a configured diff tool

---

## 3. CI/CD Pipeline Monitoring

### DevOps for StreamDeck Plugin

The **DevOps for StreamDeck** plugin (by SantiMA10) is the most comprehensive CI/CD monitoring solution. It supports:

- **GitHub Actions** -- monitor workflow status per repository
- **GitLab CI** -- pipeline status tracking
- **Vercel** -- deployment status by project name
- **Netlify** -- build status by Site ID
- **Travis CI** -- build monitoring

**Configuration**: For each button, specify the username/repo (GitHub/GitLab), Site ID (Netlify), or project name (Vercel). Optionally filter to a specific branch or show all-branch status. The button displays colored indicators: green for passing, red for failing, yellow for in-progress.

### Netlify Deploy Trigger

Netlify published an official guide for triggering deploys from Stream Deck:

1. Create a build hook in Netlify (Settings > Build & deploy > Build hooks)
2. Create a serverless function that accepts a secret parameter and POSTs to the build hook URL
3. Configure a Stream Deck "Website" button to call the function URL with the secret
4. Optionally enable "GET request in background" to avoid opening a browser

This pattern works for any service with webhook-based deploy triggers (Vercel, Railway, Render, etc.).

### Custom CI/CD Dashboard

For the Stream Deck XL's 32 buttons, you could dedicate an entire page to CI/CD:

| Row | Buttons |
|---|---|
| Row 1 | Main branch status for each critical repo (8 buttons) |
| Row 2 | Staging deploy status, preview deploys, Vercel status (8 buttons) |
| Row 3 | Production deploy triggers with confirmation, rollback buttons (8 buttons) |
| Row 4 | Test coverage trends, security scan status, dependency update status (8 buttons) |

Each button polls its respective API at configurable intervals and updates the icon/color accordingly.

---

## 4. Docker & Container Management

### Elgato Docker Plugin

The **elgato-docker** plugin (available on the Elgato Marketplace) provides direct Docker management from Stream Deck buttons:

- **Start/Stop containers** with a single key press
- **Display container status** (running, stopped, exited) directly on the button face
- **Manage Docker Compose stacks** alongside individual containers
- **Select Docker Contexts** per button, including remote connections with TLS
- **Multi-container monitoring** across multiple buttons simultaneously

**Setup requirements:**
- Docker Engine API must be enabled
- The plugin connects via the Docker socket
- Each button is configured with a container name or stack name

### Developer Use Cases

| Button | Action |
|---|---|
| PostgreSQL | Start/stop the local PostgreSQL container |
| Redis | Start/stop Redis with status indicator |
| Full Stack Up | `docker compose up -d` for the entire dev stack |
| Full Stack Down | `docker compose down` |
| Rebuild | `docker compose build --no-cache && docker compose up -d` |
| Logs | Open container logs in a terminal window |

### Kubernetes (Gap)

No dedicated Stream Deck plugin exists for Kubernetes management. However, the pattern is straightforward to build:
- Use shell-script buttons to run `kubectl` commands
- `kubectl get pods -o json` piped through `jq` to extract status for button icons
- Context switching between clusters via `kubectl config use-context`
- Port-forward toggles for accessing services locally

---

## 5. Terminal & SSH Session Management

### Direct Terminal Control

Several approaches exist for terminal management:

**AppleScript method** (macOS): One developer built a local Ruby Sinatra web server (port 4040) that executes commands via HTTP endpoints, triggered by Stream Deck "Website" buttons. This avoids the unreliable AppleScript plugin and provides a clean separation of concerns.

**iTerm2 integration**: Create buttons that open new iTerm2 tabs with pre-configured SSH connections:
```
tell application "iTerm2"
    create window with default profile
    tell current session of current window
        write text "ssh user@server-name"
    end tell
end tell
```

**TerminalDeck**: A dedicated Tauri app (Rust + React) that provides a 15-button interface for controlling terminals. It supports Terminal.app, iTerm2, and Warp, using HID API for direct Stream Deck communication and AppleScript for terminal control.

### SSH Server Dashboard

Dedicate a Stream Deck page to server management:

| Button | Action |
|---|---|
| Production SSH | Open SSH session to production bastion |
| Staging SSH | Open SSH session to staging |
| DB Tunnel | `ssh -L 5432:db-host:5432 bastion` for database access |
| Logs | `ssh server 'tail -f /var/log/app.log'` in a new tab |
| Restart App | `ssh server 'sudo systemctl restart app'` |

### VS Code Remote Development

Buttons can launch VS Code remote sessions directly:
```
code --folder-uri vscode-remote://ssh-remote+devserver/path/to/project
```

This opens a remote VS Code window connected to the specified server and directory -- one button press to jump into a remote codebase.

---

## 6. Meeting Controls

Meeting controls are consistently cited as the single highest-ROI Stream Deck use case for remote developers. The cognitive cost of fumbling for the mute button during a call is disproportionately high.

### Zoom (Official Integration)

Stream Deck is the **first Zoom Certified Personal Productivity Device**, providing direct two-way communication with reliable state sync and dynamic visual feedback.

**Available actions:**
- Mute / unmute microphone (with visual state indicator)
- Toggle camera on/off
- Start / stop screen share
- Record locally or to the cloud
- Raise / lower hand
- Send emoji reactions
- Leave meeting

The button icons update in real-time to reflect the current state (e.g., mic icon turns red when muted).

### MuteDeck (Multi-Platform)

For developers who use multiple meeting platforms, **MuteDeck** provides unified controls across:
- Zoom, Microsoft Teams, Google Meet
- Slack, Discord, FaceTime
- Riverside, StreamYard
- Web versions of Zoom and Teams

Five buttons cover all essential controls: mute, camera, share, leave, and status. The same buttons work regardless of which platform is active.

### Meeting Page Layout (Stream Deck XL)

| Button | Function |
|---|---|
| Mic Toggle | Mute/unmute with red/green indicator |
| Camera Toggle | On/off with visual state |
| Screen Share | Start/stop sharing |
| Raise Hand | Toggle hand raise |
| Leave Call | Exit meeting |
| DND On | Enable system-wide Do Not Disturb |
| DND Off | Disable Do Not Disturb |
| Slack Snooze | Snooze Slack notifications during meeting |

### Smart Profile Strategy

Configure a Smart Profile for Zoom (and/or Teams/Meet) so the meeting controls automatically appear when you join a call, and the regular development buttons return when the call ends.

---

## 7. Pomodoro & Time Tracking

### Pomodoro Timers

Multiple Pomodoro plugins are available on the Elgato Marketplace:

- **Tomato Timer** -- Configurable work/break intervals with countdown display on the button face
- **Productivity Timer** -- General-purpose timer with customizable durations
- **Task Timer** -- Task-oriented timer for tracking work sessions
- **streamdeck-pomodoro** (community) -- Open-source Pomodoro implementation

The button displays a live countdown and automatically transitions to break mode when the work interval ends.

### Time Tracking Services

**Toggl Track integration:**
- Press a Toggl button to start tracking time on a specific project
- Button turns red and displays elapsed time while tracking
- Pressing again stops the timer
- State syncs bidirectionally -- starting/stopping in the Toggl app updates the button

**Clockify integration:**
- Similar start/stop functionality
- Configure workspace, project, and task per button
- Multiple plugins available (official and community forks)

### Developer Time Tracking Layout

| Button | Action |
|---|---|
| Client A | Start/stop Toggl timer for Client A project |
| Client B | Start/stop Toggl timer for Client B project |
| Internal | Start/stop timer for internal/overhead work |
| Focus 25 | Start 25-minute Pomodoro focus session |
| Break 5 | Start 5-minute break timer |
| Long Break | Start 15-minute break timer |
| DND + Focus | Multi-action: enable DND, start Pomodoro, set Slack status to "Focusing" |

---

## 8. Claude Code & AI Assistant Integration

This is the fastest-growing area of Stream Deck developer tooling. Four major projects have emerged:

### AgentDeck

**AgentDeck** is the most ambitious project -- a physical control surface analogous to an audio mixing console, but for AI coding agents. It reads agent state in real-time and dynamically reconfigures buttons and encoders.

**Supported AI agents:** Claude Code, Codex CLI, OpenCode, OpenClaw

**Key capabilities:**
- Semantic button colors: green (approve), red (deny), blue (permanent allow)
- STOP button sends Ctrl+C to interrupt agents
- Mode switching: Plan, Accept Edits, Default
- Push-to-talk voice input via Apple SFSpeech (on-device, offline)
- Animated water-gauge dashboards for token usage with rate-limit countdowns
- Multi-agent session management across up to 13 display surfaces

**Architecture:** Hub-and-spoke model with a central daemon (port 9120) aggregating state from session bridges (port 9121+). Claude Code hooks (7 total) POST JSON capturing SessionStart, SessionEnd, PreToolUse, PostToolUse, Stop, Notification, and UserPromptSubmit events.

**Hardware support:** Stream Deck+, Android tablets, iPhone/iPad/macOS, ESP32 displays, Ulanzi devices, Pixoo64, and terminal TUI.

### TerminalDeck

**TerminalDeck** enables hands-free control of Claude Code through Stream Deck MK.2 and voice dictation. Built with Tauri v2 (Rust + React).

**15-button layout includes:**
- Launch Claude Code
- Open/switch/manage terminal windows
- Respond to prompts (Yes, No, Yes to All, Cancel) without keyboard
- Navigate conversation history
- Toggle voice dictation for completely hands-free coding
- Submit prompts and execute commands

### streamdeck-mcp (Model Context Protocol Server)

**streamdeck-mcp** takes the opposite approach: instead of controlling Claude from Stream Deck, it lets Claude control the Stream Deck. An AI assistant describes the desired layout ("make me a Slack control board") and the MCP server generates complete profiles with themed buttons, custom icons, and configured actions.

**Available MCP tools:**
- `streamdeck_read_profiles` -- inventory active profiles
- `streamdeck_read_page` -- retrieve page manifests
- `streamdeck_write_page` -- create/modify pages
- `streamdeck_create_icon` -- generate PNG icons from 7,400+ Material Design Icons
- `streamdeck_create_action` -- write executable shell scripts
- `streamdeck_restart_app` -- relaunch Stream Deck app to pick up changes

**Installation for Claude Code:** `claude mcp add streamdeck -- uvx streamdeck-mcp`

It includes a bundled Designer Skill with 8 theme archetypes (kawaii, retrowave, brutalist, Nordic, terminal, nature, minimal, corporate).

### DeckMate

**DeckMate** focuses on generating Stream Deck profiles optimized for Claude Code workflows and Tactical Agentic Coding (TAC) patterns. It automates creation of manifest files, button icons, and multi-action sequences for triggering terminal commands, injecting prompt snippets, and navigating project structures.

### Custom AI Integration Ideas

For developers building their own tooling:

| Button | Action |
|---|---|
| Ask Claude | Run `claude -p "$(pbpaste)"` with clipboard contents as prompt |
| Explain Error | Pipe last terminal error to `claude -p "explain this error"` |
| Review Diff | Run `git diff \| claude -p "review this diff for bugs"` |
| Generate Tests | `claude -p "generate tests for $(pbpaste)"` |
| Token Dashboard | Display current session token usage from Claude hooks |
| Approve All | Send "yes to all" to active Claude Code session |
| Interrupt | Send Ctrl+C to interrupt Claude Code |

---

## 9. Database Quick-Access

No dedicated Stream Deck database plugin exists, but shell-script buttons provide a clean solution.

### Connection Management

| Button | Action |
|---|---|
| DB Connect | Open psql/mysql client in a new terminal tab |
| DB Tunnel | `ssh -L 5432:rds-endpoint:5432 bastion-host` |
| DB GUI | Open TablePlus/DataGrip/pgAdmin |
| Connection Status | Script that tests connection and updates button color (green/red) |

### Saved Query Buttons

For frequently run queries during development:

| Button | Action |
|---|---|
| Recent Users | `psql -c "SELECT * FROM users ORDER BY created_at DESC LIMIT 10"` |
| Active Sessions | `psql -c "SELECT count(*) FROM pg_stat_activity"` |
| Table Sizes | `psql -c "SELECT relname, pg_size_pretty(pg_total_relation_size(relid)) FROM pg_catalog.pg_statio_user_tables ORDER BY pg_total_relation_size(relid) DESC LIMIT 10"` |
| Reset Dev DB | `psql -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"` (with confirmation!) |
| Run Migrations | `pnpm db:migrate` or `npx prisma migrate dev` |
| Seed Data | `pnpm db:seed` |

### Redis/Cache Management

| Button | Action |
|---|---|
| Redis CLI | Open `redis-cli` in terminal |
| Flush Cache | `redis-cli FLUSHDB` (with confirmation) |
| Cache Stats | `redis-cli INFO stats` displayed as notification |

---

## 10. Multi-Monitor & Virtual Desktop Switching

### Window Mover Plugin

The **Window Mover** plugin detects your monitors automatically and provides per-display layout management:

- Choose target display and window position (left half, right half, top-right, bottom-left, full screen)
- Define exact pixel size and position for any app window
- Target the foreground window, a specific app, or a window by title
- Save and restore entire window layouts with one button press

### Developer Workspace Layouts

| Button | Action |
|---|---|
| Code Layout | VS Code full-screen on monitor 1, terminal half-screen on monitor 2 |
| Review Layout | Browser on monitor 1, VS Code on monitor 2, terminal on monitor 3 |
| Meeting Layout | Zoom on monitor 1, notes on monitor 2, screen share content on monitor 3 |
| Focus Mode | Maximize current app, hide all others, enable DND |
| Restore All | Return all windows to their default positions |

### Virtual Desktop Switching

On macOS, use hotkey buttons mapped to Mission Control shortcuts (Ctrl+Left/Right arrow) to switch between Spaces. Combined with Keyboard Maestro or BetterTouchTool, you can create buttons that jump to specific numbered desktops.

### BetterTouchTool Integration (macOS)

BetterTouchTool added direct Stream Deck support, providing capabilities beyond the standard software:

- Fully scriptable buttons with state feedback
- Window snapping and management beyond what Window Mover offers
- AppleScript/JavaScript execution for complex window arrangements
- Conditional button states based on active app or window title

---

## 11. Development Server Management

### Dev Server Controls

| Button | Action |
|---|---|
| Start Dev | `cd ~/project && pnpm dev` in a new terminal tab |
| Stop Dev | Kill process on port 3000: `lsof -ti:3000 \| xargs kill` |
| Restart Dev | Stop then start in sequence |
| Build | `pnpm build` in background |
| Type Check | `pnpm tsc --noEmit` |
| Lint | `pnpm lint` |
| Test | `pnpm test` |
| Test Watch | `pnpm test --watch` in a new terminal tab |

### Port Status Monitoring

A custom script can poll common development ports and update button colors:

```bash
#!/bin/bash
# Check if port 3000 is in use
if lsof -i:3000 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "running"  # Green button
else
    echo "stopped"  # Red button
fi
```

### Multi-Service Dev Environment

For microservice architectures, dedicate buttons to each service:

| Button | Service | Port |
|---|---|---|
| Frontend | Next.js | 3000 |
| API | Express/Fastify | 3001 |
| Auth | Auth service | 3002 |
| Workers | Background jobs | 3003 |
| DB | PostgreSQL | 5432 |
| Redis | Cache | 6379 |
| All Up | Start everything | -- |
| All Down | Stop everything | -- |

Each button shows green when the service is running and red when stopped, providing a hardware status dashboard for your local development environment.

---

## 12. Notification Routing & Alerting

### Slack Integration

Multiple approaches for Slack on Stream Deck:

**DevDeck-Slack** (Python, Linux): Manages presence, status, and DND directly from buttons.

**Slack URI Links** (all platforms): Configure buttons as website actions pointing to Slack deep links:
```
slack://channel?team=TXXXXXXXX&id=CXXXXXXXX
```
This jumps directly to a specific channel or DM.

**Slack Status Buttons:**
| Button | Action |
|---|---|
| Set Focusing | Set Slack status to "Focusing" with DND |
| Set Available | Clear status, disable DND |
| Jump to DMs | Open direct messages |
| Jump to #dev | Open the #dev channel |
| Snooze 1hr | Snooze notifications for 1 hour |

### PagerDuty / On-Call Alerting

While no dedicated Stream Deck PagerDuty plugin exists, the pattern is implementable via the API:

- **On-call indicator**: Poll the PagerDuty API and show a green/red button based on whether you are currently on-call
- **Acknowledge incident**: Button that calls `POST /incidents/{id}/acknowledge`
- **Incident count**: Display number of open incidents on the button face

### System Monitor Buttons

Plugins exist for displaying system vitals on Stream Deck buttons:

- CPU usage percentage with gauge visualization
- RAM usage with color-coded thresholds
- GPU utilization and temperature
- Network throughput
- Disk usage

This transforms a row of Stream Deck buttons into a hardware monitoring dashboard -- useful for noticing when a runaway process is consuming resources during development.

---

## 13. Advanced Automation Patterns

### Multi-Action Workflows

Stream Deck's multi-action feature chains multiple steps into a single button press. Developer-focused examples:

**Morning Startup Button:**
1. Open VS Code in the current project
2. Open iTerm2 with project directory
3. Start dev server
4. Open browser to localhost:3000
5. Set Slack status to "Working"
6. Start Pomodoro timer

**Deploy to Staging Button:**
1. Run `pnpm build`
2. Run `pnpm test`
3. Push to staging branch
4. Monitor deploy status
5. Send Slack notification when complete

**End of Day Button:**
1. Commit any uncommitted work as WIP
2. Push all branches
3. Stop all dev servers
4. Set Slack status to "Away"
5. Lock screen

### Key Logic (Multi-Press)

Stream Deck supports three actions per button:
- **Single press**: Primary action
- **Double press**: Secondary action
- **Long press**: Tertiary action

Example: A git button could map:
- Single press: `git status`
- Double press: `git add -A && git commit -m "wip"`
- Long press: `git push`

### Profile Architecture for the XL

With 32 buttons, a developer can organize profiles as:

**Page 1 -- Command Center:**
- Row 1: App launchers and workspace layouts (8 buttons)
- Row 2: Git operations and branch management (8 buttons)
- Row 3: Dev server controls and port status (8 buttons)
- Row 4: Meeting controls, music, system monitors (8 buttons)

**Page 2 -- CI/CD & Infrastructure:**
- Row 1: GitHub Actions status for each repo (8 buttons)
- Row 2: Deploy triggers and rollback controls (8 buttons)
- Row 3: Docker container management (8 buttons)
- Row 4: Database controls and SSH sessions (8 buttons)

**Page 3 -- AI & Focus:**
- Row 1: Claude Code controls and prompt triggers (8 buttons)
- Row 2: Pomodoro timers and time tracking (8 buttons)
- Row 3: Notification management and DND modes (8 buttons)
- Row 4: Context-specific actions (8 buttons)

### Smart Profiles (Auto-Switching)

Configure Smart Profiles so the Stream Deck automatically shows:
- **VS Code profile** when VS Code is focused
- **Terminal profile** when iTerm2/Terminal is focused
- **Meeting profile** when Zoom/Teams/Meet is active
- **Browser profile** when Chrome/Firefox is focused
- **Default profile** for everything else

---

## 14. Building Custom Plugins

### Official SDK (Node.js / TypeScript)

The Elgato Stream Deck SDK uses Node.js 24+ and communicates via WebSocket. The CLI provides scaffolding:

```bash
npx @anthropic-ai/create-stream-deck-plugin my-plugin
```

**Plugin structure:**
- `manifest.json` -- metadata, actions, version compatibility
- `src/` -- TypeScript source files
- Property Inspector (HTML/CSS/JS) for button configuration UI

Plugins respond to WebSocket events (button pressed, action appeared, settings changed) and can update button images, titles, and states in real-time.

### python-elgato-streamdeck Library

For developers who prefer Python or need to bypass the official software entirely:

```bash
pip install streamdeck
```

This library communicates directly with Stream Deck hardware via USB HID, allowing:
- Enumerate connected devices
- Set brightness
- Set images on each button (PIL/Pillow images)
- Read button states via callbacks
- Full control without the Elgato desktop app

This is the foundation that DevDeck (the YAML-configured Python framework) is built on, and it is especially useful on Linux where the official software is not available.

### Custom Plugin Ideas for Developers

| Plugin Concept | Description |
|---|---|
| **PR Review Queue** | Poll GitHub API, show count of PRs awaiting your review, cycle through them |
| **Dependency Dashboard** | Show outdated dependency count, one-button update |
| **Error Monitor** | Connect to Sentry/Datadog, show error count with severity colors |
| **Feature Flag Controller** | Toggle LaunchDarkly/Unleash flags from buttons |
| **API Health Checker** | Ping endpoints, show response time on button, alert on failure |
| **Log Streamer** | Tail logs with keyword highlighting, flash button on error patterns |
| **Environment Switcher** | Toggle between dev/staging/prod configs for local development |
| **Schema Diff** | Compare database schema between environments |

---

## 15. macOS Automation Layer

For macOS developers, the automation stack deepens significantly when combining Stream Deck with system-level tools.

### Keyboard Maestro

Keyboard Maestro macros triggered by Stream Deck buttons can:
- Detect which app is frontmost and adjust behavior accordingly
- Use image recognition to find UI elements and interact with them
- Chain complex multi-app workflows with conditional logic
- Manipulate clipboard contents before pasting

### Mac Automation Plugin

The free **Mac Automation** plugin for Stream Deck bundles:
- Run Keyboard Maestro macros by name or UUID
- Execute AppleScript directly from buttons
- Run JavaScript for Automation (JXA)
- Trigger macOS Shortcuts

### Raycast Integration

Raycast (a Spotlight replacement popular with developers) integrates via hotkeys:
- Trigger Raycast extensions from Stream Deck
- Open the emoji picker
- Toggle floating notes
- Launch Raycast AI
- Run custom Raycast scripts

### Bunch App

Bunch (`.bunch` files) automates entire workspace contexts:
```
# coding.bunch
VS Code ~
iTerm2 ~
Firefox ~ https://localhost:3000
@slack set-status "Coding" :computer:
@dnd on
```

A Stream Deck button launches the Bunch, which opens all apps, arranges windows, sets statuses, and enables focus mode.

---

## Implementation Priority for a Developer

If setting up a Stream Deck XL from scratch, prioritize in this order based on daily time savings:

| Priority | Category | Expected Daily Savings |
|---|---|---|
| 1 | Meeting controls (mute, camera, share) | 5-10 minutes of fumbling |
| 2 | Dev server start/stop and port management | 3-5 minutes of terminal commands |
| 3 | Git operations (commit, push, status) | 5-10 minutes of typing |
| 4 | App launching and window management | 3-5 minutes of arranging |
| 5 | CI/CD status monitoring | 5-10 minutes of tab switching |
| 6 | Docker container management | 2-5 minutes of docker commands |
| 7 | Claude Code / AI agent controls | Variable, growing rapidly |
| 8 | Smart Profiles for context switching | Eliminates manual profile swaps |
| 9 | Pomodoro and time tracking | Discipline aid, hard to quantify |
| 10 | Custom plugins for specific workflows | Depends on workflow |

The total addressable time savings for an active developer is roughly 30-60 minutes per day, not counting the cognitive load reduction from having visual, tactile controls instead of remembering keyboard shortcuts across dozens of applications.

---

## Sources

1. [Using a Stream Deck for productivity -- a software developer's solution (James Ridgway)](https://www.jamesridgway.co.uk/using-a-stream-deck-for-productivity-a-software-developers-solution/)
2. [Displaying GitHub information on Elgato Stream Deck (DEV Community)](https://dev.to/kasuken/displaying-github-information-on-elgato-streamdeck-4g96)
3. [AgentDeck -- Physical controller for AI coding agents (GitHub)](https://github.com/puritysb/AgentDeck)
4. [TerminalDeck -- Control Claude Code with Stream Deck (GitHub)](https://github.com/sidmohan0/terminaldeck)
5. [streamdeck-mcp -- MCP server for Stream Deck control (GitHub)](https://github.com/verygoodplugins/streamdeck-mcp)
6. [DeckMate -- Stream Deck Integration for Claude Code](https://mcpmarket.com/zh/tools/skills/deckmate-stream-deck-assistant)
7. [Stream Deck Docker plugin (GitHub)](https://github.com/Darkdragon14/streamdeck-docker)
8. [DevOps for StreamDeck -- CI/CD status plugin (GitHub)](https://github.com/SantiMA10/devops-streamdeck)
9. [JetBrains IntelliJ Stream Deck plugin (GitHub)](https://github.com/JetBrains/intellij-streamdeck-plugin)
10. [VS Code Stream Deck extension (GitHub)](https://github.com/nicollasricas/vscode-deck)
11. [NPM Scripts Deck -- VSCode and Stream Deck framework (DEV Community)](https://dev.to/ugaya40/showdevnpm-scripts-deck-vscode-and-stream-deck-integration-framework-with-npm-scripts-provider-7k4)
12. [How a Stream Deck Can Revolutionize Your Coding Workflow (Finchett)](https://finchett.com/how-a-stream-deck-can-revolutionize-your-coding-workflow/)
13. [Stream Deck for Developers (adam.ac)](https://adam.ac/blog/stream-deck-for-developers/)
14. [My Stream Deck Setup (Sebastian Witowski)](https://switowski.com/blog/my-stream-deck-setup/)
15. [Deploying Netlify sites with Stream Deck (Netlify Blog)](https://www.netlify.com/blog/how-to-deploy-your-netlify-site-with-an-elgato-stream-deck/)
16. [MuteDeck -- Universal meeting controls](https://mutedeck.com/)
17. [Stream Deck Zoom Plugin (Elgato)](https://www.elgato.com/us/en/explorer/products/stream-deck/zoom-plugin-stream-deck/)
18. [Toggl Stream Deck plugin (GitHub)](https://github.com/tobimori/streamdeck-toggl)
19. [Clockify Stream Deck plugin (GitHub)](https://github.com/KaiReichart/streamdeck-clockify)
20. [python-elgato-streamdeck library (GitHub)](https://github.com/abcminiuser/python-elgato-streamdeck)
21. [Stream Deck SDK documentation (Elgato)](https://docs.elgato.com/streamdeck/sdk/introduction/getting-started/)
22. [Window Mover plugin (Elgato)](https://www.elgato.com/us/en/explorer/products/marketplace/window-mover-for-stream-deck-organize-your-apps-instantly/)
23. [BetterTouchTool and Stream Deck (Brett Terpstra)](https://brettterpstra.com/2022/07/02/bettertouchtool-and-stream-deck/)
24. [Mac Automation plugin for Stream Deck (ThoughtAsylum)](https://www.thoughtasylum.com/2025/07/14/stream-deck-plugin-mac-automation/)
25. [Stream Deck developer profiles (GitHub, mikeckennedy)](https://github.com/mikeckennedy/streamdeck-developer-profiles)
26. [Elgato Marketplace -- Development plugins](https://marketplace.elgato.com/stream-deck/plugins?type=development)
27. [Stream Deck Smart Profiles (Elgato)](https://help.elgato.com/hc/en-us/articles/360053419071-Elgato-Stream-Deck-Smart-Profiles)
28. [Leveraging a Stream Deck to assist productivity (DEV Community)](https://dev.to/documentednerd/leveraging-a-stream-deck-to-assist-productivity-4j14)
29. [Stream Deck for Productivity: 2026 Guide (Asian Efficiency)](https://www.asianefficiency.com/productivity/stream-deck-for-productivity/)
30. [Stream Deck API Request plugin (GitHub)](https://github.com/mjbnz/streamdeck-api-request)
