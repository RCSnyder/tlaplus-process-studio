# TLA+ Process Studio

_Model it before you automate it._

A 100% client-side web application for modeling any business process as a TLA+ state machine, collecting structured stakeholder feedback, and iterating toward an accurate shared model with the help of any LLM.

Built with Rust, WebAssembly, and [Yew](https://yew.rs). Nothing leaves your browser — no server, no API, no analytics, no cookies, no CDN calls. All data lives in `localStorage`.

## What It Does

You cannot change what you cannot see. TLA+ Process Studio makes a process visible as a state machine — named states, explicit transitions, documented exceptions — so the people who live with it every day can inspect it together and decide what to change.

### Core Loop

1. **Generate** — Use any LLM to produce a parser-safe TLA+ state machine of your process. The built-in prompts aggressively constrain the output to a single module in the subset this app can reliably visualize: one state variable, one state set, explicit named transitions, exact `Init`, and flat `Next`. Paste it into the editor and click **Parse**.
2. **Simulate** — The left panel shows available transitions from the current state. Click one to advance. The diagram highlights where you are and where you can go.
3. **Comment** — The middle panel shows the current node. Type what's wrong, missing, or unclear. Comments are tagged with the state name and your name.
4. **Iterate** — Copy the **Iterate** prompt, which bundles the current spec and all comments into a structured revision prompt. The LLM classifies feedback, finds gaps, and outputs a revised spec. Paste it back and repeat.

### Tabs

- **Model** — Three-panel layout: Simulate (next nodes), Feedback (current node + comments), and State Machine Diagram (all nodes). Shared control bar with Back, Reset, and breadcrumb trail.
- **Prompts** — Four built-in prompts: **New spec** (bootstrap interview), **Basic syntax** (freeform-to-parser-safe conversion), **Iterate** (spec + comments bundled for revision), and **Agent** (full agent interface bundle with stable selectors).
- **Versions** — Manual snapshots, auto-saves on parse, import/export of individual versions or full workspace state as JSON.
- **Guide** — Philosophy, workflow walkthrough, stakeholder guidance, and further reading references.

### Features

- **TLA+ Parser** — Regex-based MVP parser that extracts states, transitions, comments, and invariants from TLA+ modules. Expects state sets named `*States` or `*Stages` and a variable named `*State`.
- **State Machine Simulator** — Interactive walkthrough of parsed transitions from the current state.
- **SVG State Diagram** — Custom rendered state diagram with zoom, pan, and clickable nodes. No external dependencies.
- **Structured Comments** — Per-state feedback with category tags and author attribution, persisted to `localStorage`.
- **Share URLs** — One-click URL generation with spec + comments encoded in the fragment hash.
- **Deep Links** — Tab anchors (`#model`, `#prompts`, `#versions`, `#guide`) and example params (`?example=slug`) are bookmarkable.
- **Responsive Layout** — Dual-pane viewport-locked UI that stacks on narrow screens. Auto dark/light mode via `prefers-color-scheme`, plus manual toggle.
- **Example Specs** — Seven built-in example state machines accessible from the dropdown, covering software delivery, AI workflows, MLOps, innovation adoption, and the invisible machines of organizational life.

### Example Specs

The app ships with ready-to-explore examples in the **Example specs** dropdown to get a mental model of what common business process state machines look like:

| Example                    | States | What It Models                                                                                                                                                                                                                                                                                                                              |
| -------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Collaborative Modeling** | 9      | The collaborative spec-building workflow this tool is designed for                                                                                                                                                                                                                                                                          |
| **Innovation Adoption**    | 21     | The minimill pattern — how disruptive technology gets adopted in enterprises, from signal detection through capability compounding. Draws on César Hidalgo's "The Infinite Alphabet" on how knowledge grows combinatorially. Includes failure modes: hype capture, pilot purgatory, vendor lock-in, shadow adoption, transformation theater |
| **Team Delivery**          | 14     | Software engineering ticket lifecycle from backlog through production release, including code review loops, QA rejection, blockers, parked work, and hotfixes                                                                                                                                                                               |
| **QRSPI Workflow**         | 17     | The [Questions-Research-Structure-Plan-Implement](https://github.com/jaeyunha/QRSPI-workflow) methodology for AI-assisted coding, with rejection and rework paths at each phase gate                                                                                                                                                        |
| **MLOps Lifecycle**        | 18     | End-to-end machine learning operations from problem framing through production monitoring, with drift detection, canary rollback, and incident response                                                                                                                                                                                     |
| **Meeting Lifecycle**      | 13     | The invisible state machine underneath every meeting — happy path from need to follow-up, plus the dysfunction modes: no agenda, derailment, no-decision loops, forgotten actions                                                                                                                                                           |
| **Hiring Pipeline**        | 16     | Candidate journey from application through onboarding, including ghosting, rejection at each stage, offer negotiation, candidate withdrawal, and requisition cancellation                                                                                                                                                                   |

Each example is a complete, parseable TLA+ spec with comments that explain what happens at each state and why.

## Getting Started

### Option 1: Docker Compose

```bash
docker compose up base-kit
```

Builds and serves the app inside a Rust + Trunk container on `http://localhost:8080`.

### Option 2: Local Tooling

```bash
./run.sh
```

Installs [Trunk](https://trunkrs.dev) if needed, adds the `wasm32-unknown-unknown` target, and starts the dev server with auto-open.

### Option 3: Manual

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve --open
```

## Project Structure

```text
├── Cargo.toml          # Rust dependencies (yew, regex, serde, web-sys, wasm-bindgen)
├── Trunk.toml          # Trunk build config → dist/ on port 8080
├── index.html          # HTML shell, zoom/pan scripts, agent-readable comments
├── style.css           # Full UI styles with auto dark mode
├── compose.yaml        # Docker Compose for containerized dev
├── run.sh              # One-command local bootstrap
├── fixtures/
│   └── examples/       # Example TLA+ specs (embedded at compile time)
│       ├── innovation-adoption.tla
│       ├── team-delivery.tla
│       ├── qrspi-workflow.tla
│       ├── mlops-lifecycle.tla
│       ├── meeting-lifecycle.tla
│       └── hiring-pipeline.tla
└── src/
    ├── main.rs         # Yew app component, all UI, storage, prompts, export logic
    ├── model.rs        # Data types: ParsedMachine, Action, Comment
    └── parser.rs       # Regex-based TLA+ parser
```

## How the Parser Works

The parser in [src/parser.rs](src/parser.rs) uses regex to extract structure from TLA+ modules:

- **Module name** from `---- MODULE Name ----`
- **States** from set literals assigned to identifiers ending in `States` or `Stages`
- **Transitions** from named operators containing `variable = "FromState"` and `variable' = "ToState"` patterns (variable must end in `State`)
- **Comments** from `(* ... *)` blocks immediately preceding an action definition
- **Invariants** from operators containing `=>`

It is intentionally an MVP parser — it handles the subset of TLA+ that the prompt templates produce. Complex or quantified actions will generate a warning rather than silently fail.

## Parser-Safe Authoring Contract

This product is intentionally opinionated. It does not try to support arbitrary TLA+. It supports a narrow, stakeholder-readable authoring subset that can be parsed and visualized reliably.

The built-in prompts are written to force LLMs into that subset. A valid spec for this app should satisfy all of the following:

- Exactly one `VARIABLE`, and its name must end in `State`
- Exactly one state set, and its name must end in `Stages` or `States`
- State names must be quoted PascalCase strings with no spaces
- Each transition must be a named operator with exactly two conjuncts:

```tla
ActionName ==
  /\ processState = "FromState"
  /\ processState' = "ToState"
```

- `Init` must be exactly `Init == processState = "InitialState"`
- `Next` must be a flat disjunction of action names only
- Narrative comments must be `(* ... *)` blocks immediately above the action they describe
- No helper operators using `==`
- No `IF/THEN/ELSE`, `CASE`, `LET/IN`, `UNCHANGED`, quantifiers, or extra conjuncts in transition bodies
- No prose before the module and no prose after `====` when generating output intended to be pasted into the editor

If you want full TLA+ freedom later, treat this tool's format as the visual collaboration layer, not the final formal specification.

## AI Agent Integration

TLA+ Process Studio is designed to be driven by an external LLM agent using any browser automation tool (MCP Playwright, browser-use, Puppeteer, DevTools, etc.). The app exposes a stable `data-*` attribute interface so selectors don't break when CSS classes change.

### Stable DOM selectors

| Selector                        | Element          | Purpose                                            |
| ------------------------------- | ---------------- | -------------------------------------------------- |
| `[data-field="spec"]`           | `<textarea>`     | TLA+ source editor — read or write the spec        |
| `[data-action="parse"]`         | `<button>`       | Triggers re-parse and diagram update               |
| `[data-action="save-snapshot"]` | `<button>`       | Saves current spec+comments as a named version     |
| `[data-parser-status]`          | App root `<div>` | Parse state: `"ok"` \| `"warnings"` \| `"no-spec"` |
| `[data-module]`                 | App root `<div>` | Current parsed module name                         |
| `[data-state-count]`            | App root `<div>` | Number of parsed states                            |

### localStorage API

```js
// Read the spec
localStorage.getItem("tla_studio_source")

// Read all stakeholder comments
JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
// → [{target, author, text, category?, chain?}, ...]

// Read named snapshots (version history)
JSON.parse(localStorage.getItem("tla_studio_snapshots") || "[]")
// → [{name, source, comments, timestamp, hash}, ...]
```

### Writing a revised spec

Direct `.value =` assignment won't trigger Yew's event system. Use the native setter pattern:

```js
const ta = document.querySelector("[data-field='spec']")
const setter = Object.getOwnPropertyDescriptor(
  window.HTMLTextAreaElement.prototype,
  "value"
).set
setter.call(ta, newSpecString)
ta.dispatchEvent(new Event("input", { bubbles: true }))

// Then trigger parse
document.querySelector("[data-action='parse']").click()
```

### Recommended agent loop

```text
1. Navigate to page URL (Share URLs include preloaded spec+comments in the fragment)
2. Read: localStorage.getItem("tla_studio_source")
3. Read: JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
4. Generate revised TLA+ that addresses the comments (follow parser rules below and output only the module)
5. Write spec via native setter pattern above
6. Click: [data-action="parse"]
7. Check: document.querySelector("[data-parser-status]").dataset.parserStatus
   - "warnings" → read warnings banner, fix spec, repeat from step 5
   - "ok" → proceed to step 8
   - "no-spec" → write failed, try setter pattern again
8. Optionally click [data-action="save-snapshot"]
9. Report to user. Repeat from step 2 if further iteration requested
```

The **Agent** tab in the Prompts panel pre-bundles the current spec, all comments, and the full interface docs above into a single copyable block — paste it into any LLM or agent to bootstrap an automated session.

## Storage

All state is in the browser's `localStorage` under three keys:

| Key                    | Content                                                              |
| ---------------------- | -------------------------------------------------------------------- |
| `tla_studio_source`    | Raw TLA+ spec text                                                   |
| `tla_studio_comments`  | JSON array of `{target, author, text, category?}`                    |
| `tla_studio_snapshots` | JSON array of named snapshots (source + comments + timestamp + hash) |

## Who Should Be in the Room

Bring **decision-makers**, **people who can block**, **people affected by the outcome**, and **those with hands-on expertise**. No single person sees the whole system — the model converges where their perspectives overlap.

Collaboration happens either synchronously — projecting the tool in a meeting while stakeholders call out feedback — or asynchronously by sharing URLs and exporting workspace snapshots between participants.

**⚠️ Shared state contains details about your business processes, comments, and organizational structure. Be mindful of where you paste it.**

## License

[MIT](LICENSE) — Copyright (c) 2026 R. Cooper Snyder
