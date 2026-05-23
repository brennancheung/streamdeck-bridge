# Animation Techniques

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Image Update Performance

---

## Key Findings

- The Stream Deck XL can achieve roughly **5-10 fps for single-key animation** and **3-5 fps for full-deck refresh** (all 32 keys), constrained by USB HID transfer and JPEG encoding time.
- **Prepared buffers** (`prepareFillKeyBuffer` / `sendPreparedBuffer`) are the single most important optimization -- they pre-encode JPEG once and skip compression on subsequent sends.
- **GIF playback** requires decoding frames upfront with a library like `gifuct-js`, converting each frame to an RGB buffer, pre-encoding all frames as prepared buffers, and then cycling through them on a timer.
- **Multi-button spanning** works via `fillPanelBuffer` or manual tiling with gap-aware cropping. Animated spans require slicing each animation frame into 32 tiles and sending only the tiles that changed.
- **Satori + resvg-js** is the most promising rendering pipeline for status displays: write JSX/HTML-like templates, render to SVG, convert to PNG/raw buffer -- all without a browser, in under 5ms per frame at 96x96.
- **Sharp** excels at image manipulation (resize, composite, format conversion) but cannot draw text or shapes directly. Use it for the final JPEG encoding step or combine with SVG overlays for text.
- **@napi-rs/canvas** provides a full Canvas 2D API in Node.js without system dependencies, making it ideal for procedural animations (gauges, waveforms, progress bars) at ~68 ops/sec.
- **Dirty tracking** (only updating buttons whose content changed) is essential for any status dashboard. Buffer comparison or hash-based change detection avoids wasting USB bandwidth on unchanged buttons.
- Install **`@julusian/jpeg-turbo`** -- it makes JPEG encoding 5-10x faster than the pure-JS `jpeg-js` fallback, which is critical for animation workloads.

---

## 1. Hardware Constraints for Animation

Understanding the physical limits is essential before choosing a rendering approach.

### Transfer Budget

| Parameter | Value |
|---|---|
| Key resolution | 96 x 96 pixels |
| Image format on wire | JPEG (quality 100 recommended) |
| Typical JPEG size | 2-5 KB per key |
| HID report size | 1024 bytes (8-byte header + 1016 payload) |
| Reports per key update | 3-5 |
| Estimated time per key update | ~5-15 ms |
| Full 32-key refresh | ~100-300 ms (3-10 fps) |
| Single-key animation ceiling | ~60-200 updates/sec (theoretical), ~15-30 fps (practical) |

### Bottleneck Analysis

The bottleneck shifts depending on what you are doing:

1. **Single key, pre-encoded frames**: USB transfer dominates (~5-15 ms per update). Achievable: 15-30 fps.
2. **Single key, live rendering**: JPEG encoding dominates unless using jpeg-turbo. With jpeg-turbo: 15-25 fps. Without: 5-10 fps.
3. **Full deck refresh**: Serialized USB writes dominate. 32 keys x 5-15 ms = 160-480 ms. Achievable: 2-6 fps.
4. **Selective update (e.g., 4 of 32 keys)**: 4 x 5-15 ms = 20-60 ms. Achievable: 15-30 fps.

The takeaway: animate sparingly, update selectively, and pre-encode whenever possible.

---

## 2. GIF Playback on Buttons

### Decoding GIF Frames

Use `gifuct-js` to parse animated GIFs into individual frames with timing data:

```typescript
import { parseGIF, decompressFrames } from 'gifuct-js'
import { readFile } from 'fs/promises'

interface DecodedFrame {
  imageData: Uint8ClampedArray  // RGBA pixel data
  delay: number                 // Frame delay in ms
  width: number
  height: number
}

async function decodeGif(filePath: string): Promise<DecodedFrame[]> {
  const buffer = await readFile(filePath)
  const gif = parseGIF(buffer.buffer as ArrayBuffer)
  // buildPatch=true creates canvas-ready RGBA arrays
  const frames = decompressFrames(gif, true)

  return frames.map((frame) => ({
    imageData: frame.patch,
    delay: frame.delay * 10,  // GIF delay is in centiseconds
    width: frame.dims.width,
    height: frame.dims.height,
  }))
}
```

