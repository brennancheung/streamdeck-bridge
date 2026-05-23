# Audio and Media Control

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > macOS System Automation

---

## Key Findings

- **System volume** is trivially controlled via `osascript` one-liners (0-100 scale, mute/unmute) -- no dependencies needed
- **Per-app volume** requires Apple's Audio Tap API (macOS 14.2+) and is complex; no simple Node.js wrapper exists
- **Audio device switching** is solved by `SwitchAudioSource` (Homebrew) or the `macos-audio-devices` npm package
- **Media playback** (play/pause/next/previous) works three ways: AppleScript per-app, `nowplaying-cli` (any app), or simulated media key events
- **Now playing metadata** (artist, title, album art) is available via `nowplaying-cli` or app-specific AppleScript
- **Microphone mute** works via `osascript` input volume or `SwitchAudioSource -m toggle -t input`
- **Node.js integration** is straightforward: shell out to CLI tools via `child_process.execSync()` for all of the above
- **Best high-level npm packages**: `loudness` (system volume), `macos-audio-devices` (device management), `spotify-node-applescript` (Spotify control)

---

## 1. System Volume Control

### 1.1 AppleScript / osascript (Zero Dependencies)

The simplest and most reliable approach. Works on all macOS versions.

```bash
# Set volume to 50% (range: 0-100)
osascript -e "set volume output volume 50"

# Mute (preserves volume level)
osascript -e "set volume with output muted"

# Unmute
osascript -e "set volume without output muted"

# Get current volume and mute state
osascript -e "output volume of (get volume settings)"
# Returns: 50

osascript -e "output muted of (get volume settings)"
# Returns: false

# Set volume AND mute state in one call
osascript -e "set volume without output muted output volume 75"

# Increment volume by 10
osascript -e "set volume output volume (output volume of (get volume settings) + 10)"
```

The full `set volume` syntax supports these parameters:

```applescript
set volume output volume <0-100>    -- speaker volume
set volume input volume <0-100>     -- microphone level
set volume alert volume <0-100>     -- alert sound volume
set volume with output muted        -- mute speakers
set volume without output muted     -- unmute speakers
```

### 1.2 Node.js: Direct osascript Shell-Out

No npm packages needed. This is the most portable approach.

```typescript
import { execSync } from 'child_process'

function setVolume(level: number): void {
  execSync(`osascript -e "set volume output volume ${Math.round(level)}"`)
}

function getVolume(): number {
  const result = execSync('osascript -e "output volume of (get volume settings)"')
  return parseInt(result.toString().trim(), 10)
}

function setMuted(muted: boolean): void {
  const flag = muted ? 'with' : 'without'
  execSync(`osascript -e "set volume ${flag} output muted"`)
}

function isMuted(): boolean {
  const result = execSync('osascript -e "output muted of (get volume settings)"')
  return result.toString().trim() === 'true'
}

function toggleMute(): void {
  setMuted(!isMuted())
}

// Usage
setVolume(50)
console.log(getVolume())  // 50
setMuted(true)
console.log(isMuted())    // true
```

### 1.3 Node.js: `loudness` npm Package

A cross-platform wrapper. Inactive since 2023 but still functional on macOS.

```bash
pnpm add loudness
```

```typescript
import loudness from 'loudness'

// All methods return Promises
await loudness.setVolume(45)
const vol = await loudness.getVolume()   // 45
await loudness.setMuted(true)
const muted = await loudness.getMuted()  // true
```

The API surface is minimal: `setVolume(n)`, `getVolume()`, `setMuted(bool)`, `getMuted()`. Volume is 0-100. Under the hood on macOS it calls `osascript`, so it adds no native dependencies.

### 1.4 CoreAudio (Low-Level, Swift/C)

For native addons or companion binaries. Uses `AudioObjectSetPropertyData` with a `Float32` volume value (0.0-1.0).

