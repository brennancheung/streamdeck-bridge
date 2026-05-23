# Status Dashboards and Analytics

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > Novel Use Cases & Ideas

---

## Key Findings

- The Stream Deck XL's 32 individually-addressable LCD keys (each 72x72 pixels, with 144x144 @2x assets supported by the SDK) form a natural grid for persistent status displays, and every key image can be programmatically updated at runtime using the official SDK or direct-control libraries
- Multiple proven open-source projects already use Stream Deck buttons for system monitoring (CPU/GPU/RAM gauges, temperatures, network throughput) with real-time animated charts rendered directly on buttons
- The official Stream Deck SDK supports dynamic SVG rendering via `setImage()`, making it straightforward to generate charts, gauges, and sparklines without external image processing -- SVG is the recommended image format
- The python-elgato-streamdeck library includes a tiled image example that spans a single large image across the entire 8x4 button grid, accounting for inter-key spacing, enabling full-panel dashboard displays
- External data integrations exist for GitHub Actions CI/CD status, Google Analytics real-time users, stock/crypto prices, weather, Uptime Kuma monitors, and Home Assistant entities -- all displaying live data on buttons
- Effective 72x72 pixel design favors bold color-coded backgrounds (green/red/amber), single large numbers, simple arc/ring gauges, and sparkline charts over detailed text -- information density must be ruthlessly prioritized
- Practical refresh rates range from sub-second (system metrics via shared memory) to 15 minutes (weather API) depending on data volatility and API rate limits
- For monitoring tool integration (Grafana, Datadog, CloudWatch), the pattern is to use their HTTP/render APIs to fetch metric values or panel screenshots, then resize and display them on buttons

---

## 1. System Monitoring on Stream Deck Buttons

### 1.1 Existing Projects and Plugins

Several mature projects demonstrate system monitoring on Stream Deck hardware:

**System Vitals Monitor** (Windows, commercial plugin)
- Displays RAM/CPU/GPU usage as percentages or throughput (GB/MB)
- Shows LAN network traffic, ping to DNS servers (Google 8.8.8.8, Cloudflare 1.1.1.1)
- CPU/GPU core temperatures, AIO liquid temperatures
- CPU/GPU watt consumption
- HDD/SSD usage and drive I/O speed
- WiFi signal strength, battery level, master audio volume
- Renders metrics as "real-time smooth-flowing color charts in various styles (line graphs, matrix dots, patterns), similar to the performance graphs in Windows Task Manager"
- Requires administrator privileges for Libre Hardware Monitor and NVIDIA NVML API access