### Pre-Encoding All Frames

The critical optimization: encode every GIF frame to a prepared buffer at startup, not during playback.

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'
import sharp from 'sharp'
import type { StreamDeck } from '@elgato-stream-deck/node'

async function prepareGifFrames(
  deck: StreamDeck,
  keyIndex: number,
  gifPath: string
) {
  const frames = await decodeGif(gifPath)
  const size = 96  // XL key size

  const prepared = await Promise.all(
    frames.map(async (frame) => {
      // Resize RGBA frame to 96x96 using sharp
      const rgbBuffer = await sharp(Buffer.from(frame.imageData.buffer), {
        raw: { width: frame.width, height: frame.height, channels: 4 },
      })
        .resize(size, size)
        .flatten({ background: '#000000' })
        .raw()
        .toBuffer()

      // Pre-encode to JPEG (expensive step -- done once)
      const preparedBuffer = await deck.prepareFillKeyBuffer(
        keyIndex,
        rgbBuffer,
        { format: 'rgb' }
      )
      return { prepared: preparedBuffer, delay: frame.delay }
    })
  )

  return prepared
}
```

### Playback Loop with Accurate Timing

Use monotonic time to avoid drift, following the pattern from the python-elgato-streamdeck reference implementation:

```typescript
async function playGifOnKey(
  deck: StreamDeck,
  keyIndex: number,
  gifPath: string
) {
  const frames = await prepareGifFrames(deck, keyIndex, gifPath)
  let frameIndex = 0
  let nextFrameTime = performance.now()

  const tick = async () => {
    const frame = frames[frameIndex]
    await deck.sendPreparedBuffer(frame.prepared)

    frameIndex = (frameIndex + 1) % frames.length
    nextFrameTime += frame.delay

    // Calculate sleep, skipping frames if behind schedule
    const now = performance.now()
    const sleepMs = nextFrameTime - now
    if (sleepMs > 0) {
      setTimeout(tick, sleepMs)
    } else {
      // Behind schedule -- send next frame immediately
      setImmediate(tick)
    }
  }

  tick()
}
```

---

## 3. Multi-Button Spanning Animations

### How Tiling Works

The Stream Deck XL has physical gaps (bezels) between buttons. When spanning an image across the full 8x4 grid, you must account for the hidden pixels between keys.

```
  Full panel image (with gaps accounted for):
  +------+--+------+--+------+--+------+--+------+--+------+--+------+--+------+
  | 96px |gap| 96px |gap| 96px |gap| 96px |gap| 96px |gap| 96px |gap| 96px |gap| 96px |
  +------+--+------+--+------+--+------+--+------+--+------+--+------+--+------+
  |      |  |      |  |      |  |      |  |      |  |      |  |      |  |      |
  +------+--+------+--+------+--+------+--+------+--+------+--+------+--+------+
```

### Using fillPanelBuffer (Simple Approach)

The library's `fillPanelBuffer` handles tiling automatically. It accepts a single large buffer and splits it across all keys:

```typescript
import sharp from 'sharp'