```swift
import CoreAudio

func setSystemVolume(_ volume: Float32) {
    var defaultDevice = AudioDeviceID(0)
    var size = UInt32(MemoryLayout<AudioDeviceID>.size)
    
    var address = AudioObjectPropertyAddress(
        mSelector: kAudioHardwarePropertyDefaultOutputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
    
    AudioObjectGetPropertyData(
        AudioObjectID(kAudioObjectSystemObject),
        &address, 0, nil, &size, &defaultDevice
    )
    
    var vol = volume
    address.mSelector = kAudioDevicePropertyVolumeScalar
    address.mScope = kAudioDevicePropertyScopeOutput
    
    // Set for both channels (left=1, right=2)
    for channel: UInt32 in 1...2 {
        address.mElement = channel
        AudioObjectSetPropertyData(
            defaultDevice, &address, 0, nil,
            UInt32(MemoryLayout<Float32>.size), &vol
        )
    }
}
```

Swift frameworks like `SimplyCoreAudio` and `ISSoundAdditions` wrap this complexity into clean APIs if building a native companion tool.

---

## 2. Per-Application Volume Control

macOS has no built-in volume mixer like Windows. Per-app volume requires Apple's **Audio Tap API** (macOS 14.2+ Sonoma).

### 2.1 Audio Tap API Overview

The Audio Tap API lets you intercept a specific process's audio stream, modify it (e.g., adjust gain), and route it. However, it is significantly more complex than system volume control:

- Requires creating an `AudioTap` on a target process
- Needs a virtual audio device to re-route tapped audio
- Must process audio in real-time via an audio render callback
- Only available on macOS 14.2+

Apps like **VolumeHub**, **SoundFlow**, and **FineTune** demonstrate working implementations.

### 2.2 Practical Recommendation for Stream Deck

Per-app volume is **not practical to implement from Node.js**. The Audio Tap API requires Swift/Objective-C and significant audio programming. Instead, consider:

1. **App-specific volume**: Many apps (Spotify, Music, browsers) expose volume via AppleScript
2. **Pre-built tools**: Use SoundSource, VolumeHub, or Background Music and automate them
3. **Focus on system volume**: For Stream Deck, system volume + per-app AppleScript covers 90% of use cases

### 2.3 App-Specific Volume via AppleScript

```bash
# Spotify volume (0-100)
osascript -e 'tell application "Spotify" to set sound volume to 50'
osascript -e 'tell application "Spotify" to sound volume'

# Music.app volume (0-100)  
osascript -e 'tell application "Music" to set sound volume to 50'

# Chrome tab audio (mute via AppleScript is not supported; use system-level tools)
```

---

## 3. Audio Device Switching

### 3.1 SwitchAudioSource (CLI Tool)

The standard CLI tool for device switching. Install via Homebrew:

```bash
brew install switchaudio-osx
```

Usage:

```bash
# List all audio devices
SwitchAudioSource -a

# List only output devices
SwitchAudioSource -a -t output

# Show current output device
SwitchAudioSource -c

# Switch to a specific device by name
SwitchAudioSource -s "MacBook Pro Speakers"

# Switch to a device by ID
SwitchAudioSource -i 74

# Cycle to the next output device
SwitchAudioSource -n

# JSON output (for parsing)
SwitchAudioSource -a -f json

# Mute/unmute/toggle the microphone input
SwitchAudioSource -m mute -t input
SwitchAudioSource -m unmute -t input
SwitchAudioSource -m toggle -t input
```

### 3.2 Node.js: `macos-audio-devices` npm Package

A full-featured Node.js library for audio device management.

```bash
pnpm add macos-audio-devices
```

```typescript
import audioDevices from 'macos-audio-devices'

// List all output devices
const outputs = audioDevices.getOutputDevices.sync()
// Returns: [{ id, uid, name, isOutput, isInput, volume, transportType }]

// Get current default output
const current = audioDevices.getDefaultOutputDevice.sync()
console.log(current.name)  // "MacBook Pro Speakers"

// Switch output device
audioDevices.setDefaultOutputDevice(outputs[1].id)

// Get/set volume on a specific device (0.0 - 1.0)
const vol = audioDevices.getOutputDeviceVolume.sync(current.id)
audioDevices.setOutputDeviceVolume(current.id, 0.5)

// List input devices
const inputs = audioDevices.getInputDevices.sync()

// Switch input device
audioDevices.setDefaultInputDevice(inputs[0].id)

// Create an aggregate device (combine multiple outputs)
const aggregate = await audioDevices.createAggregateDevice(
  'Combined Output',
  outputs[0].id,
  [outputs[1].id],
  { multiOutput: true }
)

// Destroy aggregate device
await audioDevices.destroyAggregateDevice(aggregate.id)
```