**StreamDeckMonitor** (C#, open source, GitHub: SmokeyMcBong/StreamDeckMonitor)
- Real-time data captured every second: framerate, CPU temperature, CPU load, GPU temperature, GPU load, current time
- Uses StreamDeckSharp for hardware interface, LibreHardwareMonitorLib for system metrics
- MSI Afterburner integration for framerate data
- Supports both static images and video animations as button backgrounds
- Bundled Configurator app for customizing fonts, colors, dimensions, animation framerates, and brightness

**HWiNFO Stream Deck Plugin** (Go/JavaScript, open source, GitHub: shayne/hwinfo-streamdeck)
- Displays any HWiNFO64 sensor value on Stream Deck buttons
- Separates plugin from data providers, enabling remote machine monitoring
- Text resizing support, configurable graph visualization with ranges
- Requires HWiNFO64 running in Sensors-only mode with Shared Memory Support enabled

**BetterTouchTool CPU/RAM Gauges** (macOS, SwiftUI)
- Renders SwiftUI Gauge views into images and sends them to BetterTouchTool for Stream Deck display
- Uses native Apple SwiftUI gauge components for modern visual aesthetic
- Requires macOS Ventura or later for the SwiftUI Gauge view
- Open source with a BasePlugin skeleton for creating additional monitoring visualizations

### 1.2 Architecture Patterns for System Monitoring

The common architecture for system monitoring on Stream Deck follows this flow:

```
[System Metrics Source] --> [Data Collector] --> [Image Renderer] --> [Stream Deck API]
```

**Data sources** vary by platform:
- Windows: LibreHardwareMonitor, HWiNFO64 (shared memory), WMI, Performance Counters
- macOS: sysctl, IOKit, Activity Monitor APIs, psutil (Python)
- Linux: /proc filesystem, sysfs, psutil (Python), lm-sensors

**Collection frequency** for system metrics can be very high (sub-second) since the data is local and accessed via shared memory or filesystem reads, not network API calls.

**Rendering pipeline** typically:
1. Read current metric value
2. Generate a 72x72 (or 144x144 @2x) image with the visualization
3. Convert to the device's native format (JPEG for original models, or base64/SVG via SDK)
4. Push to the specific key via `set_key_image()` or `setImage()`

---

## 2. Visual Design at 72x72 Pixels

### 2.1 What Works Visually on a Button

At 72x72 pixels, every pixel matters. The following visualization types have been proven effective:

**Circular Arc Gauges (Ring/Donut)** -- A colored arc from 0-100% around a central number (18-24pt). The arc fills proportionally to the metric value. Works for: CPU usage, memory, disk, battery. Implementation: PIL `ImageDraw.arc()` or SVG `stroke-dashoffset` on a circle.

**Horizontal/Vertical Bar Charts** -- Filled rectangles showing percentage. Stack multiple thin bars vertically for multi-metric display (e.g., CPU/RAM/DSK/NET on one button). Color transitions green -> amber -> red based on thresholds.

**Sparkline Charts** -- Mini line charts showing recent history (30-60 data points). Plot area fills ~60x40 pixels, leaving room for a label and current value. Optional area fill beneath the line for visual weight. Works for: CPU history, network throughput, response times.

**Traffic Light / Status Indicator** -- Full button background in green, amber, or red with optional short text overlay (2-4 characters). Instantly readable from across a desk. Works for: service health, build status, alert states.

**Single Large Number** -- One metric value in large bold font (28-36pt) with small label text above (8-10pt). Optional color coding of the number or background. Works for: active users, unread count, error count, price.

### 2.2 Typography at Small Sizes

Text readability is the primary challenge at 72x72 pixels:

- **Maximum useful font size**: 28-36pt for a single number (2-3 digits)
- **Label text**: 8-10pt works but must be sans-serif and bold
- **Maximum text lines**: 2-3 lines before legibility degrades
- **Recommended fonts**: System sans-serif fonts (Arial, Helvetica, SF Pro), pixel/bitmap fonts for very small sizes
- **Avoid**: Serif fonts, thin weights, more than 6-8 characters per line
- **Color contrast**: White text on dark backgrounds or dark text on bright status backgrounds

For Python/PIL rendering, use `ImageFont.truetype()` with a sans-serif TTF at appropriate sizes. Bitmap fonts (loaded via `ImageFont.load()`) can be sharper at very small fixed sizes since they are designed pixel-by-pixel rather than rasterized from vectors.

### 2.3 Color Design Language

A consistent color system is essential for at-a-glance reading:

| Color | Meaning | Hex | Use Cases |
|-------|---------|-----|-----------|
| Green | Healthy / OK / Up | #00C853 | Services up, builds passing, low usage |
| Amber/Yellow | Warning / Degraded | #FFD600 | High usage, slow response, pending |
| Red | Critical / Down / Failed | #FF1744 | Services down, builds failed, errors |
| Blue | Informational / Active | #2979FF | Active users, in-progress, neutral data |
| Gray | Unknown / Inactive | #757575 | No data, paused, disabled |
| White | Neutral text/data | #FFFFFF | Numbers, labels on dark backgrounds |

For continuous metrics (like CPU usage), gradient color transitions work well:
- 0-60%: Green
- 60-80%: Amber/Yellow
- 80-100%: Red

---

## 3. Rendering Techniques

### 3.1 Python with PIL/Pillow (Direct Control)

Using the `python-elgato-streamdeck` library with PIL/Pillow is the most flexible approach for custom dashboards. The core pattern is:

1. Create a 72x72 PIL `Image` with `Image.new("RGB", (72, 72), (0, 0, 0))`
2. Use `ImageDraw` to render arcs (`draw.arc()` for gauges), lines (for sparklines), rectangles (for bars), and text (`draw.text()` with `ImageFont.truetype()`)
3. Apply threshold-based coloring: green below 60%, amber 60-80%, red above 80%
4. Convert and push: `PILHelper.to_native_key_format(deck, img)` then `deck.set_key_image(key, native)`

Key PIL drawing primitives for dashboards:
- `draw.arc(bbox, start, end, fill, width)` -- circular gauge arcs
- `draw.line(points, fill, width)` -- sparkline chart lines
- `draw.rectangle(bbox, fill)` -- bar charts and backgrounds
- `draw.text(xy, text, fill, font, anchor)` -- labels and values (use `anchor="mm"` for centering)

### 3.2 SVG via the Official Stream Deck SDK (Plugin Development)

The official SDK (Node.js/TypeScript) recommends SVG as the image format for dynamic content. The pattern is to generate an SVG string and pass it as a data URL:

```javascript
const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="72" height="72">...</svg>`;
action.setImage(`data:image/svg+xml,${encodeURIComponent(svg)}`);
```

SVG advantages for Stream Deck dashboards:
- Text rendering is crisp at any size
- No external image processing libraries needed
- Lightweight to generate (string concatenation with template literals)
- Supported natively by the SDK -- no conversion step
- Can include gradients and complex paths
- `stroke-dasharray` / `stroke-dashoffset` on circles creates smooth arc gauges
- `<polyline>` elements create sparkline charts directly in markup

### 3.3 Node.js with Canvas/Sharp (Direct Control)

Using the `@elgato-stream-deck/node` package with Sharp for image processing:

- Sharp can eliminate alpha channels, resize images, and provide uncompressed RGB buffers
- Install `@julusian/jpeg-turbo` alongside for significantly improved performance on Stream Deck XL (JPEG encoding is a bottleneck for 32-key updates)
- Canvas (`@napi-rs/canvas` or `node-canvas`) can render charts to buffers before passing to the Stream Deck API

### 3.4 Matplotlib for Complex Charts (Python)

For more sophisticated visualizations (area charts, bar charts with labels, multi-series plots), render matplotlib figures to PIL images via BytesIO. Use `matplotlib.use('Agg')` for headless rendering, set `figsize=(1, 1)` and `dpi=72`, remove axes with `ax.axis('off')`, save to BytesIO as PNG, and open with PIL.

Note: matplotlib has higher overhead than raw PIL drawing. For real-time updates faster than every 2-3 seconds, raw PIL or SVG is preferred.

---

## 4. Pulling Data from External Services

### 4.1 Existing Plugins for External Data

**Weather Display**
- streamDeck-weatherPlugin (GitHub: JaouherK/streamDeck-weatherPlugin) -- uses WeatherAPI.com
- Shows current conditions as picture image, city name, temperature
- Refresh: every 15 minutes
- Units configurable: Celsius/Fahrenheit, km/h or mph
- Five distinct actions: current weather, weather slider (multiple cities), astronomy, forecast, weather details

**Stock/Crypto Prices**
- TickerTap (GitHub: matextrem/streamdeck-tickertap) -- no API key required
  - Sources: Finviz (US stocks, forex, commodities), Investing.com (EU/Asian stocks, ETFs), CoinMarketCap (crypto)
  - Displays: ticker symbol, current price, price change with color coding (green up, red down)
  - Refresh options: on push, 5 min, 30 min, 1 hour, or custom interval
  - Optional P&L calculation when average cost is provided
- Crypto Ticker PRO -- Bitfinex and Binance via WebSocket (real-time), Yahoo Finance for stocks (15-min delay)
- Bitcoin Ticker -- BTC, ETH, percentage change, trends
- Kimchi Ticker -- Bitcoin, Ethereum, global stocks, forex, major indices

**Google Analytics**
- stream-deck-google-analytics (GitHub: captain-melanie/stream-deck-google-analytics)
- Displays real-time: active users, specific goal completions, all goal completions, total events
- Uses Google Analytics Real-Time API
- Requires: Google Cloud Platform credentials (profile ID, client ID, secret, refresh token)

**Unread Email Counts**
- UnreadMails-Streamdeck-Plugin (GitHub: LeGone/UnreadMails-Streamdeck-Plugin) -- displays unread email count
- OutlookUnreadCounter (GitHub: McCzarny/OutlookUnreadCounter) -- Microsoft Outlook unread count
- elgatosf/streamdeck-applemail -- Apple Mail badge with option to hide when count is 0

**Slack Integration**
- streamdeck-slack (GitHub: paultyng/streamdeck-slack) -- Slack plugin for Stream Deck
- streamdeck-slack-status (GitHub: ellreka/streamdeck-slack-status) -- updates Slack status from Stream Deck

### 4.2 Custom API Integration Pattern

For any REST API, the general pattern is a polling loop: fetch JSON from the API, extract the display value with a parse function, render a 72x72 image with a render function, convert to native format, and push to the key. Wrap in try/except to display an error indicator (red background, "!" icon) on failure. Sleep for the appropriate interval between polls.

### 4.3 WebSocket/Streaming Integration

For real-time data (crypto prices, monitoring alerts), WebSocket connections avoid polling overhead:

- Crypto Ticker PRO uses WebSocket connections to Bitfinex and Binance for real-time price updates
- Uptime Kuma plugin uses SocketIO interface for instant state synchronization
- Home Assistant plugin uses WebSocket for entity state changes

WebSocket-based updates are superior for:
- Sub-second latency requirements
- High-frequency data (price tickers, active user counts)
- Event-driven status changes (alerts, deployments completing)

---

## 5. Monitoring Tool Integration

### 5.1 Grafana Integration

Grafana does not have a native Stream Deck plugin, but its HTTP API enables custom integration:

**Panel Rendering API:** `GET /render/d-solo/<UID>/<SLUG>?panelId=2&width=72&height=72` with a Bearer API key returns a rendered PNG. Render larger and downscale for better quality. The image renderer uses headless Chromium (resource-intensive); as of Grafana 12.x, a remote Docker service is recommended.

**Data Query API:** `POST /api/ds/query` returns raw metric data for rendering custom gauges locally -- more efficient than screenshots for simple values. Poll at 30s-5min intervals.

### 5.2 Datadog Integration

Datadog's **Metrics Query API** (`GET /api/v1/query`) returns time-series data for sparklines. The **Monitors API** (`GET /api/v1/monitor`) returns status of all monitors (OK, Alert, Warn, No Data), mapping naturally to color-coded buttons. The **Graph Snapshot API** returns rendered graph image URLs that can be fetched and resized.

### 5.3 AWS CloudWatch Integration

Query via AWS SDK (`get_metric_statistics()`). Useful metrics: Lambda (Invocations, Errors, Duration, Throttles), EC2 (CPUUtilization, NetworkIn/Out, StatusCheckFailed), RDS (DatabaseConnections, FreeStorageSpace), ALB (RequestCount, HTTPCode_Target_5XX_Count). Each metric maps to a color-coded button (green = healthy, red = alarm).

### 5.4 Prometheus Integration

Query via HTTP API: `GET /api/v1/query` for instant queries, `/api/v1/query_range` for sparkline data. The Alertmanager API (`GET /api/v2/alerts`) returns active alerts for color-coded status buttons (critical = red, warning = amber, resolved = green).

---

## 6. CI/CD Build and Deploy Status

### 6.1 DevOps for Stream Deck Plugin

The devops-streamdeck plugin (GitHub: SantiMA10/devops-streamdeck) provides direct CI/CD status display:

**Supported platforms:**
- GitHub Actions (requires Personal Token with repo:status, repo_deployment, public_repo scopes)
- GitLab CI (requires read_api and read_user scopes)
- Travis CI (both .com and .org)
- Netlify (site deployment status via Site ID)
- Vercel (project deployment status)

**Configuration per button:**
- Account/Token: authentication credential
- Repository: owner/repo combination
- Branch: optional filter for specific branch status
- Automatic polling for status updates

**Built with React and TypeScript**, using Parcel as bundler. Architecture separates Property Inspector (configuration UI), Plugin (core logic), and Setup screens.

### 6.2 Custom GitHub Actions Integration

For more granular CI/CD monitoring, query the GitHub Actions API directly: `GET /repos/{owner}/{repo}/actions/workflows/{workflow}/runs?per_page=1&branch=main`. The response contains `status` (queued, in_progress, completed) and `conclusion` (success, failure, cancelled) fields.

**Visual mapping for CI/CD status:**
| Status | Button Appearance |
|--------|-------------------|
| Success/Passing | Green background, checkmark icon |
| Failed | Red background, X icon |
| In Progress | Blue background, spinner/dots animation |
| Queued | Gray background, clock icon |
| Cancelled | Dark gray, slash icon |

### 6.3 Kubernetes Cluster Status

While no dedicated Stream Deck plugin exists for Kubernetes monitoring, the Kubernetes API can be polled to display:

- Pod health across namespaces (one button per namespace or deployment)
- Node status (Ready/NotReady)
- Deployment rollout progress
- Alert count from Prometheus Alertmanager

The pattern is the same: query the K8s API (`kubectl get pods -o json` or the REST API directly), parse status, render a color-coded button.

---

## 7. Notification Counts and Badges

### 7.1 Existing Notification Plugins

**Email:**
- UnreadMails plugin: displays unread count on button icon
- Outlook Unread Counter: shows Microsoft Outlook unread count
- Apple Mail: official Elgato plugin with badge count, hides badge when count is 0

**Slack:**
- streamdeck-slack: general Slack integration
- streamdeck-slack-status: status management from Stream Deck

**GitHub:**
- The devops-streamdeck plugin supports GitHub notifications

### 7.2 Custom Notification Badge Design

Three effective patterns for displaying notification counts at 72x72 pixels:

- **Pattern A (Large number):** Single large bold number (28-36pt) with small label below. Best for single-service monitoring where the count is the primary information.
- **Pattern B (Icon + badge):** App icon centered, red circular badge with count in top-right corner. Mimics familiar mobile badge UX. Effective when the service identity matters.
- **Pattern C (Multi-service summary):** 4 rows of "service count" pairs in small text. Packs multiple services onto one button but requires very small text (8-9pt).

### 7.3 Polling Considerations for Notifications

| Service | API Endpoint | Recommended Interval | Notes |
|---------|-------------|---------------------|-------|
| Gmail | Gmail API /messages?q=is:unread | 30-60 seconds | Rate limit: 250 units/second |
| Outlook | Microsoft Graph /me/mailFolders/inbox | 30-60 seconds | Requires Azure AD app |
| Slack | conversations.history + unread_count | 15-30 seconds | Or use Events API via WebSocket |
| GitHub | /notifications | 60 seconds | Respects X-Poll-Interval header |
| Jira | /rest/api/2/search?jql=... | 60-120 seconds | JQL query for assigned issues |

---

## 8. Full-Grid Dashboard Display

### 8.1 Tiled Image Across All Keys

The python-elgato-streamdeck library includes a complete example for spanning a single image across all 32 keys of a Stream Deck XL.

**How it works:**

1. Calculate total deck image size including inter-key spacing (approximately 36px between keys). For an 8x4 grid: `8*72 + 7*36 = 828` px wide, `4*72 + 3*36 = 396` px tall
2. Resize or generate the source image to this total size
3. For each key, compute its row/col position, calculate pixel offsets accounting for spacing, and crop the corresponding 72x72 region
4. Push each cropped tile to its corresponding key

**Dashboard layout possibilities for an 8x4 grid:**

```
+------+------+------+------+------+------+------+------+
| CPU  | CPU  | RAM  | RAM  | Disk | Net  | Net  | Alerts|
|Gauge |Spark |Gauge |Spark | Bar  | In   | Out  | Count |
+------+------+------+------+------+------+------+------+
| SVC1 | SVC2 | SVC3 | SVC4 | SVC5 | SVC6 | SVC7 | SVC8 |
| [OK] | [OK] |[WARN]| [OK] | [OK] |[FAIL]| [OK] | [OK] |
+------+------+------+------+------+------+------+------+
| GH   | GH   | GH   | GH   | Build| Build| Build| Deploy|
| PR#1 | PR#2 | PR#3 | CI   | Prod | Stg  | Dev  | Status|
+------+------+------+------+------+------+------+------+
| Mail | Slack|GitHub| Jira | Users| Rev  | Errs | Time  |
|  42  |  7   |  3   |  11  | 1.2k |$847  |  0   | 14:32 |
+------+------+------+------+------+------+------+------+
```

Row 1: System metrics with mixed visualization types
Row 2: Service health indicators (color-coded backgrounds)
Row 3: CI/CD pipeline and deployment status
Row 4: Notification counts and key business metrics

### 8.2 Single Composite Image Dashboard

For a unified dashboard view, create an 828x396 pixel image, render each cell at its grid position (accounting for the 36px spacing gaps that fall between keys), then iterate over all 32 keys, cropping each 72x72 tile from the composite and pushing it to the device. This approach allows you to render cross-button elements (charts spanning multiple keys) as part of a single image.

### 8.3 Mixed-Mode Dashboards

The most practical approach combines both patterns:
- Some buttons show independent, self-contained indicators (status dots, single numbers)
- Groups of 2x2 or 4x2 buttons display larger visualizations (charts spanning multiple keys)
- One row might be a tiled sparkline chart while another row is individual status indicators

This avoids the visual discontinuity of the inter-key gaps for content that benefits from continuity (charts) while keeping simple indicators on single buttons.

---

## 9. Refresh Rates and Update Strategies

### 9.1 Recommended Refresh Rates by Data Type

| Data Type | Update Interval | Rationale |
|-----------|----------------|-----------|
| System CPU/RAM/GPU | 1-2 seconds | Local data, high volatility |
| Disk usage | 30-60 seconds | Changes slowly |
| Network throughput | 1-5 seconds | High volatility, local data |
| Crypto/stock prices (WebSocket) | Real-time (sub-second) | WebSocket push, no polling cost |
| Stock prices (REST API) | 5-15 minutes | API rate limits, slower data |
| Weather | 15-30 minutes | Slow-changing, API limits |
| CI/CD build status | 15-30 seconds | Event-driven, moderate urgency |
| Service health/uptime | 10-30 seconds | Critical, but not sub-second |
| Email/notification counts | 30-60 seconds | Moderate urgency |
| Google Analytics users | 30-60 seconds | Real-time API allows frequent polls |
| CloudWatch/Prometheus | 30-60 seconds | Metric resolution typically 1-5 min |
| Grafana panel screenshots | 60-300 seconds | Heavy to render, slow to change |
| Clock/time display | 1 second | Must appear live |

### 9.2 Performance Considerations

**USB bandwidth:**
- Stream Deck XL uses USB 2.0
- Each 72x72 JPEG key image is approximately 3-5 KB
- Updating all 32 keys: ~100-160 KB per full refresh
- USB 2.0 throughput (480 Mbps) is not the bottleneck
- The bottleneck is JPEG encoding on the host side and the device's internal processing

**Image encoding performance:**
- For Node.js: install `@julusian/jpeg-turbo` for "greatly improved performance for writing images to the StreamDeck XL" -- without it, jpeg-js is "noticeably more cpu intensive and slower"
- For Python: PIL's JPEG encoding is reasonably fast; the real overhead is in matplotlib if used
- For SVG (official SDK): no encoding needed, string is passed directly

**Practical limits:**
- Updating a single key: effectively instantaneous (<50ms)
- Updating all 32 keys sequentially: 200-500ms depending on encoding
- The Intinor Direkt plugin uses a 2-second polling interval between state updates
- For smooth animations, 10-15 FPS per key is achievable but CPU-intensive for 32 keys simultaneously
- For dashboard use cases, updating changed keys only (not all 32 each cycle) significantly reduces load

### 9.3 Smart Update Strategies

**Delta updates:** Only re-render and push keys whose underlying data has changed. Track previous values and skip updates when the display would not visually change.

**Tiered refresh:** Different rows or groups of keys can poll at different intervals. System metrics every 2 seconds, CI/CD every 30 seconds, weather every 15 minutes.

**Priority rendering:** When multiple keys need updating simultaneously, prioritize critical indicators (alerts, errors) over informational displays (weather, clock).

**Batch rendering:** Render all pending image updates first, then push them to the device in rapid succession to minimize visual tearing between related keys.

---

## 10. Server and Infrastructure Monitoring

### 10.1 Uptime Kuma Integration

The Streamdeck-Uptime-Kuma plugin (GitHub: MarlBurroW/Streamdeck-Uptime-Kuma) provides a polished integration:

**Visual indicators:**
- Green background: monitor is up
- Red background: monitor is down
- Orange background: monitor is paused
- Gray background: unknown status (awaiting first update)

**Displayed data (cyclable by pressing the button):**
- Current ping in milliseconds (from last heartbeat)
- Average ping over last 24 hours
- Uptime percentage for last 24 hours
- Uptime percentage for last 30 days

**Key features:**
- Uses SocketIO interface (same as the Uptime Kuma frontend) for instant state synchronization -- no polling delay
- Pause/resume monitors directly from Stream Deck
- Stream Deck+ compatible

### 10.2 Home Assistant as a Monitoring Bridge

Home Assistant can serve as a middleware layer between monitoring systems and Stream Deck. Install the streamdeck-homeassistant plugin (GitHub: cgiesche/streamdeck-homeassistant), connect via WebSocket, and create Home Assistant sensors for your monitoring data (REST sensors, MQTT sensors, command-line sensors). This gives you a unified data layer with hundreds of integrations, and the Stream Deck plugin displays entity states in real-time.

### 10.3 DevDeck for Developer Workflows

DevDeck (GitHub: jamesridgway/devdeck) is a Python-based Stream Deck control system with YAML configuration, a plugin system, and built-in controls (clock, command execution, timer, volume). Plugins include Slack integration, Home Assistant control, and key light management. Write custom DeckControl or DeckController classes for monitoring or data display.

---

## 11. Practical Dashboard Design Recommendations

### 11.1 Information Hierarchy

When designing a 32-button dashboard, organize information by urgency and scan frequency:

**Top row (keys 0-7):** Critical system health -- things you glance at constantly
- Service status indicators (green/red)
- Active alert count
- Error rate

**Second row (keys 8-15):** Performance metrics -- checked periodically
- CPU/RAM/Disk gauges
- Network throughput sparklines
- Response time indicators

**Third row (keys 16-23):** Workflow status -- checked during work sessions
- CI/CD build status per environment
- Pull request counts
- Deployment status

**Bottom row (keys 24-31):** Informational -- nice to have at a glance
- Notification counts (email, Slack, GitHub)
- Weather, clock
- Business metrics (active users, revenue)

### 11.2 Visual Consistency Rules

1. **One visualization type per row** for visual coherence
2. **Consistent color language** across all buttons (green = good, red = bad, always)
3. **Dark backgrounds** (#1a1a1a or #000000) for contrast and reduced eye strain
4. **Maximum two text elements per button**: one value and one label
5. **No borders or decorative elements** -- every pixel carries information
6. **Consistent font sizes**: pick two sizes (large for values, small for labels) and use them everywhere
7. **Leave breathing room**: 4-6px margin from button edge improves readability

### 11.3 Glance-ability and Multi-Page Design

The primary advantage of a Stream Deck dashboard over a monitor is peripheral vision. Design for it: color should convey meaning without reading text, status changes should cause obvious background color shifts, and numbers should be readable from 2-3 feet away (20pt minimum). Avoid subtle gradients -- make states visually distinct.

Stream Deck profiles and pages extend beyond 32 buttons. Use one button as a "page switch" to cycle between dashboard views (infrastructure health, business metrics, personal productivity), or use button presses to drill down from summary to detail.

---

## 12. Stream Deck + XL and Key Image Resolution

The newer Stream Deck + XL adds 36 LCD keys, 6 rotary encoders (useful for scrolling time ranges on charts or navigating dashboard pages), and a touch strip for persistent status bars. The SDK manifest specifies action state images at 72x72 pixels with 144x144 @2x variants. Design at 144x144 for maximum clarity; SVG format automatically scales to any resolution.

---

## Sources

1. [System Vitals Monitor Plugin](https://vivre-motion.com/products/systems-vitals-for-windows-stream-deck-plugin)
2. [StreamDeckMonitor - C# Real-Time System Stats](https://github.com/SmokeyMcBong/StreamDeckMonitor)
3. [HWiNFO Stream Deck Plugin](https://github.com/shayne/hwinfo-streamdeck)
4. [BetterTouchTool CPU/RAM Gauges](https://community.folivora.ai/t/fancy-cpu-and-ram-usage-gauges-on-the-streamdeck-also-where-are-all-the-plugins-at/28738)
5. [python-elgato-streamdeck Library](https://github.com/abcminiuser/python-elgato-streamdeck)
6. [python-elgato-streamdeck Tiled Image Example](https://python-elgato-streamdeck.readthedocs.io/en/latest/examples/tiled.html)
7. [DevOps for Stream Deck (CI/CD Status)](https://github.com/SantiMA10/devops-streamdeck)
8. [Stream Deck Weather Plugin](https://github.com/linariii/streamdeck-weather)
9. [TickerTap - Real-Time Asset Values](https://github.com/matextrem/streamdeck-tickertap)
10. [Crypto Ticker PRO](https://marketplace.elgato.com/product/crypto-ticker-pro-4350dbca-7e3c-4933-8a94-f8f8a960079e)
11. [Bitcoin Ticker](https://marketplace.elgato.com/product/bitcoin-ticker-dc86fe0d-ab07-4c97-8001-9e52359a7ff6)
12. [Stream Deck Google Analytics Real-Time](https://github.com/captain-melanie/stream-deck-google-analytics)
13. [UnreadMails Stream Deck Plugin](https://github.com/LeGone/UnreadMails-Streamdeck-Plugin)
14. [Outlook Unread Counter](https://github.com/McCzarny/OutlookUnreadCounter)
15. [Uptime Kuma Stream Deck Plugin](https://github.com/MarlBurroW/Streamdeck-Uptime-Kuma)
16. [Home Assistant Stream Deck Plugin](https://github.com/cgiesche/streamdeck-homeassistant)
17. [DevDeck - Developer Stream Deck Software](https://github.com/jamesridgway/devdeck)
18. [Jira/Confluence Stream Deck Plugin](https://github.com/mediabounds/streamdeck-jira)
19. [Stream Deck SDK - Keys Documentation](https://docs.elgato.com/streamdeck/sdk/guides/keys/)
20. [Stream Deck SDK - Manifest Reference](https://docs.elgato.com/streamdeck/sdk/references/manifest/)
21. [node-elgato-stream-deck (Node.js Library)](https://github.com/Julusian/node-elgato-stream-deck)
22. [Elgato Stream Deck SDK Getting Started](https://docs.elgato.com/streamdeck/sdk/introduction/getting-started/)
23. [Grafana Image Renderer](https://github.com/grafana/grafana-image-renderer)
24. [Grafana Image Rendering Setup](https://grafana.com/docs/grafana/latest/setup-grafana/image-rendering/)
25. [Python Sparklines with Matplotlib](https://www.markhneedham.com/blog/2017/09/23/python-3-create-sparklines-using-matplotlib/)
26. [PIL/Pillow ImageDraw.arc() Documentation](https://www.geeksforgeeks.org/python-pil-imagedraw-draw-arc/)
27. [PIL/Pillow ImageFont Documentation](https://pillow.readthedocs.io/en/stable/reference/ImageFont.html)
28. [Elgato Stream Deck XL Product Page](https://www.elgato.com/us/en/p/stream-deck-xl)
29. [Stream Deck + XL Product Page](https://www.elgato.com/us/en/p/stream-deck-plus-xl)
30. [Celona Network Monitoring API + Stream Deck](https://docs.celona.io/en/articles/6335959-celona-network-monitoring-api-integration-with-elgato-stream-deck)
31. [Hardware Stats Monitor - Elgato Marketplace](https://marketplace.elgato.com/product/hardware-stats-monitor-876baa34-f177-4eef-8b5a-0a31e5a38b22)
32. [streamdeck-slack (Slack Plugin)](https://github.com/paultyng/streamdeck-slack)
33. [home-assistant-streamdeck-yaml](https://github.com/basnijholt/home-assistant-streamdeck-yaml/)