async function animateFullPanel(deck: StreamDeck) {
  const dims = deck.calculateFillPanelDimensions()
  // XL: { width: 768, height: 384 } (96*8 x 96*4)

  // Render each animation frame as a full panel
  const frameBuffers = await Promise.all(
    animationFrames.map(async (sourceImage) => {
      const buffer = await sharp(sourceImage)
        .resize(dims.width, dims.height)
        .flatten()
        .raw()
        .toBuffer()

      // Pre-encode splits into 32 prepared buffers (one per key)
      return deck.prepareFillPanelBuffer(buffer, { format: 'rgb' })
    })
  )

  // Playback: send all 32 tiles per frame
  let frameIndex = 0
  setInterval(async () => {
    const tiles = frameBuffers[frameIndex]
    await Promise.all(tiles.map((tile) => deck.sendPreparedBuffer(tile)))
    frameIndex = (frameIndex + 1) % frameBuffers.length
  }, 200)  // 5 fps is realistic for full-deck
}
```

### Manual Tiling with Gap-Aware Cropping

For partial updates (only refreshing keys that changed), implement manual tiling:

```typescript
function tileImageForKeys(
  fullImage: Buffer,
  fullWidth: number,
  fullHeight: number,
  keySize: number,
  cols: number,
  rows: number,
  gapX: number,  // Pixels hidden by bezel horizontally
  gapY: number   // Pixels hidden by bezel vertically
): Map<number, Buffer> {
  const tiles = new Map<number, Buffer>()

  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const keyIndex = row * cols + col
      const startX = col * (keySize + gapX)
      const startY = row * (keySize + gapY)

      // Extract the tile from the full image
      // (use sharp.extract() or manual buffer slicing)
      tiles.set(keyIndex, extractRegion(
        fullImage, fullWidth, startX, startY, keySize, keySize
      ))
    }
  }

  return tiles
}
```

---

## 4. Rendering Pipelines Compared

### Pipeline A: Sharp + SVG Text (Best for Simple Status Displays)

Sharp cannot draw text natively, but you can composite SVG text overlays onto generated images. This is fast and dependency-light.

```typescript
import sharp from 'sharp'

async function renderStatusButton(
  label: string,
  value: string,
  color: string
): Promise<Buffer> {
  const size = 96

  // Create SVG with text
  const svg = `
    <svg width="${size}" height="${size}" xmlns="http://www.w3.org/2000/svg">
      <rect width="${size}" height="${size}" fill="#1a1a2e"/>
      <text x="${size / 2}" y="32" font-family="Arial" font-size="12"
            fill="#888" text-anchor="middle">${label}</text>
      <text x="${size / 2}" y="62" font-family="Arial" font-size="22"
            fill="${color}" text-anchor="middle" font-weight="bold">${value}</text>
    </svg>`

  return sharp(Buffer.from(svg))
    .resize(size, size)
    .flatten()
    .raw()
    .toBuffer()
}
```

**Performance**: ~0.5-2 ms per 96x96 image. No external dependencies beyond sharp.

**Limitations**: SVG text rendering in sharp (via librsvg/libvips) has limited font support. Custom fonts require system installation or embedding. Complex layouts are difficult.

### Pipeline B: Satori + resvg-js (Best for Complex Layouts)

Satori converts JSX to SVG, and resvg-js renders SVG to PNG. This gives you flexbox layout, font loading, and a React-like component model -- all without a browser.

```typescript
import satori from 'satori'
import { Resvg } from '@resvg/resvg-js'
import { readFile } from 'fs/promises'

// Load font once at startup
const fontData = await readFile('/path/to/Inter-Regular.ttf')

async function renderWithSatori(
  label: string,
  value: string,
  barPercent: number
): Promise<Buffer> {
  const svg = await satori(
    {
      type: 'div',
      props: {
        style: {
          width: 96, height: 96,
          display: 'flex', flexDirection: 'column',
          alignItems: 'center', justifyContent: 'center',
          backgroundColor: '#1a1a2e', color: 'white',
          fontFamily: 'Inter',
        },
        children: [
          { type: 'div', props: {
            style: { fontSize: 11, color: '#888' },
            children: label
          }},
          { type: 'div', props: {
            style: { fontSize: 20, fontWeight: 'bold', marginTop: 4 },
            children: value
          }},
          { type: 'div', props: {
            style: {
              width: 72, height: 6, backgroundColor: '#333',
              borderRadius: 3, marginTop: 8, overflow: 'hidden',
            },
            children: [
              { type: 'div', props: {
                style: {
                  width: `${barPercent}%`, height: '100%',
                  backgroundColor: barPercent > 80 ? '#e74c3c' : '#2ecc71',
                },
              }},
            ],
          }},
        ],
      },
    },
    {
      width: 96,
      height: 96,
      fonts: [{ name: 'Inter', data: fontData, style: 'normal' }],
    }
  )

  const resvg = new Resvg(svg, { fitTo: { mode: 'width', value: 96 } })
  return resvg.render().asPng()
}
```

**Performance**: ~3-8 ms per frame (Satori ~2-4 ms, resvg ~1-4 ms). Much faster than Puppeteer.

**Trade-off**: You get flexbox layout and proper font rendering, but Satori only supports a subset of CSS. No `position: absolute`, no CSS Grid, no animations (obviously -- it is static rendering).

### Pipeline C: @napi-rs/canvas (Best for Procedural/Generative Animation)

Full Canvas 2D API, ideal for drawing gauges, waveforms, charts, and custom graphics procedurally.

```typescript
import { createCanvas } from '@napi-rs/canvas'