Device properties include:

| Property | Type | Description |
|----------|------|-------------|
| `id` | `number` | Unique device identifier |
| `uid` | `string` | UID for AVCaptureDevice compatibility |
| `name` | `string` | Human-readable name |
| `isOutput` | `boolean` | Is an output device |
| `isInput` | `boolean` | Is an input device |
| `volume` | `number` | Current volume (0.0-1.0, output only) |
| `transportType` | `string` | Connection type (usb, bluetooth, builtin, hdmi, etc.) |

### 3.3 Node.js: Shell Out to SwitchAudioSource

```typescript
import { execSync } from 'child_process'

function listOutputDevices(): string[] {
  const output = execSync('SwitchAudioSource -a -t output').toString()
  return output.trim().split('\n')
}

function getCurrentOutput(): string {
  return execSync('SwitchAudioSource -c').toString().trim()
}

function switchOutput(deviceName: string): void {
  execSync(`SwitchAudioSource -s "${deviceName}"`)
}

function cycleNextOutput(): void {
  execSync('SwitchAudioSource -n')
}
```

---

## 4. Media Playback Control

### 4.1 nowplaying-cli (Any App, Recommended)

The best universal approach. Controls whatever app is currently playing, using the MediaRemote private framework under the hood.

```bash
brew install nowplaying-cli
```

```bash
# Playback control (works with ANY media app)
nowplaying-cli togglePlayPause
nowplaying-cli play
nowplaying-cli pause
nowplaying-cli next
nowplaying-cli previous
nowplaying-cli seek 30    # Jump to 30 seconds

# Get current track metadata
nowplaying-cli get title artist album
# Output: "Song Title" "Artist Name" "Album Name"

# Get all available metadata
nowplaying-cli get-raw

# Get specific fields
nowplaying-cli get title artist album duration elapsedTime playbackRate

# Get album artwork (base64 encoded)
nowplaying-cli get artworkData artworkMIMEType
```

Available metadata properties:

| Property | Description |
|----------|-------------|
| `title` | Track title |
| `artist` | Artist name |
| `album` | Album name |
| `genre` | Genre |
| `duration` | Total duration in seconds |
| `elapsedTime` | Current playback position |
| `playbackRate` | 0 = paused, 1 = playing |
| `trackNumber` | Track number on album |
| `discNumber` | Disc number |
| `shuffleMode` | Shuffle state |
| `repeatMode` | Repeat state |
| `artworkData` | Base64-encoded artwork bytes |
| `artworkMIMEType` | MIME type of artwork (e.g., image/jpeg) |
| `artworkDataWidth` | Artwork width in pixels |
| `artworkDataHeight` | Artwork height in pixels |
| `uniqueIdentifier` | Unique ID of the playing item |
| `isMusicApp` | Whether the source is Apple Music |

Node.js wrapper:

```typescript
import { execSync } from 'child_process'

interface NowPlaying {
  title: string
  artist: string
  album: string
  isPlaying: boolean
  duration: number
  elapsed: number
}

function getNowPlaying(): NowPlaying {
  const raw = execSync(
    'nowplaying-cli get title artist album playbackRate duration elapsedTime'
  ).toString().trim()
  
  // Output is newline-separated values
  const [title, artist, album, playbackRate, duration, elapsed] = raw.split('\n')
  
  return {
    title: title || '',
    artist: artist || '',
    album: album || '',
    isPlaying: parseFloat(playbackRate) > 0,
    duration: parseFloat(duration) || 0,
    elapsed: parseFloat(elapsed) || 0,
  }
}

function mediaControl(action: 'play' | 'pause' | 'togglePlayPause' | 'next' | 'previous'): void {
  execSync(`nowplaying-cli ${action}`)
}

function getAlbumArt(): Buffer | null {
  try {
    const base64 = execSync('nowplaying-cli get artworkData').toString().trim()
    if (!base64 || base64 === 'null') return null
    return Buffer.from(base64, 'base64')
  } catch {
    return null
  }
}

// Save album art to file for Stream Deck display
function saveAlbumArt(filePath: string): boolean {
  const art = getAlbumArt()
  if (!art) return false
  const fs = await import('fs')
  fs.writeFileSync(filePath, art)
  return true
}
```

