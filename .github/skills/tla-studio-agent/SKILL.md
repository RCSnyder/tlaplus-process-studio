---
name: tla-studio-agent
description: "Interact with a running TLA+ Process Studio instance as an agent. Use when: automating iteration on a TLA+ process model; reading or writing a spec via browser automation; applying stakeholder comments to generate a revised spec; running an agentic loop against any deployment of TLA+ Process Studio (local, hosted, forked, or internal). Works with MCP Playwright, browser-use, Puppeteer, or any browser DevTools access. URL is always provided by the user — this skill is deployment-agnostic."
argument-hint: "URL of the running TLA+ Process Studio instance (e.g. http://localhost:8080)"
---

# TLA+ Process Studio — Agent Skill

TLA+ Process Studio is a 100% client-side web app. There is no server, no API, no auth.
All state lives in the browser's `localStorage`. Nothing is transmitted over the network.

The URL of the instance you're working with comes from the user. Do not guess or hardcode it.

---

## When to Use

- User says "update the model based on the comments" and you have browser access
- User pastes a Share URL and asks you to iterate on it
- Automating a review loop: read → analyze → revise → write → parse → check → repeat
- Any task requiring you to drive a TLA+ Process Studio instance programmatically

---

## Stable Interface (use these — not CSS classes)

The app exposes `data-*` attributes as a stable contract. CSS classes may change; these won't.

| Selector                        | Element          | Purpose                                                |
| ------------------------------- | ---------------- | ------------------------------------------------------ |
| `[data-field="spec"]`           | `<textarea>`     | TLA+ source — read current value or write revised spec |
| `[data-action="parse"]`         | `<button>`       | Triggers re-parse and diagram update                   |
| `[data-action="save-snapshot"]` | `<button>`       | Saves current spec+comments as a named version         |
| `[data-parser-status]`          | App root `<div>` | Parse result: `"ok"` \| `"warnings"` \| `"no-spec"`    |
| `[data-module]`                 | App root `<div>` | Current parsed module name                             |
| `[data-state-count]`            | App root `<div>` | Number of parsed states (string)                       |

---

## Read API

```js
// Full TLA+ spec (persisted across reloads)
localStorage.getItem("tla_studio_source")

// All stakeholder comments
JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
// Shape: [{target, author, text, category?, chain?}, ...]

// Named snapshots (version history)
JSON.parse(localStorage.getItem("tla_studio_snapshots") || "[]")
// Shape: [{name, source, comments, timestamp, hash}, ...]

// Live editor value (may differ from localStorage if unparsed edits exist)
document.querySelector("[data-field='spec']").value

// Current parse status
document.querySelector("[data-parser-status]").dataset.parserStatus
// → "ok" | "warnings" | "no-spec"
```

---

## Write API

**CRITICAL:** Direct `.value =` assignment does not fire Yew's event system. Always use the native setter pattern:

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

---

## Agent Loop (standard procedure)

```
STEP 1  Navigate to the URL provided by the user
        If the URL contains a fragment (#share=...) it has preloaded spec+comments — let it settle before reading

STEP 2  Read spec
        localStorage.getItem("tla_studio_source")

STEP 3  Read comments
        JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
        Group by `target` (state name) to understand which states have feedback

STEP 4  Analyze
        Identify what the comments are asking for. Classify per state: corrections, missing transitions,
        naming issues, failure modes, scope questions. Generate a revised TLA+ spec — follow parser
        rules exactly (see below) or the diagram will not render.

STEP 5  Write revised spec
        Use the native setter pattern (see Write API above)

STEP 6  Parse
        document.querySelector("[data-action='parse']").click()
        Wait for the DOM to settle (100–300ms)

STEP 7  Check status
        const status = document.querySelector("[data-parser-status]").dataset.parserStatus
        - "ok"       → proceed to STEP 8
        - "warnings" → read the warnings banner text, fix the spec, repeat from STEP 5
        - "no-spec"  → the write failed; try the setter pattern again

STEP 8  Save (optional but recommended)
        document.querySelector("[data-action='save-snapshot']").click()

STEP 9  Report to user. Offer to continue iterating.
```

---

## Parser Rules (must follow or diagram won't render)

- **One variable** ending in `State` (e.g. `processState`, `saleState`)
- **One state set** named ending in `States` or `Stages`
- **State names**: quoted PascalCase, no spaces — `"InReview"` not `"In Review"`
- **Every transition** must be exactly this form:
  ```
  ActionName ==
      /\ varState = "FromState"
      /\ varState' = "ToState"
  ```
- **Required**: `Init == varState = "InitialState"`
- **Required**: `Next == \/ Action1 \/ Action2 \/ ...`
- **Invariants** must use `=>`: e.g. `varState \in AllStates => TRUE`
- **Forbidden** in transition bodies: `IF/THEN/ELSE`, `CASE`, `LET/IN`, `UNCHANGED`, quantifiers, extra conjuncts

---

## Notes for Forked / Internal Deployments

This skill is URL-agnostic by design. The interface (`data-*` attrs, `localStorage` keys, setter pattern) is
identical on every deployment of TLA+ Process Studio regardless of origin. When a team forks and self-hosts,
point agents at their internal URL — this skill applies unchanged.