function renderGauge(value: number, max: number): Buffer {
  const size = 96
  const canvas = createCanvas(size, size)
  const ctx = canvas.getContext('2d')

  // Background
  ctx.fillStyle = '#1a1a2e'
  ctx.fillRect(0, 0, size, size)

  // Arc gauge
  const centerX = size / 2
  const centerY = size / 2 + 8
  const radius = 34
  const startAngle = 0.75 * Math.PI
  const endAngle = 2.25 * Math.PI
  const valueAngle = startAngle + (value / max) * (endAngle - startAngle)

  // Background arc
  ctx.beginPath()
  ctx.arc(centerX, centerY, radius, startAngle, endAngle)
  ctx.strokeStyle = '#333'
  ctx.lineWidth = 6
  ctx.lineCap = 'round'
  ctx.stroke()

  // Value arc
  ctx.beginPath()
  ctx.arc(centerX, centerY, radius, startAngle, valueAngle)
  ctx.strokeStyle = value / max > 0.8 ? '#e74c3c' : '#2ecc71'
  ctx.lineWidth = 6
  ctx.lineCap = 'round'
  ctx.stroke()

  // Center text
  ctx.fillStyle = 'white'
  ctx.font = 'bold 20px sans-serif'
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(`${Math.round(value)}%`, centerX, centerY)

  // Return raw RGBA buffer
  return canvas.toBuffer('raw')
}
```

**Performance**: ~1-3 ms per 96x96 frame. Zero system dependencies (prebuilt Skia binaries via N-API).

**Best for**: CPU gauges, audio waveforms, progress indicators, spinning loaders, animated charts -- anything where you need pixel-level control and fast iteration.

### Pipeline D: Puppeteer / Headless Chrome (Heavyweight, Full CSS)

Launches a headless Chromium instance and screenshots HTML/CSS at the target resolution. Full CSS support including gradients, shadows, complex typography.

```typescript
import puppeteer from 'puppeteer'

// Launch once, reuse for all renders
const browser = await puppeteer.launch({ headless: true })
const page = await browser.newPage()
await page.setViewport({ width: 96, height: 96 })

async function renderHtml(html: string): Promise<Buffer> {
  await page.setContent(html)
  return await page.screenshot({ type: 'png', omitBackground: false })
}
```

**Performance**: ~50-200 ms per frame (page load + screenshot). Unacceptable for animation. Chromium consumes ~200-500 MB of RAM.

**Use case**: One-time generation of complex static icons. Not viable for animated or frequently-updating content.

### Pipeline Comparison Summary

| Pipeline | Render Time | RAM | Layout | Text | Best For |
|---|---|---|---|---|---|
| Sharp + SVG | 0.5-2 ms | ~30 MB | None (manual) | Basic SVG | Simple status values |
| Satori + resvg | 3-8 ms | ~50 MB | Flexbox | Full fonts | Status cards, dashboards |
| @napi-rs/canvas | 1-3 ms | ~40 MB | Canvas 2D | Canvas text | Gauges, charts, waveforms |
| Puppeteer | 50-200 ms | ~500 MB | Full CSS | Full CSS | Static icon generation |

---

## 5. Procedural and Generative Animations

### Progress Bar

```typescript
import { createCanvas } from '@napi-rs/canvas'