### 4.2 App-Specific AppleScript Control

When you need to target a specific app:

```bash
# --- Spotify ---
osascript -e 'tell application "Spotify" to playpause'
osascript -e 'tell application "Spotify" to next track'
osascript -e 'tell application "Spotify" to previous track'
osascript -e 'tell application "Spotify" to set player position to 30'

# Get Spotify track info
osascript -e 'tell application "Spotify"
  set trackName to name of current track
  set trackArtist to artist of current track
  set trackAlbum to album of current track
  set artURL to artwork url of current track
  return trackName & "|" & trackArtist & "|" & trackAlbum & "|" & artURL
end tell'

# --- Apple Music ---
osascript -e 'tell application "Music" to playpause'
osascript -e 'tell application "Music" to next track'
osascript -e 'tell application "Music" to previous track'

# Get Music.app track info
osascript -e 'tell application "Music"
  set trackName to name of current track
  set trackArtist to artist of current track
  set trackAlbum to album of current track
  return trackName & "|" & trackArtist & "|" & trackAlbum
end tell'

# Check which app is playing
osascript -e 'tell application "Spotify" to player state'
# Returns: "playing", "paused", or "stopped"
```

### 4.3 `spotify-node-applescript` npm Package

Full-featured Spotify control from Node.js:

```bash
pnpm add spotify-node-applescript
```

```typescript
import spotify from 'spotify-node-applescript'

// Playback control
spotify.playPause(() => {})
spotify.next(() => {})
spotify.previous(() => {})
spotify.jumpTo(30, () => {})  // Seek to 30 seconds

// Volume control (Spotify's own volume, not system)
spotify.setVolume(50, () => {})
spotify.volumeUp(() => {})
spotify.volumeDown(() => {})
spotify.muteVolume(() => {})
spotify.unmuteVolume(() => {})

// Get current track info
spotify.getTrack((err, track) => {
  console.log(track)
  // {
  //   artist: 'Artist Name',
  //   album: 'Album Name',
  //   disc_number: 1,
  //   duration: 256,
  //   played_count: 0,
  //   track_number: 1,
  //   popularity: 72,
  //   id: 'spotify:track:...',
  //   name: 'Song Title',
  //   album_artist: 'Album Artist',
  //   artwork_url: 'https://i.scdn.co/image/...',
  //   spotify_url: 'spotify:track:...'
  // }
})

// Get player state
spotify.getState((err, state) => {
  console.log(state)
  // { volume: 50, position: 15, state: 'playing' }
})

// Play specific track
spotify.playTrack('spotify:track:3DYVWvPh3kGwPasp7yjahc', () => {})

// Toggle shuffle/repeat
spotify.toggleShuffling(() => {})
spotify.toggleRepeating(() => {})

// Check if Spotify is running
spotify.isRunning((err, running) => {
  console.log(running)  // true
})
```

---

## 5. Simulating Media Keys

For controlling whatever app is currently the "Now Playing" target without targeting a specific app. This is how physical keyboard media keys work.

### 5.1 Python (via Quartz Framework)

