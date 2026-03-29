# TLA+ Process Studio

A 100% client-side web application for modeling any business process as a TLA+ state machine, collecting structured stakeholder feedback, and iterating toward an accurate shared model with the help of any LLM.

Built with Rust, WebAssembly, and [Yew](https://yew.rs). Nothing leaves your browser — no server, no API, no analytics, no cookies. All data lives in `localStorage`.

## What It Does

Most process documentation is wrong. Not because anyone lied, but because no single person sees the whole system. TLA+ Process Studio makes a process visible as a state machine so the people who live with it every day can inspect it together and decide what to change.

### Core Loop

1. **Generate** — Use any LLM to produce a TLA+ state machine of your process. Built-in prompts interview you about actors, flows, failures, and safety rules, then output a spec. Paste it into the editor and click **Parse**.
2. **Simulate** — Click transitions to walk the state machine step by step. At each state, ask: does this match reality? What's missing? What breaks?
3. **Comment** — Click any state to leave categorized feedback. Engineers, PMs, ops, domain experts — everyone's input is captured with structured tags (missing step, failure mode, workaround, naming, scope question).
4. **Iterate** — Copy the **Iterate** prompt, which bundles the current spec and all comments into a structured revision prompt. The LLM classifies feedback, finds gaps, and outputs a revised spec. Paste it back and repeat.

### Features

- **TLA+ Parser** — Regex-based MVP parser that extracts states, transitions, comments, and invariants from TLA+ modules. Expects state sets named `*States` or `*Stages` and a variable named `*State`.
- **State Machine Simulator** — Interactive walkthrough of parsed transitions from the current state.
- **Mermaid Diagram** — Auto-generated state diagram rendered via Mermaid.js, with dark mode support.
- **Structured Comments** — Per-state feedback with category tags and author attribution, persisted to `localStorage`.
- **Prompt Library** — Three built-in prompts: **New spec** (bootstrap interview), **Iterate** (spec + comments bundled for revision), and **Agent** (spec + comments + `localStorage` API instructions for AI agent integration).
- **Version Management** — Manual snapshots, auto-saves on parse, backup-before-destructive-action, import/export of individual versions or full workspace state as JSON.
- **Share URLs** — One-click URL generation with spec + comments encoded in the fragment hash.
- **Responsive Layout** — Dual-pane viewport-locked UI that stacks on narrow screens. Auto dark/light mode via `prefers-color-scheme`.

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
├── index.html          # HTML shell, Mermaid.js CDN, agent-readable comments
├── style.css           # Full UI styles with auto dark mode
├── compose.yaml        # Docker Compose for containerized dev
├── run.sh              # One-command local bootstrap
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

## AI Agent Integration

The app embeds instructions for AI agents (via MCP Playwright or browser DevTools) directly in the HTML and in the **Agent** prompt panel:

```js
// Read the spec
localStorage.getItem("tla_studio_source")

// Read comments
JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")

// Write a revised spec into the editor
const ta = document.querySelector(".editor-area")
const setter = Object.getOwnPropertyDescriptor(
  window.HTMLTextAreaElement.prototype,
  "value"
).set
setter.call(ta, newSpecString)
ta.dispatchEvent(new Event("input", { bubbles: true }))

// Click Parse
document.querySelector(".btn-primary").click()
```

## Storage

All state is in the browser's `localStorage` under three keys:

| Key                    | Content                                                              |
| ---------------------- | -------------------------------------------------------------------- |
| `tla_studio_source`    | Raw TLA+ spec text                                                   |
| `tla_studio_comments`  | JSON array of `{target, author, text, category?}`                    |
| `tla_studio_snapshots` | JSON array of named snapshots (source + comments + timestamp + hash) |

## Who Should Be in the Room

Bring **decision-makers**, **people who can block**, **people affected by the outcome**, and **those with hands-on expertise**. No single person sees the whole system — each sees a part, and the truth lives in the overlap.

## License

[MIT](LICENSE) — Copyright (c) 2026 R. Cooper Snyder