function renderProgressBar(percent: number, label: string): Buffer {
  const canvas = createCanvas(96, 96)
  const ctx = canvas.getContext('2d')

  ctx.fillStyle = '#1a1a2e'
  ctx.fillRect(0, 0, 96, 96)

  // Label
  ctx.fillStyle = '#aaa'
  ctx.font = '11px sans-serif'
  ctx.textAlign = 'center'
  ctx.fillText(label, 48, 28)

  // Percentage
  ctx.fillStyle = 'white'
  ctx.font = 'bold 22px sans-serif'
  ctx.fillText(`${Math.round(percent)}%`, 48, 54)

  // Bar background
  ctx.fillStyle = '#333'
  ctx.fillRect(12, 66, 72, 8)

  // Bar fill
  const barColor = percent > 90 ? '#e74c3c' : percent > 70 ? '#f39c12' : '#2ecc71'
  ctx.fillStyle = barColor
  ctx.fillRect(12, 66, 72 * (percent / 100), 8)

  return canvas.toBuffer('raw')
}
```

### Spinning Indicator

```typescript
function renderSpinner(angle: number): Buffer {
  const canvas = createCanvas(96, 96)
  const ctx = canvas.getContext('2d')

  ctx.fillStyle = '#1a1a2e'
  ctx.fillRect(0, 0, 96, 96)

  const cx = 48, cy = 48, radius = 30
  const segments = 8

  for (let i = 0; i < segments; i++) {
    const segAngle = (i / segments) * Math.PI * 2 + angle
    const opacity = (i / segments)
    const x = cx + Math.cos(segAngle) * radius
    const y = cy + Math.sin(segAngle) * radius

    ctx.beginPath()
    ctx.arc(x, y, 4, 0, Math.PI * 2)
    ctx.fillStyle = `rgba(255, 255, 255, ${opacity})`
    ctx.fill()
  }

  return canvas.toBuffer('raw')
}

// Animate at 15 fps
let angle = 0
setInterval(async () => {
  const buffer = renderSpinner(angle)
  await deck.fillKeyBuffer(keyIndex, buffer, { format: 'rgba' })
  angle += 0.15
}, 66)
```

### Audio Level Meter

```typescript
function renderLevelMeter(levels: number[]): Buffer {
  const canvas = createCanvas(96, 96)
  const ctx = canvas.getContext('2d')

  ctx.fillStyle = '#0a0a1a'
  ctx.fillRect(0, 0, 96, 96)

  const barWidth = 6
  const gap = 2
  const totalBars = Math.min(levels.length, 10)
  const startX = (96 - totalBars * (barWidth + gap)) / 2

  for (let i = 0; i < totalBars; i++) {
    const level = Math.max(0, Math.min(1, levels[i]))
    const barHeight = level * 76
    const x = startX + i * (barWidth + gap)
    const y = 86 - barHeight

    // Color gradient: green -> yellow -> red
    if (level > 0.8) ctx.fillStyle = '#e74c3c'
    else if (level > 0.6) ctx.fillStyle = '#f39c12'
    else ctx.fillStyle = '#2ecc71'

    ctx.fillRect(x, y, barWidth, barHeight)
  }

  return canvas.toBuffer('raw')
}
```

---

## 6. Dirty Tracking and Selective Updates

For a status dashboard where 32 keys display different metrics, most keys remain unchanged between updates. Sending unchanged images wastes USB bandwidth and CPU time.

### Hash-Based Change Detection

```typescript
import { createHash } from 'crypto'

class ButtonManager {
  private lastHashes = new Map<number, string>()
  private preparedCache = new Map<number, { hash: string; prepared: unknown }>()

  async updateKey(
    deck: StreamDeck,
    keyIndex: number,
    imageBuffer: Buffer
  ): Promise<boolean> {
    // Fast hash of the raw pixel buffer
    const hash = createHash('md5').update(imageBuffer).digest('hex')

    // Skip if unchanged
    if (this.lastHashes.get(keyIndex) === hash) {
      return false  // No update needed
    }

    // Check if we have a cached prepared buffer for this hash
    const cached = this.preparedCache.get(keyIndex)
    if (cached && cached.hash === hash) {
      await deck.sendPreparedBuffer(cached.prepared)
    } else {
      // Encode and send
      const prepared = await deck.prepareFillKeyBuffer(
        keyIndex, imageBuffer, { format: 'rgb' }
      )
      await deck.sendPreparedBuffer(prepared)
      this.preparedCache.set(keyIndex, { hash, prepared })
    }

    this.lastHashes.set(keyIndex, hash)
    return true  // Updated
  }
}
```

### Buffer Comparison (Faster Than Hashing)

For real-time scenarios where even MD5 hashing is too slow, compare buffers directly:

```typescript
function buffersEqual(a: Buffer, b: Buffer): boolean {
  if (a.length !== b.length) return false
  return a.compare(b) === 0  // Native C++ comparison, very fast
}