```python
#!/usr/bin/env python3
import Quartz
import sys

# Key type constants from IOKit/hidsystem/ev_keymap.h
NX_KEYTYPE_SOUND_UP = 0
NX_KEYTYPE_SOUND_DOWN = 1
NX_KEYTYPE_BRIGHTNESS_UP = 2
NX_KEYTYPE_BRIGHTNESS_DOWN = 3
NX_KEYTYPE_MUTE = 7
NX_KEYTYPE_PLAY = 16
NX_KEYTYPE_NEXT = 17
NX_KEYTYPE_PREVIOUS = 18
NX_KEYTYPE_FAST = 19
NX_KEYTYPE_REWIND = 20

NSSystemDefined = 14

def simulate_media_key(key: int):
    """Simulate a media key press and release."""
    for down in (True, False):
        event = Quartz.NSEvent.otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2_(
            NSSystemDefined,       # type
            (0, 0),                # location
            0xa00 if down else 0xb00,  # flags (key down / key up)
            0,                     # timestamp
            0,                     # window
            0,                     # context
            8,                     # subtype (NX_SUBTYPE_AUX_CONTROL_BUTTONS)
            (key << 16) | ((0xa if down else 0xb) << 8),  # data1
            -1                     # data2
        )
        Quartz.CGEventPost(0, event.CGEvent())  # 0 = kCGHIDEventTap

COMMANDS = {
    'play':    NX_KEYTYPE_PLAY,
    'next':    NX_KEYTYPE_NEXT,
    'prev':    NX_KEYTYPE_PREVIOUS,
    'volup':   NX_KEYTYPE_SOUND_UP,
    'voldown': NX_KEYTYPE_SOUND_DOWN,
    'mute':    NX_KEYTYPE_MUTE,
}

if __name__ == '__main__':
    cmd = sys.argv[1] if len(sys.argv) > 1 else None
    if cmd in COMMANDS:
        simulate_media_key(COMMANDS[cmd])
    else:
        print(f"Usage: {sys.argv[0]} {{{','.join(COMMANDS.keys())}}}")
```

### 5.2 Swift (Compiled Helper Binary)

```swift
import Cocoa

enum MediaKey: UInt32 {
    case play     = 16  // NX_KEYTYPE_PLAY
    case next     = 17  // NX_KEYTYPE_NEXT
    case previous = 18  // NX_KEYTYPE_PREVIOUS
    case soundUp  = 0   // NX_KEYTYPE_SOUND_UP
    case soundDown = 1  // NX_KEYTYPE_SOUND_DOWN
    case mute     = 7   // NX_KEYTYPE_MUTE
}

func simulateMediaKey(_ key: MediaKey) {
    func postEvent(keyDown: Bool) {
        let flags = keyDown ? UInt64(0xa00) : UInt64(0xb00)
        let data1 = Int((key.rawValue << 16)) | Int((keyDown ? 0xa : 0xb) << 8)
        
        let event = NSEvent.otherEvent(
            with: .systemDefined,
            location: .zero,
            modifierFlags: NSEvent.ModifierFlags(rawValue: flags),
            timestamp: 0,
            windowNumber: 0,
            context: nil,
            subtype: 8,   // NX_SUBTYPE_AUX_CONTROL_BUTTONS
            data1: data1,
            data2: -1
        )
        
        if let cgEvent = event?.cgEvent {
            cgEvent.post(tap: .cghidEventTap)
        }
    }
    
    postEvent(keyDown: true)
    postEvent(keyDown: false)
}

// Usage
simulateMediaKey(.play)
simulateMediaKey(.next)
```

### 5.3 Node.js: Shell Out to Python Script

```typescript
import { execSync } from 'child_process'

function simulateMediaKey(key: 'play' | 'next' | 'prev' | 'volup' | 'voldown' | 'mute'): void {
  execSync(`python3 /path/to/media_keys.py ${key}`)
}
```

**Important caveat**: Simulated media key events via `CGEventPost` are ignored inside sandboxed apps. A Stream Deck plugin running outside the sandbox (or a standalone Node.js process) will work fine.

---

## 6. MediaRemote Private Framework

The framework behind `nowplaying-cli` and macOS's Now Playing widget. It communicates with `mediaserverd`.

### 6.1 Key Functions

```c
// Send a playback command
Boolean MRMediaRemoteSendCommand(MRCommand command, id userInfo);

// Get now playing info (async)
void MRMediaRemoteGetNowPlayingInfo(
    dispatch_queue_t queue,
    void (^completion)(CFDictionaryRef info)
);

// Check if something is playing (async)
void MRMediaRemoteGetNowPlayingApplicationIsPlaying(
    dispatch_queue_t queue,
    void (^completion)(Boolean isPlaying)
);

// Register for now playing change notifications
void MRMediaRemoteRegisterForNowPlayingNotifications(dispatch_queue_t queue);

// Other functions
void MRMediaRemoteSetElapsedTime(double elapsedTime);
void MRMediaRemoteSetShuffleMode(int mode);
void MRMediaRemoteSetRepeatMode(int mode);
void MRMediaRemoteSetPlaybackSpeed(int speed);
```

