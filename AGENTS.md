# Agent instructions for streamdeck

This repo is for research and possible implementation work around using an **Elgato Stream Deck XL** without Elgato's official software.

## Current goal

Research what exists outside of Elgato's software and determine what we can reasonably build ourselves.

Questions to answer:

- What open-source Stream Deck libraries and tools already exist?
- Which tools support the Stream Deck XL specifically?
- Can we use the device directly over USB HID without Elgato's software?
- What protocol details are known for button presses, screen/image updates, brightness, sleep/wake, and device metadata?
- What macOS permissions are actually needed for a minimal replacement?
- What is feasible as a lightweight local-first app, CLI, or menu bar utility?
- What are the tradeoffs between using existing tools versus building our own?

## Product direction

Prefer a small, local-first macOS tool rather than a large plugin platform.

Potential direction:

- Swift-first macOS implementation if we build an app
- Native menu bar/status item app
- Direct USB HID access if feasible
- CLI or local HTTP API for automation
- Simple button profiles/actions stored locally
- No accounts, cloud services, telemetry, or unnecessary permissions

Avoid adding language sprawl unless there is a clear reason. During research, it is fine to inspect Python/Node/Rust/Go projects as references, but do not treat those languages as the preferred final architecture without discussion.

## Research expectations

When researching, document findings clearly under `docs/`:

- source/project name and URL
- supported devices, especially Stream Deck XL
- implementation language and dependencies
- license
- protocol/API notes
- macOS compatibility
- what can be reused or learned
- risks, unknowns, and maintenance status

Do not copy large chunks of third-party source code into this repo. Summarize and link instead. Keep license compatibility in mind.

## Device focus

Primary target:

```text
Elgato Stream Deck XL
```

The user also has one of the original Stream Deck devices, but this research should pay special attention to XL support.

## Tone / user preference

The motivation is similar to the Razer Chroma Status project: the user dislikes bloated vendor software and wants to know whether a small, understandable, local tool can solve the problem better.

Keep recommendations practical and candid:

- what already works
- what would be easy
- what would be annoying
- what is likely not worth building
- what a minimal viable replacement could look like