class FastButtonManager {
  private lastBuffers = new Map<number, Buffer>()

  async updateKeyIfChanged(
    deck: StreamDeck,
    keyIndex: number,
    newBuffer: Buffer
  ): Promise<boolean> {
    const lastBuffer = this.lastBuffers.get(keyIndex)
    if (lastBuffer && buffersEqual(lastBuffer, newBuffer)) {
      return false
    }

    await deck.fillKeyBuffer(keyIndex, newBuffer, { format: 'rgb' })
    this.lastBuffers.set(keyIndex, Buffer.from(newBuffer))  // Copy
    return true
  }
}
```

### State-Driven Dirty Tracking (Most Efficient)

Instead of comparing pixel buffers, track whether the underlying data changed:

```typescript
interface KeyState {
  label: string
  value: string
  color: string
}

class StateDrivenManager {
  private lastStates = new Map<number, string>()
  private preparedStates = new Map<string, unknown>()

  async updateKey(
    deck: StreamDeck,
    keyIndex: number,
    state: KeyState,
    renderFn: (state: KeyState) => Promise<Buffer>
  ) {
    const stateKey = JSON.stringify(state)

    if (this.lastStates.get(keyIndex) === stateKey) {
      return  // State unchanged, skip entirely
    }

    // Check prepared buffer cache
    let prepared = this.preparedStates.get(stateKey)
    if (!prepared) {
      const buffer = await renderFn(state)
      prepared = await deck.prepareFillKeyBuffer(
        keyIndex, buffer, { format: 'rgb' }
      )
      this.preparedStates.set(stateKey, prepared)
    }

    await deck.sendPreparedBuffer(prepared)
    this.lastStates.set(keyIndex, stateKey)
  }
}
```

This is the fastest approach: no rendering or encoding happens at all when state has not changed.

---

## 7. Image Processing Library Recommendations

### For Stream Deck XL Projects in Node.js

| Library | Best For | Install |
|---|---|---|
| **sharp** | Resize, format convert, final JPEG encode | `pnpm add sharp` |
| **@napi-rs/canvas** | Procedural drawing (gauges, charts) | `pnpm add @napi-rs/canvas` |
| **satori** | JSX-to-SVG layout rendering | `pnpm add satori` |
| **@resvg/resvg-js** | SVG-to-PNG/raw buffer | `pnpm add @resvg/resvg-js` |
| **gifuct-js** | GIF frame extraction | `pnpm add gifuct-js` |
| **@julusian/jpeg-turbo** | Fast JPEG encoding for HID transfer | `pnpm add @julusian/jpeg-turbo` |

### Avoid

| Library | Reason |
|---|---|
| **Jimp** | Pure JS, 10-40x slower than sharp |
| **Puppeteer** | 500 MB RAM, 50-200 ms per render, overkill |
| **node-canvas** (Automattic) | Requires system Cairo/Pango install; @napi-rs/canvas is easier |
| **jpeg-js** | Pure JS JPEG encoder; use @julusian/jpeg-turbo instead |

### Sharp vs @napi-rs/canvas Decision

- Use **sharp** when your workflow is: load image, resize, composite SVG overlay, encode JPEG. Sharp does not have a drawing API -- you cannot programmatically draw circles, arcs, or custom shapes.
- Use **@napi-rs/canvas** when you need the Canvas 2D API: `fillRect`, `arc`, `lineTo`, `fillText`, gradients, transformations. It outputs raw RGBA buffers that you can pass directly to `fillKeyBuffer`.
- You can **combine both**: render with @napi-rs/canvas, then pipe the raw buffer through sharp for JPEG encoding if you need direct JPEG output rather than going through the library's internal encoder.

---

## 8. SVG Rendering to Button Images

SVG is a natural fit for small, sharp button images. Two approaches:

### Direct SVG via Sharp

Sharp uses libvips which includes librsvg for SVG input:

```typescript
import sharp from 'sharp'