### 6.2 MRCommand Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `kMRPlay` | 0 | Start playback |
| `kMRPause` | 1 | Pause playback |
| `kMRTogglePlayPause` | 2 | Toggle play/pause |
| `kMRStop` | 3 | Stop playback |
| `kMRNextTrack` | 4 | Next track |
| `kMRPreviousTrack` | 5 | Previous track |
| `kMRToggleShuffle` | 6 | Toggle shuffle |
| `kMRToggleRepeat` | 7 | Toggle repeat |
| `kMRStartForwardSeek` | 8 | Begin fast-forward |
| `kMREndForwardSeek` | 9 | End fast-forward |
| `kMRStartBackwardSeek` | 10 | Begin rewind |
| `kMREndBackwardSeek` | 11 | End rewind |
| `kMRGoBackFifteenSeconds` | 12 | Skip back 15s |
| `kMRSkipFifteenSeconds` | 13 | Skip ahead 15s |
| `kMRLikeTrack` | 0x6A | Like current track |
| `kMRBanTrack` | 0x6B | Ban current track |

### 6.3 Now Playing Info Keys

```
kMRMediaRemoteNowPlayingInfoTitle
kMRMediaRemoteNowPlayingInfoArtist
kMRMediaRemoteNowPlayingInfoAlbum
kMRMediaRemoteNowPlayingInfoArtworkData
kMRMediaRemoteNowPlayingInfoArtworkMIMEType
kMRMediaRemoteNowPlayingInfoDuration
kMRMediaRemoteNowPlayingInfoElapsedTime
kMRMediaRemoteNowPlayingInfoPlaybackRate
kMRMediaRemoteNowPlayingInfoGenre
kMRMediaRemoteNowPlayingInfoComposer
kMRMediaRemoteNowPlayingInfoTrackNumber
kMRMediaRemoteNowPlayingInfoTotalTrackCount
kMRMediaRemoteNowPlayingInfoDiscNumber
kMRMediaRemoteNowPlayingInfoTotalDiscCount
kMRMediaRemoteNowPlayingInfoRepeatMode
kMRMediaRemoteNowPlayingInfoShuffleMode
kMRMediaRemoteNowPlayingInfoTimestamp
kMRMediaRemoteNowPlayingInfoUniqueIdentifier
kMRMediaRemoteNowPlayingInfoIsMusicApp
```

### 6.4 Practical Note

You do not need to use MediaRemote directly. `nowplaying-cli` wraps it into a CLI tool. For Node.js Stream Deck buttons, shelling out to `nowplaying-cli` is the recommended approach. Direct MediaRemote usage requires building a native Swift/C binary.

---

## 7. Microphone Mute/Unmute

### 7.1 AppleScript (Simple but Imperfect)

```bash
# Mute microphone (set input volume to 0)
osascript -e "set volume input volume 0"

# Unmute microphone (set input volume to 100)
osascript -e "set volume input volume 100"

# Get current input volume
osascript -e "input volume of (get volume settings)"

# Toggle mute/unmute
osascript -e 'set vol to input volume of (get volume settings)
if vol > 0 then
  set volume input volume 0
else
  set volume input volume 100
end if'
```

**Known limitation**: Setting input volume to 0 via AppleScript shows the slider at 0, but the microphone may still pick up sound. The system-level mute is more reliable.

### 7.2 SwitchAudioSource (More Reliable)

```bash
# Toggle microphone mute
SwitchAudioSource -m toggle -t input

# Explicitly mute
SwitchAudioSource -m mute -t input

# Explicitly unmute
SwitchAudioSource -m unmute -t input
```

This uses a proper device-level mute rather than just setting volume to zero.

### 7.3 Node.js Microphone Control