const svg = `<svg width="96" height="96" xmlns="http://www.w3.org/2000/svg">
  <rect width="96" height="96" rx="8" fill="#2d3436"/>
  <circle cx="48" cy="40" r="20" fill="none" stroke="#00b894" stroke-width="3"/>
  <text x="48" y="46" text-anchor="middle" fill="white"
        font-size="16" font-weight="bold">OK</text>
  <text x="48" y="78" text-anchor="middle" fill="#636e72"
        font-size="10">Service A</text>
</svg>`

const rawBuffer = await sharp(Buffer.from(svg))
  .flatten()
  .raw()
  .toBuffer()
// rawBuffer is 96*96*3 = 27,648 bytes of RGB data
```

### resvg-js (Better Text Rendering, Custom Fonts)

resvg-js is powered by Rust and renders SVGs with better text fidelity and custom font support:

```typescript
import { Resvg } from '@resvg/resvg-js'

const svg = `<svg width="96" height="96">...</svg>`
const resvg = new Resvg(svg, {
  fitTo: { mode: 'width', value: 96 },
  font: {
    fontFiles: ['./fonts/Inter-Bold.ttf'],
    defaultFontFamily: 'Inter',
  },
})

const pngBuffer = resvg.render().asPng()
// Convert PNG to raw RGB for Stream Deck
const rawBuffer = await sharp(pngBuffer).raw().toBuffer()
```

**Performance**: resvg-js renders a 96x96 SVG in ~1-3 ms. Sharp's SVG path is comparable. Both are acceptable for per-frame animation rendering.

---

## 9. Frame Timing and Synchronization

### Monotonic Clock (Avoid Drift)

Never use `setInterval` alone for animation -- the callback scheduling overhead causes drift over time. Use absolute timestamps:

```typescript
function createAnimationLoop(
  fps: number,
  onFrame: (frameNumber: number) => Promise<void>
) {
  const frameDuration = 1000 / fps
  let frameNumber = 0
  let nextFrameTime = performance.now()
  let running = true

  async function tick() {
    if (!running) return

    await onFrame(frameNumber++)
    nextFrameTime += frameDuration

    const now = performance.now()
    const delay = nextFrameTime - now

    if (delay > 0) {
      setTimeout(tick, delay)
    } else {
      // Behind schedule -- catch up
      setImmediate(tick)
    }
  }

  tick()
  return { stop: () => { running = false } }
}

// Usage:
createAnimationLoop(10, async (frame) => {
  const buffer = renderSpinner(frame * 0.15)
  await deck.fillKeyBuffer(0, buffer, { format: 'rgba' })
})
```

### Synchronizing Multi-Key Updates

When updating multiple keys per frame (e.g., a dashboard), send all updates in parallel within a single tick:

```typescript
async function updateDashboard(deck: StreamDeck, metrics: Metrics) {
  const updates: Promise<void>[] = []

  // Only update keys whose metrics changed
  if (metrics.cpuChanged) {
    updates.push(
      deck.fillKeyBuffer(0, renderGauge(metrics.cpu, 100), { format: 'rgba' })
    )
  }
  if (metrics.memChanged) {
    updates.push(
      deck.fillKeyBuffer(1, renderGauge(metrics.mem, 100), { format: 'rgba' })
    )
  }

  // Send all changed keys in parallel
  // (the library internally queues HID writes serially via p-queue)
  await Promise.all(updates)
}
```

Note: even though you call `fillKeyBuffer` in parallel, the underlying USB HID writes are serialized by `p-queue` inside the library. Parallel calls just queue them efficiently.

---

## 10. Complete Example: Status Dashboard with Animation

Putting it all together -- a dashboard that polls system metrics, renders procedurally, and only updates changed keys:

```typescript
import { openStreamDeck, listStreamDecks } from '@elgato-stream-deck/node'
import { createCanvas } from '@napi-rs/canvas'

// Render functions (see Section 5 for implementations)
// renderGauge(value, max) -> Buffer
// renderProgressBar(percent, label) -> Buffer
// renderLevelMeter(levels) -> Buffer