```typescript
import { execSync } from 'child_process'

function toggleMicMute(): void {
  execSync('SwitchAudioSource -m toggle -t input')
}

function setMicMuted(muted: boolean): void {
  execSync(`SwitchAudioSource -m ${muted ? 'mute' : 'unmute'} -t input`)
}

// Fallback using osascript if SwitchAudioSource is not installed
function setMicMutedFallback(muted: boolean): void {
  const vol = muted ? 0 : 100
  execSync(`osascript -e "set volume input volume ${vol}"`)
}

function getMicVolume(): number {
  const result = execSync('osascript -e "input volume of (get volume settings)"')
  return parseInt(result.toString().trim(), 10)
}
```

---

## 8. Album Art for Stream Deck Display

### 8.1 Using nowplaying-cli

```typescript
import { execSync } from 'child_process'
import { writeFileSync, mkdirSync } from 'fs'
import { join } from 'path'

const CACHE_DIR = '/tmp/streamdeck-album-art'

function getAlbumArtBuffer(): Buffer | null {
  try {
    const raw = execSync('nowplaying-cli get artworkData').toString().trim()
    if (!raw || raw === 'null' || raw === '') return null
    return Buffer.from(raw, 'base64')
  } catch {
    return null
  }
}

function getAlbumArtPath(): string | null {
  const buf = getAlbumArtBuffer()
  if (!buf) return null
  
  mkdirSync(CACHE_DIR, { recursive: true })
  
  // Get MIME type for extension
  let ext = 'jpg'
  try {
    const mime = execSync('nowplaying-cli get artworkMIMEType').toString().trim()
    if (mime.includes('png')) ext = 'png'
  } catch {}
  
  const filePath = join(CACHE_DIR, `current.${ext}`)
  writeFileSync(filePath, buf)
  return filePath
}
```

### 8.2 Using Spotify AppleScript

Spotify exposes artwork URLs directly:

```typescript
import { execSync } from 'child_process'

function getSpotifyArtworkUrl(): string | null {
  try {
    const url = execSync(
      `osascript -e 'tell application "Spotify" to artwork url of current track'`
    ).toString().trim()
    return url || null
  } catch {
    return null
  }
}

// Spotify artwork URLs are HTTPS links to images on scdn.co
// You can fetch and resize them for Stream Deck (72x72 or 96x96 px)
```

### 8.3 Using Apple Music AppleScript

```typescript
function getMusicArtwork(savePath: string): boolean {
  try {
    execSync(`osascript -e '
      tell application "Music"
        set artData to raw data of artwork 1 of current track
      end tell
      set fileRef to open for access POSIX file "${savePath}" with write permission
      write artData to fileRef
      close access fileRef
    '`)
    return true
  } catch {
    return false
  }
}
```

---

## 9. Complete Stream Deck Audio Module

A unified module combining all the above for Stream Deck button handlers:

```typescript
import { execSync } from 'child_process'

// ─── System Volume ─────────────────────────────────

export function setSystemVolume(level: number): void {
  const clamped = Math.max(0, Math.min(100, Math.round(level)))
  execSync(`osascript -e "set volume output volume ${clamped}"`)
}

export function getSystemVolume(): number {
  return parseInt(
    execSync('osascript -e "output volume of (get volume settings)"').toString().trim(),
    10
  )
}

export function toggleSystemMute(): void {
  const muted = execSync('osascript -e "output muted of (get volume settings)"')
    .toString().trim() === 'true'
  execSync(`osascript -e "set volume ${muted ? 'without' : 'with'} output muted"`)
}

// ─── Media Playback ────────────────────────────────

export function mediaPlayPause(): void {
  execSync('nowplaying-cli togglePlayPause')
}

export function mediaNext(): void {
  execSync('nowplaying-cli next')
}

export function mediaPrevious(): void {
  execSync('nowplaying-cli previous')
}

// ─── Now Playing Info ──────────────────────────────

export interface TrackInfo {
  title: string
  artist: string
  album: string
  isPlaying: boolean
}

export function getNowPlaying(): TrackInfo {
  const raw = execSync('nowplaying-cli get title artist album playbackRate')
    .toString().trim().split('\n')
  return {
    title: raw[0] || '',
    artist: raw[1] || '',
    album: raw[2] || '',
    isPlaying: parseFloat(raw[3] || '0') > 0,
  }
}