async function main() {
  const devices = await listStreamDecks()
  const deck = await openStreamDeck(devices[0].path)
  await deck.setBrightness(80)

  const lastStates = new Map<number, string>()

  // Update loop -- 2 Hz for status, 10 Hz for animated elements
  const statusLoop = createAnimationLoop(2, async () => {
    const cpu = getCpuPercent()
    const mem = getMemPercent()
    const disk = getDiskPercent()

    const updates = [
      { key: 0, state: `cpu:${Math.round(cpu)}`, render: () => renderGauge(cpu, 100) },
      { key: 1, state: `mem:${Math.round(mem)}`, render: () => renderProgressBar(mem, 'MEM') },
      { key: 2, state: `disk:${Math.round(disk)}`, render: () => renderProgressBar(disk, 'DISK') },
    ]

    for (const { key, state, render } of updates) {
      if (lastStates.get(key) !== state) {
        const buffer = render()
        await deck.fillKeyBuffer(key, buffer, { format: 'rgba' })
        lastStates.set(key, state)
      }
    }
  })

  // Cleanup
  process.on('SIGINT', async () => {
    statusLoop.stop()
    await deck.resetToLogo()
    await deck.close()
    process.exit(0)
  })
}

main().catch(console.error)
```

---

## Sources

1. [Julusian/node-elgato-stream-deck -- GitHub Repository](https://github.com/Julusian/node-elgato-stream-deck) -- Definitive Node.js library for Stream Deck direct control, includes prepared buffer API
2. [python-elgato-streamdeck Animated Images Example](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/animated.html) -- Reference implementation for GIF playback with pre-rendering and monotonic timing
3. [python-elgato-streamdeck Tiled Image Example](https://python-elgato-streamdeck.readthedocs.io/en/stable/examples/tiled.html) -- Reference implementation for multi-key image spanning with gap-aware cropping
4. [Elgato Stream Deck HID API -- Stream Deck XL](https://docs.elgato.com/streamdeck/hid/stream-deck-xl/) -- Official XL specs: 96x96 JPEG, 1024-byte reports, 8x4 grid
5. [Elgato Stream Deck HID API -- General Reference](https://docs.elgato.com/streamdeck/hid/general/) -- Protocol documentation, 50ms polling recommendation
6. [Sharp Performance Benchmarks](https://sharp.pixelplumbing.com/performance/) -- Sharp is ~10x faster than Jimp for image processing
7. [@napi-rs/canvas -- npm](https://www.npmjs.com/package/@napi-rs/canvas) -- High-performance Skia Canvas 2D for Node.js, zero system dependencies
8. [Vercel Satori -- GitHub](https://github.com/vercel/satori) -- JSX-to-SVG renderer, 100x lighter than Puppeteer, flexbox layout support
9. [@resvg/resvg-js -- npm](https://www.npmjs.com/package/@resvg/resvg-js) -- Rust-powered SVG-to-PNG renderer, custom font support
10. [gifuct-js -- GitHub](https://github.com/matt-way/gifuct-js) -- Fastest JavaScript GIF decoder, provides canvas-ready RGBA frame data
11. [@julusian/jpeg-turbo -- npm](https://www.npmjs.com/package/@julusian/jpeg-turbo) -- Native libjpeg-turbo bindings, recommended by Stream Deck library for XL performance
12. [node-canvas vs @napi-rs/canvas vs skia-canvas Comparison](https://www.pkgpulse.com/blog/node-canvas-vs-napi-rs-canvas-vs-skia-canvas-server-side-canvas-nodejs-2026) -- Performance benchmarks: @napi-rs/canvas at 68 ops/sec vs node-canvas at 60 ops/sec
13. [Programming an Elgato Streamdeck with Java (Protocol Details)](https://spinscale.de/posts/2025-02-11-programming-an-elgato-streamdeck-with-java-part-1.html) -- HID packet structure: 8-byte header, 1016-byte payload, chunked transfer
14. [Stream Deck HID Protocol Notes (Cliff Rowley)](https://gist.github.com/cliffrowley/d18a9c4569537b195f2b1eb6c68469e0) -- Reverse-engineered protocol details, V1 vs V2 differences