// ─── Audio Device Switching ────────────────────────

export function cycleOutputDevice(): void {
  execSync('SwitchAudioSource -n')
}

export function setOutputDevice(name: string): void {
  execSync(`SwitchAudioSource -s "${name}"`)
}

export function getCurrentOutputDevice(): string {
  return execSync('SwitchAudioSource -c').toString().trim()
}

// ─── Microphone ────────────────────────────────────

export function toggleMicMute(): void {
  execSync('SwitchAudioSource -m toggle -t input')
}
```

---

## 10. Required Tool Installation

```bash
# All Homebrew tools needed for the module above
brew install switchaudio-osx nowplaying-cli

# npm packages (optional, for richer device management)
pnpm add loudness macos-audio-devices spotify-node-applescript
```

No installation needed for `osascript` -- it ships with macOS.

---

## 11. Comparison Matrix

| Capability | osascript | nowplaying-cli | SwitchAudioSource | loudness (npm) | macos-audio-devices (npm) |
|------------|-----------|----------------|-------------------|----------------|---------------------------|
| System volume | Yes (0-100) | No | No | Yes (0-100) | Yes (0.0-1.0) |
| Mute/unmute | Yes | No | Yes | Yes | No |
| Play/pause | App-specific | Any app | No | No | No |
| Next/previous | App-specific | Any app | No | No | No |
| Track info | App-specific | Any app | No | No | No |
| Album art | App-specific | Base64 data | No | No | No |
| Device switch | No | No | Yes | No | Yes |
| Device list | No | No | Yes | No | Yes |
| Mic mute | Partial* | No | Yes | No | No |
| Per-app volume | App-specific | No | No | No | No |
| Install needed | Built-in | Homebrew | Homebrew | npm | npm |

*AppleScript mic mute sets input volume to 0 but may not fully mute hardware.

---

## Sources

1. [How to control OS X System Volume with AppleScript](https://coolaj86.com/articles/how-to-control-os-x-system-volume-with-applescript/)
2. [Adjusting a Mac's System Volume on the Command Line](https://excessivelyadequate.com/posts/vol.html)
3. [SwitchAudioSource (switchaudio-osx) - GitHub](https://github.com/deweller/switchaudio-osx)
4. [nowplaying-cli - GitHub](https://github.com/kirtan-shah/nowplaying-cli)
5. [node-loudness / loudness npm package - GitHub](https://github.com/LinusU/node-loudness)
6. [macos-audio-devices npm package - GitHub](https://github.com/karaggeorge/macos-audio-devices)
7. [spotify-node-applescript - GitHub](https://github.com/andrehaveman/spotify-node-applescript)
8. [MediaRemote.framework - The Apple Wiki](https://theapplewiki.com/wiki/Dev:MediaRemote.framework)
9. [MediaRemote.h header - GitHub (Helius)](https://github.com/s1ris/Helius/blob/master/MediaRemote.h)
10. [macOS Media Key Press Emulation - GitHub Gist](https://gist.github.com/storoj/429e260f697594d42337843b3d523bf5)
11. [Python media keys script - GitHub Gist](https://gist.github.com/4078034)
12. [Silently mute the mic input via AppleScript - The Robservatory](https://robservatory.com/silently-mute-the-mic-input-via-applescript/)
13. [Apple's Audio Tap API - Developer Documentation](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps)
14. [SimplyCoreAudio Swift framework - GitHub](https://github.com/rnine/SimplyCoreAudio)
15. [ISSoundAdditions - GitHub](https://github.com/InerziaSoft/ISSoundAdditions)
16. [mac-volume CLI tool - Rob Allen](https://akrabat.com/mac-volume-a-cli-to-control-device-volume/)
17. [node-applescript - GitHub](https://github.com/TooTallNate/node-applescript)
18. [node-osascript - npm](https://www.npmjs.com/package/node-osascript)
19. [Toggle microphone on macOS - Rogerio Vicente](https://rogeriopvl.com/posts/202405210-toggle-microphone-on-macos/)
20. [Stream Deck Plugin Environment - Elgato Docs](https://docs.elgato.com/streamdeck/sdk/introduction/plugin-environment/)
