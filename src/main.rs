mod model;
mod parser;

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{window, HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use model::Action;
use parser::parse_tla;

// ─── Sample TLA+ ───

const SAMPLE_TLA: &str = r#"---- MODULE CollaborativeModeling ----

(* 
  Meta-model: the collaborative process of building a TLA+ state machine
  using TLA+ Process Studio itself. This is the tool modeling its own use.
*)

VARIABLE processState

ProcessStates == {
  "No Spec",
  "Drafting Prompt",
  "Waiting on LLM",
  "Spec Loaded",
  "Simulating",
  "Commenting",
  "Reviewing Comments",
  "Iterating",
  "Consensus Reached",
  "Archived"
}

(* The facilitator opens the tool for the first time *)
StartFresh ==
  processState = "No Spec"
  /\ processState' = "Drafting Prompt"

(* Copy the bootstrap prompt and customize it for the domain *)
DraftBootstrapPrompt ==
  processState = "Drafting Prompt"
  /\ processState' = "Waiting on LLM"

(* LLM returns a TLA+ spec; paste it into the editor and click Parse *)
ReceiveSpec ==
  processState = "Waiting on LLM"
  /\ processState' = "Spec Loaded"

(* The facilitator walks through the state machine to understand it *)
BeginSimulation ==
  processState = "Spec Loaded"
  /\ processState' = "Simulating"

(* Click a transition — look at each state and ask: is this right? *)
StepThroughTransition ==
  processState = "Simulating"
  /\ processState' = "Commenting"

(* A stakeholder leaves feedback on a state: what's wrong, missing, or unclear *)
LeaveComment ==
  processState = "Commenting"
  /\ processState' = "Simulating"

(* All states have been reviewed; time to look at the full picture *)
FinishReview ==
  processState = "Simulating"
  /\ processState' = "Reviewing Comments"

(* The team decides the model isn't right yet — iterate *)
DecideToIterate ==
  processState = "Reviewing Comments"
  /\ processState' = "Iterating"

(* Copy the Iterate prompt (includes spec + all comments), send to LLM *)
SendIterationToLLM ==
  processState = "Iterating"
  /\ processState' = "Waiting on LLM"

(* The team agrees the model reflects reality *)
ReachConsensus ==
  processState = "Reviewing Comments"
  /\ processState' = "Consensus Reached"

(* Save the final version and share it *)
ArchiveModel ==
  processState = "Consensus Reached"
  /\ processState' = "Archived"

(* Reopen a completed model for further refinement *)
ReopenModel ==
  processState = "Archived"
  /\ processState' = "Spec Loaded"

(* Load a shared URL or imported version *)
LoadSharedSpec ==
  processState = "No Spec"
  /\ processState' = "Spec Loaded"

Init == processState = "No Spec"

Next ==
  \/ StartFresh
  \/ DraftBootstrapPrompt
  \/ ReceiveSpec
  \/ BeginSimulation
  \/ StepThroughTransition
  \/ LeaveComment
  \/ FinishReview
  \/ DecideToIterate
  \/ SendIterationToLLM
  \/ ReachConsensus
  \/ ArchiveModel
  \/ ReopenModel
  \/ LoadSharedSpec

===="#;

// ─── Storage keys ───

const STORAGE_KEY: &str = "tla_studio_comments";
const STORAGE_SOURCE_KEY: &str = "tla_studio_source";
const STORAGE_SNAPSHOTS_KEY: &str = "tla_studio_snapshots";

// ─── Prompt constants ───

const BOOTSTRAP_PROMPT: &str = r#"You are a business-process discovery partner. Your job is to help me build a shared model of a real-world process as a TLA+ state machine. The people who do this work every day are the experts — your role is to capture their knowledge faithfully, including the clever adaptations and hard-won experience that never make it into official documentation.

## Core principle

The state machine is not the final word — it is a **conversation artifact** that everyone can point at, argue with, and improve. Every real process has multiple perspectives: what the documentation says, what people experience day-to-day, what the data shows, and what everyone wishes would happen. The model should hold all of these perspectives honestly, not collapse them into one.

## Interview protocol

Ask these questions ONE AT A TIME, waiting for my answer before proceeding. After each answer, reflect back what you understood in one sentence so I can correct you before we move on.

Frame the conversation warmly: you're here to learn from the person who knows this process best. There are no wrong answers — if something sounds messy or contradictory, that's valuable information about the real process, not a mistake.

### Phase 1 — Context, purpose & economics
1. What is the name of this process? Give me a one-sentence purpose statement. (e.g., "Order Fulfillment — takes a customer order from placement to delivery")
2. WHY does this process exist? What business outcome does it produce? Who is the customer of this process (internal or external)?
3. Who are the ACTORS involved? For each: their role, roughly how much of their day this process consumes, and how this process affects them — is it the core of their job, a side task, or something they inherit from another team?
4. What TRIGGERS the process — what event or condition causes it to start? How often does it run? (10x/day? 3x/month?)

### Phase 2 — The process as experienced
5. Walk me through what happens step by step, as if you're narrating a typical case you handled recently. For each step: who does it, what state things are in, what they do, and what state it moves to. A recent real example is more useful than the general case.
6. At each step: Is this **control work** (deterministic, follows a rule), **coordination work** (handoffs, reminders, following up with people), or **judgment work** (requires a human to assess, interpret, or decide)?
7. At any step, has the team developed its own way of doing things that differs from the documented process? These adaptations usually exist for good reasons — they're the expertise that keeps things moving. Help me capture them.
8. What INFORMATION does each actor need at each step? Where does it come from? Is there any manual data transfer (copy-paste between systems, emailing spreadsheets, checking one system to update another)?

### Phase 3 — Exceptions & things that create extra work
9. Tell me about a recent time this process hit a significant snag. What happened, what went sideways, and how did people recover?
10. What are the common EXCEPTIONS? For each, what kind of blocker is it:
    - Missing information (data not available to proceed)
    - External delay (waiting on someone/something outside your control)
    - Resource unavailable (person, system, or capacity not available)
    - Policy ambiguity (unclear rules, judgment call required)
    - Quality issue (output of a prior step was wrong or incomplete)
    - Financial risk (amount, approval, or budget issue)
    - Safety/compliance issue (legal, regulatory, or risk blocker)
    - Customer change (scope change, cancellation, new requirements mid-process)
11. Are there steps that exist mainly to fix or clean up problems from earlier in the process? (e.g., re-entering data, clarifying miscommunications, re-doing work that was incomplete) These create extra work that might be avoidable.
12. What do experienced people know about this process that isn't written down? The instincts, the workarounds, the "if you see X, do Y" knowledge that takes months to learn.

### Phase 4 — Time, capacity & flow
13. For each state: roughly how long does work sit there? What's the fastest it moves through, and the slowest? Where does work tend to pile up?
14. Are there TIMEOUTS or ESCALATIONS — places where something expires or gets escalated if not acted on within a deadline?
15. What does DONE look like? Describe every possible end state — success, failure, abandoned, expired, archived.

### Phase 5 — Confidence & different perspectives
16. For any of your answers — how confident are you? (Very confident / mostly confident / not sure — I'd want to check with someone.) Would anyone in a different role describe this process differently?
17. Are there parts of this process where people see things differently? Where do perspectives diverge? (These are some of the most valuable things to capture — they reveal where the process is still evolving.)

### Phase 6 — Safety, invariants & technology opportunities
18. What must ALWAYS be true throughout this process? (e.g., "a payment must never be captured without authorisation")
19. What must NEVER happen? (e.g., "an order must never be shipped twice")
20. For each step, where could technology best support the people doing this work?
    - **Best left to people**: requires judgment, empathy, relationship context, or nuanced interpretation
    - **Technology can prepare**: gather the information, surface relevant history, pre-fill forms, flag anomalies — but a person decides
    - **Technology can handle with oversight**: route, remind, escalate, or execute routine steps — a person monitors and intervenes when needed
    - **Technology can fully handle**: deterministic, rule-based, well-defined inputs and outputs — no interpretation needed

## Abstraction level guidance

Target **5–12 states** on a first pass. Each state should represent a meaningful wait point, decision point, or handoff — not an individual micro-step.

If you find yourself exceeding ~15 states, the process is probably too broad for one model. Break it into sub-processes: one high-level model showing the major phases, and separate modules for each phase that needs detail. You can always decompose a single state into its own sub-process later.

**When in doubt, fewer states is better.** A compact model that everyone can see on one screen and argue about is far more valuable than a comprehensive model no one can follow.

## Output rules

Generate a complete TLA+ module. The `(* ... *)` comments on each state and transition ARE the plain-English narrative — stakeholders will read these directly in the visual tool. Make every comment tell a complete, readable story.

Follow this format EXACTLY:

```
---- MODULE <ProcessName> ----

VARIABLE <processName>State  \* e.g. orderState, hiringState, claimState

<ProcessName>Stages == {
  "State1",
  "State2",
  ...
}

(* <Plain-English story: Who does this step? What triggers it? What information do they need?
   What actually happens? What can go wrong here? How long does it typically take?
   Work type: control | coordination | judgment
   Technology: best-left-to-people | can-prepare | can-handle-with-oversight | can-fully-handle *)
TransitionName ==
  /\ <processName>State = "FromState"
  /\ <processName>State' = "ToState"

(* ADAPTATION: <How the team actually handles this and why — what triggers the workaround,
   who does it, what problem it solves. May differ from documented process.> *)
AdaptationTransitionName ==
  /\ <processName>State = "FromState"
  /\ <processName>State' = "ToState"

(* EXTRA WORK: <What upstream issue causes this? Who has to do the cleanup?
   How often does it happen? Could the root cause be eliminated?> *)
ReworkTransitionName ==
  /\ <processName>State = "FromState"
  /\ <processName>State' = "ToState"

(* INVARIANT: <Plain English safety rule> *)
InvariantName ==
  <processName>State \in <ProcessName>Stages

Init == <processName>State = "InitialState"

Next ==
  \/ TransitionName
  \/ BranchName
  ...

====
```

Variable naming rule:
- Derive the variable name from the process: `orderState` for an order process, `claimState` for an insurance claim, `hiringState` for a hiring pipeline.
- The variable MUST end in `State` (this is required for the parser).
- Use camelCase: `<lowerCaseProcess>State`.

Requirements:
- State names: human-readable noun phrases ("Awaiting Approval", not "state_3")
- Transition names: verb phrases in PascalCase ("SubmitForReview", "TimeoutToEscalation")
- Every state MUST be reachable — no orphans
- Every non-terminal state MUST have at least one outbound transition
- Add rich (* comments *) above EVERY transition — these are the primary way stakeholders read the model. Each comment should tell a complete story: what triggers the transition, who does it, what they need, what can go wrong, work type, and technology support rating
- Mark ADAPTATION transitions where the team's actual practice differs from documented process
- Mark EXTRA WORK transitions that exist to fix or clean up upstream issues
- Where perspectives differ, add (* PERSPECTIVES DIFFER: ... *) comments capturing both views
- Put failure/recovery transitions AFTER the happy path so the spec reads top-to-bottom as a narrative
- Include at least one invariant (even if it's just type-correctness)
- ---- MODULE and ==== delimiters are required
- After the module, add a brief summary: key points where perspectives differ, top 3 extra-work items that could be eliminated at source, and top 3 technology support opportunities"#;

const ITERATION_PROMPT: &str = r#"You are reviewing a TLA+ state machine and stakeholder feedback collected in TLA+ Process Studio. Your job is to revise the specification so it converges toward an accurate, shared model of the real-world process — one that reflects what people actually experience, not just what's documented.

## Core principle

Every revision should bring the model closer to how the process is actually experienced by the people in it. Stakeholder comments are the primary evidence. When people see things differently, that's a signal to preserve — not a conflict to resolve.

## Analysis framework

Work through these steps IN ORDER before generating any code:

### Step 1 — Classify every comment
For each stakeholder comment, categorise it as one of:
- **EXPERIENCE GAP**: the spec shows one thing but this comment describes a different lived experience (team adaptation, undocumented step, different sequence)
- **MISSING STATE**: a real-world condition that exists but has no state in the spec
- **MISSING TRANSITION**: a path between states that exists in practice but not the spec
- **WRONG FLOW**: transitions that don't match what actually happens
- **NAMING**: state or transition names that don't match the language people actually use
- **EXCEPTION**: an error, timeout, rejection, or edge case not modelled — classify by type: missing info / external delay / resource unavailable / policy ambiguity / quality issue / financial risk / compliance issue / customer change
- **EXTRA WORK**: this comment identifies clean-up or fix-up work that exists because of an upstream issue
- **WORK TYPE MISMATCH**: a step is modelled as deterministic but actually requires judgment, or vice versa
- **INVARIANT VIOLATION**: a safety rule that the current spec could break
- **SCOPE QUESTION**: comment is asking whether something should be in/out of scope — flag for human decision, don't auto-resolve
- **DIFFERENT PERSPECTIVES**: two or more stakeholders see this differently — preserve all views, note where they diverge

### Step 2 — Check for process-reality gaps
Even without comments, check the spec for:
- **Experience gaps**: Are there states that only exist in documentation but don't match what people describe? Any team adaptations missing?
- **Exception coverage**: Does every state that involves waiting on an external actor (person, system, approval) have timeout/error/escalation transitions?
- **Extra work**: Are there rework loops? Are they marked as such, or disguised as normal flow?
- **Work type annotations**: Is every transition annotated with control/coordination/judgment? Are any misclassified?
- **Technology support ratings**: Are support-level tags present and reasonable?
- **Time/capacity gaps**: Are there states where work piles up with no visibility? Missing SLA/timeout states?
- **Dead ends**: States with no outbound transitions that aren't clearly terminal
- **Single-entry states**: States with only one inbound path (is there really no other way to reach this state?)
- **Confidence markers**: Are there (* REVIEW: ... *) or (* PERSPECTIVES DIFFER: ... *) comments where uncertainty is high?

### Step 3 — Propose changes
List EVERY proposed change as a bullet:
- `+ ADD STATE "Name"` — reason
- `+ ADD TRANSITION ActionName: "From" → "To"` — reason (include work type + technology support rating)
- `+ ADD ADAPTATION "From" → "To"` — what the team actually does and why
- `+ ADD EXTRA WORK "From" → "To"` — upstream cause this addresses
- `~ RENAME "Old" → "New"` — reason
- `~ RECLASSIFY TransitionName: was control, now judgment` — reason
- `~ REROUTE ActionName: was "A" → "B", now "A" → "C"` — reason
- `- REMOVE STATE "Name"` — reason (only if unreachable or duplicate)
- `+ ADD INVARIANT Name` — rule in plain English
- `? PERSPECTIVES DIFFER: "description"` — stakeholders see this differently, preserve both views

### Step 4 — Output the revised module

Generate a COMPLETE revised TLA+ module (not a diff). The `(* ... *)` comments on each transition ARE the plain-English narrative — stakeholders read these directly in the visual tool. Make every comment tell a complete, readable story.

Also check abstraction level: if the model is growing beyond ~15 states, suggest breaking it into sub-processes rather than adding more states. Fewer states that everyone can see and argue about beats a comprehensive model no one can follow.

Follow the exact format:
- ---- MODULE Name ---- and ==== delimiters
- State set as a set literal
- Each transition as a named operator with /\ <processName>State = "From" /\ <processName>State' = "To"
- The variable name must match the bootstrap output (e.g. orderState, not processState)
- (* comments *) above every transition — these are the primary way stakeholders read the model. Each comment should tell a complete story: what triggers it, who does it, work type, technology support rating
- Mark ADAPTATION transitions where the team's practice differs from documentation
- Mark EXTRA WORK transitions that exist to fix upstream issues
- (* PERSPECTIVES DIFFER: ... *) comments capturing multiple views
- (* REVIEW: ... *) comments on anything uncertain
- All invariants at the bottom
- Happy path transitions first, then branches, then failure/recovery paths

### Step 5 — Summary
After the module, provide:
- **Changes made**: bullet list of what changed and why
- **Different perspectives**: items stakeholders see differently (preserve all views — do not pick a winner)
- **Remaining gaps**: what the spec still doesn't capture
- **Top extra-work items**: upstream issues that create avoidable work downstream
- **Top technology support opportunities**: transitions where technology could best support the people doing this work

## Important
- Do NOT remove states or transitions that no comment complained about — preserve existing correct structure
- Do NOT resolve SCOPE QUESTION or DIFFERENT PERSPECTIVES comments yourself — note them for the humans to discuss
- If a comment is ambiguous, add the most likely interpretation AND flag it with a (* REVIEW: ... *) comment
- Preserve all existing (* comments *) and add new ones for new transitions
- When stakeholders see things differently, model ALL perspectives and flag with (* PERSPECTIVES DIFFER: ... *)

Here is the current state machine and collected feedback:

"#;

const AGENT_QUERY_INSTRUCTIONS: &str = r#"## How to query TLA+ Process Studio from an AI agent

This application is 100% client-side. All state lives in the browser's localStorage.
No server, no API, no cookies, no analytics. Nothing is transmitted over the network.

### Reading state (via MCP Playwright, browser DevTools, or similar):

1. Read the TLA+ specification:
   localStorage.getItem("tla_studio_source")

2. Read all stakeholder comments (JSON array):
   JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
   // Returns: [{target: "StateName", author: "Name", text: "Comment"}, ...]

3. Read the editor's current (possibly unsaved) content:
   document.querySelector(".editor-area").value

### Writing state back:

4. Set a revised spec in the editor:
   const ta = document.querySelector(".editor-area");
   const setter = Object.getOwnPropertyDescriptor(
     window.HTMLTextAreaElement.prototype, 'value'
   ).set;
   setter.call(ta, newSpecString);
   ta.dispatchEvent(new Event('input', { bubbles: true }));

5. Click Parse to update the visualization:
   document.querySelector(".btn-primary").click()

### Recommended agent workflow:
   a. Navigate to the page URL
   b. Read spec via localStorage.getItem("tla_studio_source")
   c. Read comments via JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
   d. Analyze spec + comments, generate revised TLA+
   e. Write revised spec back to editor (step 4 above)
   f. Click Parse (step 5 above)
   g. Repeat iteration cycle"#;

// ─── Data types ───

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct UserComment {
    target: String,
    author: String,
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    category: Option<String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Snapshot {
    name: String,
    source: String,
    comments: Vec<UserComment>,
    timestamp: f64,
    hash: u64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ExportArtifact {
    version: u32,
    name: String,
    source: String,
    comments: Vec<UserComment>,
    hash: u64,
    timestamp: f64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct SharePayload {
    source: String,
    comments: Vec<UserComment>,
}

// ─── Storage helpers ───

fn get_storage() -> Option<web_sys::Storage> {
    window().and_then(|w| w.local_storage().ok().flatten())
}

fn load_comments() -> Vec<UserComment> {
    get_storage()
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn save_comments(comments: &[UserComment]) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(comments) {
            let _ = storage.set_item(STORAGE_KEY, &json);
        }
    }
}

fn save_source(source: &str) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(STORAGE_SOURCE_KEY, source);
    }
}

fn storage_size_kb() -> f64 {
    let s = match get_storage() { Some(s) => s, None => return 0.0 };
    let mut bytes = 0usize;
    for key in &[STORAGE_KEY, STORAGE_SOURCE_KEY, STORAGE_SNAPSHOTS_KEY] {
        if let Ok(Some(val)) = s.get_item(key) {
            bytes += val.len();
        }
    }
    bytes as f64 / 1024.0
}

fn load_source() -> Option<String> {
    get_storage().and_then(|s| s.get_item(STORAGE_SOURCE_KEY).ok().flatten())
}

fn clear_stored_comments() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY);
    }
}

fn load_snapshots() -> Vec<Snapshot> {
    get_storage()
        .and_then(|s| s.get_item(STORAGE_SNAPSHOTS_KEY).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn save_snapshots(snaps: &[Snapshot]) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(snaps) {
            let _ = storage.set_item(STORAGE_SNAPSHOTS_KEY, &json);
        }
    }
}

/// Create a backup snapshot before a destructive action.
/// Returns the updated snapshot list (already persisted).
fn save_backup_snapshot(reason: &str, source: &str, comments: &[UserComment], existing: &[Snapshot]) -> Vec<Snapshot> {
    let mut snaps = existing.to_vec();
    let prefix = "Backup: ";
    snaps.push(Snapshot {
        name: format!("{}{} ({})", prefix, reason, format_ts(now_ms())),
        source: source.to_string(),
        comments: comments.to_vec(),
        timestamp: now_ms(),
        hash: hash_source(source),
    });
    // Cap backup snapshots at 10
    let backup_count = snaps.iter().filter(|s| s.name.starts_with(prefix)).count();
    if backup_count > 10 {
        if let Some(pos) = snaps.iter().position(|s| s.name.starts_with(prefix)) {
            snaps.remove(pos);
        }
    }
    save_snapshots(&snaps);
    snaps
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn format_ts(ms: f64) -> String {
    let d = js_sys::Date::new(&JsValue::from_f64(ms));
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
    )
}

// ─── Share URL helpers ───

fn build_share_url(source: &str, comments: &[UserComment]) -> Option<String> {
    let payload = SharePayload { source: source.to_string(), comments: comments.to_vec() };
    let json = serde_json::to_string(&payload).ok()?;
    let encoded = js_sys::encode_uri_component(&json);
    let loc = window()?.location();
    let origin = loc.origin().ok()?;
    let pathname = loc.pathname().ok()?;
    Some(format!("{}{}#share={}", origin, pathname, encoded))
}

fn load_share_from_url() -> Option<SharePayload> {
    let hash = window()?.location().hash().ok()?;
    let prefix = "#share=";
    if !hash.starts_with(prefix) { return None; }
    let encoded = &hash[prefix.len()..];
    let decoded = js_sys::decode_uri_component(encoded).ok()?.as_string()?;
    serde_json::from_str(&decoded).ok()
}

fn clear_url_hash() {
    if let Some(window) = window() {
        let _ = window.history().and_then(|h| h.replace_state_with_url(&JsValue::NULL, "", Some(&window.location().pathname().unwrap_or_default())));
    }
}

// ─── Category-aware comment prompts ───

fn category_cta(cat: &Option<String>, _lib: &str) -> String {
    match cat.as_deref() {
        Some("Missing step") =>
            "\u{1F50D} Look at this state in the model. Is there a step that actually happens here that isn\u{2019}t shown? Who does it, what triggers it, and what state does it move to? If you\u{2019}ve seen it happen even once, it\u{2019}s worth capturing."
                .to_string(),
        Some("Not what happens") =>
            "\u{26A0}\u{FE0F} The model says one thing \u{2014} what actually happens? Walk through the real sequence from this state. Where exactly does it diverge? Did it used to match and change over time, or was the model always wrong here?"
                .to_string(),
        Some("Failure mode") =>
            "\u{1F4A5} How does this process actually fail at this point? What triggers the failure, how often does it happen, and what\u{2019}s the blast radius? Is there a recovery path today, or does someone just deal with it?"
                .to_string(),
        Some("Workaround") =>
            "\u{1F527} Is there an unofficial way people actually handle this? What triggers the workaround \u{2014} always, or only when something breaks? Who knows how to do it? This is hard-won expertise worth capturing."
                .to_string(),
        Some("Naming") =>
            "\u{1F3F7}\u{FE0F} What does your team actually call this? If different people or teams use different names for the same thing, list all of them \u{2014} naming mismatches often reveal real misunderstandings."
                .to_string(),
        Some("Scope question") =>
            "\u{2753} Should this be in the model or out of scope? What depends on this decision? If we exclude it, what gap does that leave? If we include it, does the model get too big? (Consider: could it be its own sub-process instead?)"
                .to_string(),
        _ =>
            "\u{1F4AC} Look at this state. Does it match reality? What\u{2019}s missing, oversimplified, or just wrong? Pick a tag above to focus your feedback, or write freely \u{2014} your experience is the evidence this model needs."
                .to_string(),
    }
}

fn category_placeholder(cat: &Option<String>, _lib: &str) -> String {
    match cat.as_deref() {
        Some("Missing step") =>
            "What step happens here that the model doesn\u{2019}t show? Who does it, what triggers it, what state does it move to?"
                .to_string(),
        Some("Not what happens") =>
            "What actually happens here vs. what\u{2019}s shown? Where exactly does the real sequence diverge from the model?"
                .to_string(),
        Some("Failure mode") =>
            "How does this fail? What triggers it, how often, what\u{2019}s the blast radius, and how is it handled today?"
                .to_string(),
        Some("Workaround") =>
            "What do people actually do here that differs from the model? What triggers it and who knows how?"
                .to_string(),
        Some("Naming") =>
            "What does your team call this? List every name different people or teams use for it..."
                .to_string(),
        Some("Scope question") =>
            "In or out of scope? What depends on this decision? Could it be its own sub-process instead?"
                .to_string(),
        _ =>
            "What\u{2019}s true about this state that the model doesn\u{2019}t capture? Anything missing, wrong, or oversimplified..."
                .to_string(),
    }
}

// ─── Export builders ───

fn build_export(source: &str, comments: &[UserComment], states: &[String], iter_prompt: &str) -> String {
    let mut out = String::new();
    out.push_str(iter_prompt);
    out.push_str("=== TLA+ Source ===\n\n");
    out.push_str(source);
    out.push_str("\n\n=== Stakeholder Comments ===\n");
    for state in states {
        let sc: Vec<&UserComment> = comments.iter().filter(|c| c.target == *state).collect();
        if !sc.is_empty() {
            out.push_str(&format!("\n[{}]\n", state));
            for c in &sc {
                if let Some(ref cat) = c.category {
                    out.push_str(&format!("  [{}] ({}) {}\n", cat.to_uppercase(), c.author, c.text));
                } else {
                    out.push_str(&format!("  ({}) {}\n", c.author, c.text));
                }
            }
        }
    }
    if comments.is_empty() {
        out.push_str("\n(No comments collected yet — the spec may still need review)\n");
    }
    out.push_str("\n=== End ===\n");
    out
}

fn build_agent_meta(source: &str, comments: &[UserComment], states: &[String], source_hash: u64) -> String {
    let mut meta = String::new();
    meta.push_str("<!-- AGENT-READABLE: TLA+ Process Studio State -->\n");
    meta.push_str(&format!("<!-- spec_hash: {} | state_count: {} | comment_count: {} -->\n", source_hash, states.len(), comments.len()));
    meta.push_str("<!-- Read instructions: see 'Agent query instructions' section -->\n\n");
    meta.push_str("=== SPEC ===\n");
    meta.push_str(source);
    meta.push_str("\n=== COMMENTS ===\n");
    for state in states {
        let sc: Vec<&UserComment> = comments.iter().filter(|c| c.target == *state).collect();
        if !sc.is_empty() {
            meta.push_str(&format!("[{}]\n", state));
            for c in &sc {
                if let Some(ref cat) = c.category {
                    meta.push_str(&format!("  [{}] ({}) {}\n", cat.to_uppercase(), c.author, c.text));
                } else {
                    meta.push_str(&format!("  ({}) {}\n", c.author, c.text));
                }
            }
        }
    }
    meta.push_str("\n=== AGENT QUERY INSTRUCTIONS ===\n");
    meta.push_str(AGENT_QUERY_INSTRUCTIONS);
    meta.push_str("\n=== END ===\n");
    meta
}

fn build_file_artifact(name: &str, source: &str, comments: &[UserComment], hash: u64) -> String {
    let artifact = ExportArtifact {
        version: 1,
        name: name.to_string(),
        source: source.to_string(),
        comments: comments.to_vec(),
        hash,
        timestamp: now_ms(),
    };
    serde_json::to_string_pretty(&artifact).unwrap_or_default()
}

fn parse_file_artifact(json: &str) -> Option<ExportArtifact> {
    serde_json::from_str(json).ok()
}

// ─── Workspace export/import ───

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct WorkspaceExport {
    version: u32,
    source: String,
    comments: Vec<UserComment>,
    snapshots: Vec<Snapshot>,
    source_hash: u64,
    timestamp: f64,
}

fn build_workspace_export(source: &str, comments: &[UserComment], snapshots: &[Snapshot], source_hash: u64) -> String {
    let ws = WorkspaceExport {
        version: 1,
        source: source.to_string(),
        comments: comments.to_vec(),
        snapshots: snapshots.to_vec(),
        source_hash,
        timestamp: now_ms(),
    };
    serde_json::to_string_pretty(&ws).unwrap_or_default()
}

fn parse_workspace_export(json: &str) -> Option<WorkspaceExport> {
    serde_json::from_str(json).ok()
}

// ─── App component ───

#[function_component(App)]
fn app() -> Html {
    // Check for share URL on first render
    let shared = use_state(|| {
        if let Some(payload) = load_share_from_url() {
            clear_url_hash();
            save_source(&payload.source);
            save_comments(&payload.comments);
            Some(payload)
        } else {
            None
        }
    });
    let source = use_state(|| {
        if let Some(ref p) = *shared { p.source.clone() }
        else { load_source().unwrap_or_else(|| SAMPLE_TLA.to_string()) }
    });
    let last_source_hash = use_state(|| {
        if let Some(ref p) = *shared { hash_source(&p.source) }
        else { hash_source(&load_source().unwrap_or_else(|| SAMPLE_TLA.to_string())) }
    });
    let parsed = {
        let source_text = (*source).clone();
        use_memo(source_text, |text| parse_tla(text))
    };

    let sim_state = use_state(|| parsed.states.first().cloned().unwrap_or_else(|| "No Spec".to_string()));
    let comments = use_state(|| {
        if let Some(ref p) = *shared { p.comments.clone() }
        else { load_comments() }
    });
    let comment_target = use_state(|| Option::<String>::None);
    let comment_draft = use_state(String::new);
    let comment_author = use_state(|| "Anonymous".to_string());
    let comment_category = use_state(|| Option::<String>::None);
    let active_tab = use_state(|| "model".to_string());
    let active_panel = use_state(|| Option::<String>::None);
    let snapshots = use_state(load_snapshots);
    let snap_name = use_state(String::new);
    let import_text = use_state(String::new);
    let import_msg = use_state(|| Option::<String>::None);
    let ws_import_text = use_state(String::new);
    let ws_import_msg = use_state(|| Option::<String>::None);

    // ── Source editing ──
    let on_source = {
        let source = source.clone();
        Callback::from(move |e: InputEvent| {
            let ta: HtmlTextAreaElement = e.target_unchecked_into();
            let val = ta.value();
            save_source(&val);
            source.set(val);
        })
    };

    // ── Parse ──
    let on_parse = {
        let source = source.clone();
        let last_source_hash = last_source_hash.clone();
        let comments = comments.clone();
        let snapshots = snapshots.clone();
        let sim_state = sim_state.clone();
        let comment_target = comment_target.clone();
        Callback::from(move |_| {
            save_source(&source);
            let new_hash = hash_source(&source);
            if new_hash != *last_source_hash {
                // Auto-save previous state before overwriting
                let prev_source = load_source().unwrap_or_default();
                if !prev_source.trim().is_empty() {
                    let mut snaps = (*snapshots).clone();
                    snaps.push(Snapshot {
                        name: format!("Auto-save {}", format_ts(now_ms())),
                        source: prev_source,
                        comments: (*comments).clone(),
                        timestamp: now_ms(),
                        hash: *last_source_hash,
                    });
                    // Keep only the 20 most recent auto-saves
                    let auto_count = snaps.iter().filter(|s| s.name.starts_with("Auto-save ")).count();
                    if auto_count > 20 {
                        if let Some(pos) = snaps.iter().position(|s| s.name.starts_with("Auto-save ")) {
                            snaps.remove(pos);
                        }
                    }
                    save_snapshots(&snaps);
                    snapshots.set(snaps);
                }
                comments.set(Vec::new());
                clear_stored_comments();
                comment_target.set(None);
                last_source_hash.set(new_hash);
            }
            let machine = parse_tla(&source);
            let first = machine.states.first().cloned().unwrap_or_else(|| "No Spec".to_string());
            sim_state.set(first);
        })
    };

    // ── Comment handlers ──
    let on_author = {
        let comment_author = comment_author.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let val = input.value();
            comment_author.set(if val.trim().is_empty() { "Anonymous".to_string() } else { val });
        })
    };
    let on_comment_draft = {
        let comment_draft = comment_draft.clone();
        Callback::from(move |e: InputEvent| {
            let ta: HtmlTextAreaElement = e.target_unchecked_into();
            comment_draft.set(ta.value());
        })
    };
    let on_submit_comment = {
        let comments = comments.clone();
        let comment_target = comment_target.clone();
        let comment_draft = comment_draft.clone();
        let comment_author = comment_author.clone();
        let comment_category = comment_category.clone();
        Callback::from(move |_| {
            let text = (**comment_draft).trim().to_string();
            if text.is_empty() { return; }
            if let Some(ref target) = *comment_target {
                let mut next = (*comments).clone();
                next.push(UserComment {
                    target: target.clone(),
                    author: (*comment_author).clone(),
                    text,
                    category: (*comment_category).clone(),
                });
                save_comments(&next);
                comments.set(next);
                comment_draft.set(String::new());
                comment_category.set(None);
            }
        })
    };
    let on_close_comment = {
        let comment_target = comment_target.clone();
        Callback::from(move |_| { comment_target.set(None); })
    };
    let on_clear_comments = {
        let comments = comments.clone();
        let comment_target = comment_target.clone();
        let source = source.clone();
        let snapshots = snapshots.clone();
        Callback::from(move |_| {
            let confirmed = web_sys::window()
                .and_then(|w| w.confirm_with_message("Clear all comments? A backup will be saved automatically.").ok())
                .unwrap_or(false);
            if !confirmed { return; }
            let new_snaps = save_backup_snapshot("before clearing comments", &source, &comments, &snapshots);
            snapshots.set(new_snaps);
            comments.set(Vec::new());
            clear_stored_comments();
            comment_target.set(None);
        })
    };

    // ── Sim ──
    let on_reset = {
        let sim_state = sim_state.clone();
        let parsed = parsed.clone();
        Callback::from(move |_| {
            sim_state.set(parsed.states.first().cloned().unwrap_or_else(|| "No Spec".to_string()));
        })
    };

    // ── Tabs (clicking a tab closes any open panel) ──
    let on_tab_model = { let t = active_tab.clone(); let p = active_panel.clone(); Callback::from(move |_| { t.set("model".to_string()); p.set(None); }) };
    let on_tab_reference = { let t = active_tab.clone(); let p = active_panel.clone(); Callback::from(move |_| { t.set("reference".to_string()); p.set(None); }) };
    let on_tab_diagram = { let t = active_tab.clone(); let p = active_panel.clone(); Callback::from(move |_| { t.set("diagram".to_string()); p.set(None); }) };

    // ── Panel toggles (toolbar buttons) ──
    let make_panel_toggle = |name: &'static str| -> Callback<MouseEvent> {
        let p = active_panel.clone();
        Callback::from(move |_| {
            if (*p).as_deref() == Some(name) { p.set(None); } else { p.set(Some(name.to_string())); }
        })
    };
    let on_panel_prompts = make_panel_toggle("prompts");
    let on_panel_versions = make_panel_toggle("versions");

    // ── Share ──
    let on_share = {
        let source = source.clone();
        let comments = comments.clone();
        Callback::from(move |_| {
            if let Some(url) = build_share_url(&source, &comments) {
                copy_to_clipboard(&url);
            }
        })
    };

    // ── Snapshot: save ──
    let on_snap_name = {
        let snap_name = snap_name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            snap_name.set(input.value());
        })
    };
    let on_save_snapshot = {
        let source = source.clone();
        let comments = comments.clone();
        let snapshots = snapshots.clone();
        let snap_name = snap_name.clone();
        let last_source_hash = last_source_hash.clone();
        Callback::from(move |_| {
            let ts = format_ts(now_ms());
            let name = if (*snap_name).trim().is_empty() {
                format!("Snapshot {}", ts)
            } else {
                format!("{} ({})", (*snap_name).trim(), ts)
            };
            let mut snaps = (*snapshots).clone();
            snaps.push(Snapshot {
                name,
                source: (*source).clone(),
                comments: (*comments).clone(),
                timestamp: now_ms(),
                hash: *last_source_hash,
            });
            save_snapshots(&snaps);
            snapshots.set(snaps);
            snap_name.set(String::new());
        })
    };

    // ── Snapshot: load (generate callbacks per index) ──
    // We'll render load/delete buttons inline and use index closures.

    // ── Import from text ──
    let on_import_text = {
        let import_text = import_text.clone();
        Callback::from(move |e: InputEvent| {
            let ta: HtmlTextAreaElement = e.target_unchecked_into();
            import_text.set(ta.value());
        })
    };
    let on_import = {
        let import_text = import_text.clone();
        let snapshots = snapshots.clone();
        let import_msg = import_msg.clone();
        Callback::from(move |_| {
            let text = (*import_text).trim().to_string();
            if text.is_empty() { import_msg.set(Some("Paste an artifact JSON first.".to_string())); return; }
            match parse_file_artifact(&text) {
                Some(artifact) => {
                    let ts = format_ts(now_ms());
                    let name = format!("{} (imported {})", artifact.name, ts);
                    let n_comments = artifact.comments.len();
                    let mut snaps = (*snapshots).clone();
                    snaps.push(Snapshot {
                        name: name.clone(),
                        source: artifact.source,
                        comments: artifact.comments,
                        timestamp: now_ms(),
                        hash: artifact.hash,
                    });
                    save_snapshots(&snaps);
                    snapshots.set(snaps);
                    import_msg.set(Some(format!("Saved as \"{}\" ({} comments)", name, n_comments)));
                    import_text.set(String::new());
                }
                None => { import_msg.set(Some("Invalid artifact JSON. Expected format from Copy.".to_string())); }
            }
        })
    };

    // ── Workspace export/import ──
    let ws_export_json = build_workspace_export(&source, &comments, &snapshots, *last_source_hash);
    let on_ws_export = {
        let ws_export_json = ws_export_json.clone();
        Callback::from(move |_| {
            copy_to_clipboard(&ws_export_json);
        })
    };
    let on_ws_import_text = {
        let ws_import_text = ws_import_text.clone();
        Callback::from(move |e: InputEvent| {
            let ta: HtmlTextAreaElement = e.target_unchecked_into();
            ws_import_text.set(ta.value());
        })
    };
    let on_ws_import = {
        let ws_import_text = ws_import_text.clone();
        let ws_import_msg = ws_import_msg.clone();
        let source = source.clone();
        let last_source_hash = last_source_hash.clone();
        let comments = comments.clone();
        let snapshots = snapshots.clone();
        let sim_state = sim_state.clone();
        let comment_target = comment_target.clone();
        Callback::from(move |_| {
            let text = (*ws_import_text).trim().to_string();
            if text.is_empty() { ws_import_msg.set(Some("Paste a workspace JSON first.".to_string())); return; }
            match parse_workspace_export(&text) {
                Some(ws) => {
                    // Restore source
                    save_source(&ws.source);
                    source.set(ws.source.clone());
                    last_source_hash.set(ws.source_hash);
                    // Restore comments
                    save_comments(&ws.comments);
                    comments.set(ws.comments);
                    // Restore snapshots
                    save_snapshots(&ws.snapshots);
                    let n_snaps = ws.snapshots.len();
                    snapshots.set(ws.snapshots);
                    // Reset sim
                    let machine = parse_tla(&ws.source);
                    sim_state.set(machine.states.first().cloned().unwrap_or_default());
                    comment_target.set(None);
                    ws_import_msg.set(Some(format!("Workspace restored ({} versions)", n_snaps)));
                    ws_import_text.set(String::new());
                    show_toast("Restored!");
                }
                None => { ws_import_msg.set(Some("Invalid workspace JSON.".to_string())); }
            }
        })
    };

    // ── Computed values ──
    let lib = "business";
    let active_bootstrap = BOOTSTRAP_PROMPT;
    let active_iteration = ITERATION_PROMPT;
    let export_text = build_export(&source, &comments, &parsed.states, active_iteration);
    let agent_meta_text = build_agent_meta(&source, &comments, &parsed.states, *last_source_hash);
    let _file_artifact = build_file_artifact("current", &source, &comments, *last_source_hash);
    let current_sim = (*sim_state).clone();
    let available: Vec<&Action> = parsed.actions.iter().filter(|a| a.from.contains(&current_sim)).collect();
    let grouped = group_actions_by_from(parsed.actions.clone());
    let storage_count = (*comments).len();
    let tab = (*active_tab).clone();
    let panel = (*active_panel).clone();
    let mermaid_code = build_mermaid(&parsed);

    // Trigger mermaid rendering when Diagram tab is active
    {
        let mermaid_code = mermaid_code.clone();
        let tab = tab.clone();
        let panel = panel.clone();
        use_effect_with((tab, panel, mermaid_code), move |(tab, panel, code)| {
            if tab == "diagram" && panel.is_none() && !code.is_empty() {
                let code = code.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let window = web_sys::window().unwrap();
                    let render_fn = js_sys::Reflect::get(&window, &JsValue::from_str("renderMermaid")).ok();
                    if let Some(func) = render_fn {
                        if let Ok(f) = func.dyn_into::<js_sys::Function>() {
                            let _ = f.call1(&JsValue::NULL, &JsValue::from_str(&code));
                        }
                    }
                });
            }
            || ()
        });
    }

    // helper: is a panel button "active"?
    let pb = |name: &str| -> &str {
        if panel.as_deref() == Some(name) { "tbtn tbtn-active" } else { "tbtn" }
    };

    // ─── RENDER ───
    html! {
        <div class="app-shell">

            // ── Topbar ──
            <header class="topbar">
                <div class="topbar-left">
                    <h1 class="app-title">{"TLA+ Process Studio"}</h1>
                    <span class="privacy-badge">{"100% client-side \u{00B7} nothing leaves your browser"}</span>
                </div>
            </header>

            // ── Workspace ──
            <div class="workspace">

                // ── Left pane: editor ──
                <div class="pane pane-left">
                    <div class="pane-header">
                        <span class="pane-label">{"Source"}</span>
                        <div class="toolbar">
                            <button class="btn btn-primary" onclick={on_parse}>{"Parse"}</button>
                            { if storage_count > 0 {
                                html! { <button class="btn btn-danger" onclick={on_clear_comments}>{"Clear comments"}</button> }
                            } else { html!{} } }
                        </div>
                    </div>
                    <div class="pane-body">
                        <textarea class="code-area editor-area" value={(*source).clone()} oninput={on_source} spellcheck="false" />
                    </div>
                </div>

                // ── Right pane ──
                <div class="pane pane-right">
                    <div class="tab-bar">
                        <button class={if tab == "model" && panel.is_none() { "tab tab-active" } else { "tab" }} onclick={on_tab_model}>{"Model"}</button>
                        <button class={if tab == "reference" && panel.is_none() { "tab tab-active" } else { "tab" }} onclick={on_tab_reference}>{"Reference"}</button>
                        <button class={if tab == "diagram" && panel.is_none() { "tab tab-active" } else { "tab" }} onclick={on_tab_diagram}>{"Diagram"}</button>
                        <div class="tab-spacer" />
                        <button class={pb("prompts")} onclick={on_panel_prompts}>{"Prompts"}</button>
                        <button class={pb("versions")} onclick={on_panel_versions}>{format!("Versions{}", if (*snapshots).is_empty() { String::new() } else { format!(" ({})", (*snapshots).len()) })}</button>
                        <button class="tbtn" onclick={on_share}>{"\u{1F517} Share"}</button>
                    </div>
                    <div class="tab-content">

                        // ═══ PANEL: Prompts (New spec + Iterate + Agent) ═══
                        { if panel.as_deref() == Some("prompts") {
                            let bootstrap_text = active_bootstrap.to_string();
                            let bootstrap_copy = active_bootstrap.to_string();
                            let boot_desc = "Interviews you about actors, flows, failures & safety rules, then generates a TLA+ spec.";
                            let iter_desc = "Spec + stakeholder comments bundled as a structured revision prompt. Classifies feedback, finds gaps, outputs revised spec.";
                            html! {
                            <div class="tab-panel panel-fill">
                                <div class="section" style="padding:10px 14px 6px">
                                    <div class="section-bar" style="margin-bottom: 4px">
                                        <p class="help-text" style="margin:0">{"Copy a prompt into any LLM or AI agent. Paste the result back into the editor and click Parse."}</p>
                                    </div>
                                </div>
                                <div class="prompt-columns">
                                    <section class="prompt-col">
                                        <div class="section-bar">
                                            <h3 class="prompt-col-title">{"New spec"}</h3>
                                            <div class="copy-wrap">
                                                <button class="btn btn-primary" onclick={Callback::from(move |_| copy_to_clipboard(&bootstrap_copy))}>{"Copy"}</button>
                                            </div>
                                        </div>
                                        <span class="prompt-col-desc">{boot_desc}</span>
                                        <textarea class="code-area panel-textarea" readonly=true value={bootstrap_text} />
                                    </section>
                                    <section class="prompt-col">
                                        <div class="section-bar">
                                            <h3 class="prompt-col-title">{"Iterate"}</h3>
                                            <div class="copy-wrap">
                                                <button class="btn btn-primary" onclick={{
                                                    let et = export_text.clone();
                                                    Callback::from(move |_| copy_to_clipboard(&et))
                                                }}>{"Copy"}</button>
                                            </div>
                                        </div>
                                        <span class="prompt-col-desc">{iter_desc}</span>
                                        <textarea class="code-area panel-textarea" readonly=true value={export_text} />
                                    </section>
                                    <section class="prompt-col">
                                        <div class="section-bar">
                                            <h3 class="prompt-col-title">{"Agent"}</h3>
                                            <div class="copy-wrap">
                                                <button class="btn btn-primary" onclick={{
                                                    let am = agent_meta_text.clone();
                                                    Callback::from(move |_| copy_to_clipboard(&am))
                                                }}>{"Copy"}</button>
                                            </div>
                                        </div>
                                        <span class="prompt-col-desc">{"Spec, comments + localStorage instructions for AI agents."}</span>
                                        <textarea class="code-area panel-textarea" readonly=true value={agent_meta_text} />
                                    </section>
                                </div>
                            </div>
                        }} else { html!{} } }

                        // ═══ PANEL: Versions ═══
                        { if panel.as_deref() == Some("versions") {
                            let kb = storage_size_kb();
                            let size_class = if kb > 4096.0 { "size-pill size-warn" } else { "size-pill" };
                            let size_label = if kb >= 1024.0 { format!("{:.1} MB stored", kb / 1024.0) } else { format!("{:.0} KB stored", kb) };
                            html! {
                            <div class="tab-panel panel-fill">
                                <section class="section panel-stretch">
                                    <div class="section-bar">
                                        <h2 class="section-title" style="margin:0">{"Versions"}<span class={size_class}>{size_label}</span></h2>
                                        <div class="toolbar">
                                            <input class="input-sm snap-name-input" placeholder="Name (optional)" value={(*snap_name).clone()} oninput={on_snap_name} />
                                            <button class="btn btn-primary" onclick={on_save_snapshot}>{"Save current"}</button>
                                        </div>
                                    </div>

                                    { if (*snapshots).is_empty() {
                                        html! { <p class="help-text">{"No saved versions yet. Save the current spec + comments, copy to share, or paste one to import."}</p> }
                                    } else { html! {
                                        <div class="snap-list">
                                            { for (*snapshots).iter().enumerate().map(|(i, snap)| {
                                                let source = source.clone();
                                                let comments = comments.clone();
                                                let last_source_hash = last_source_hash.clone();
                                                let sim_state = sim_state.clone();
                                                let comment_target = comment_target.clone();
                                                let snap_source = snap.source.clone();
                                                let snap_comments = snap.comments.clone();
                                                let snap_hash = snap.hash;
                                                let snapshots_load = snapshots.clone();
                                                let cur_source_load = source.clone();
                                                let cur_comments_load = comments.clone();
                                                let on_load = Callback::from(move |_| {
                                                    // Auto-backup current state before overwriting
                                                    let cur_src = (*cur_source_load).clone();
                                                    if !cur_src.trim().is_empty() {
                                                        let new_snaps = save_backup_snapshot("before loading version", &cur_src, &cur_comments_load, &snapshots_load);
                                                        snapshots_load.set(new_snaps);
                                                    }
                                                    source.set(snap_source.clone());
                                                    save_source(&snap_source);
                                                    save_comments(&snap_comments);
                                                    comments.set(snap_comments.clone());
                                                    last_source_hash.set(snap_hash);
                                                    let machine = parse_tla(&snap_source);
                                                    sim_state.set(machine.states.first().cloned().unwrap_or_default());
                                                    comment_target.set(None);
                                                    show_toast("Loaded!");
                                                });
                                                let snap_artifact = build_file_artifact(&snap.name, &snap.source, &snap.comments, snap.hash);
                                                let on_copy = Callback::from(move |_| {
                                                    copy_to_clipboard(&snap_artifact);
                                                });
                                                let snap_name_del = snap.name.clone();
                                                let snapshots_del = snapshots.clone();
                                                let on_delete = Callback::from(move |_| {
                                                    let msg = format!("Permanently delete \"{}\"?", snap_name_del);
                                                    let confirmed = web_sys::window()
                                                        .and_then(|w| w.confirm_with_message(&msg).ok())
                                                        .unwrap_or(false);
                                                    if !confirmed { return; }
                                                    let mut s = (*snapshots_del).clone();
                                                    if i < s.len() { s.remove(i); }
                                                    save_snapshots(&s);
                                                    snapshots_del.set(s);
                                                });
                                                html! {
                                                    <div class={if snap.name.starts_with("Backup: ") || snap.name.starts_with("Auto-save ") { "snap-item snap-auto" } else { "snap-item" }}>
                                                        <div class="snap-info">
                                                            <span class="snap-item-name">{&snap.name}</span>
                                                            <span class="snap-item-meta">{format!("{} \u{00B7} {} comments", format_ts(snap.timestamp), snap.comments.len())}</span>
                                                        </div>
                                                        <div class="snap-actions">
                                                            <button class="btn btn-secondary" onclick={on_load}>{"Load"}</button>
                                                            <button class="btn btn-ghost" onclick={on_copy}>{"Copy"}</button>
                                                            <button class="btn btn-ghost" onclick={on_delete}>{"Del"}</button>
                                                        </div>
                                                    </div>
                                                }
                                            }) }
                                        </div>
                                    }} }

                                    <div class="versions-import">
                                        <h3 class="section-title" style="margin:0;font-size:14px">{"Import a version"}</h3>
                                        <div class="import-row">
                                            <textarea class="code-area import-area" placeholder="Paste exported version JSON here..." value={(*import_text).clone()} oninput={on_import_text} />
                                            <button class="btn btn-primary" onclick={on_import}>{"Import"}</button>
                                        </div>
                                        { if let Some(ref msg) = *import_msg {
                                            html! { <span class="import-msg">{msg}</span> }
                                        } else { html!{} } }
                                    </div>

                                    <div class="versions-import">
                                        <h3 class="section-title" style="margin:0;font-size:14px">{"Full workspace backup"}</h3>
                                        <div class="import-row">
                                            <button class="btn btn-secondary copy-wrap" onclick={on_ws_export}>{"Copy workspace to clipboard"}</button>
                                        </div>
                                        <div class="import-row">
                                            <textarea class="code-area import-area" placeholder="Paste workspace JSON to restore everything..." value={(*ws_import_text).clone()} oninput={on_ws_import_text} />
                                            <button class="btn btn-primary" onclick={on_ws_import}>{"Restore"}</button>
                                        </div>
                                        { if let Some(ref msg) = *ws_import_msg {
                                            html! { <span class="import-msg">{msg}</span> }
                                        } else { html!{} } }
                                    </div>
                                </section>
                            </div>
                        }} else { html!{} } }

                        // ═══ MODEL TAB ═══
                        { if panel.is_none() && tab == "model" { html! {
                            <div class="tab-panel">

                                // ── Instructions ──
                                <section class="section section-muted">
                                    <details class="guide-details">
                                        <summary class="guide-summary">{"How to use this tool"}</summary>
                                        <div class="guide-body">
                                            <p class="guide-text" style="margin-bottom: 10px; line-height: 1.6;">
                                                {"You can\u{2019}t fix what you can\u{2019}t see. This tool makes a system\u{2019}s real behavior visible \u{2014} its states, transitions, exceptions, and workarounds \u{2014} so the people who live with it every day can inspect it together and decide what to change."}
                                            </p>
                                            <p class="guide-text" style="margin-bottom: 10px; line-height: 1.6; font-size: 12px; opacity: 0.85;">
                                                {"Bring "}<strong>{"decision-makers"}</strong>{", "}
                                                <strong>{"people who can block"}</strong>{", "}
                                                <strong>{"people affected by the outcome"}</strong>{", and "}
                                                <strong>{"those with hands-on expertise"}</strong>
                                                {". No single person sees the whole system \u{2014} each sees a part, and the truth lives in the overlap."}
                                            </p>
                                            <p class="guide-text" style="margin-bottom: 2px; padding-bottom: 8px; border-bottom: 1px solid var(--border);">
                                                {"Use any LLM to generate a TLA+ state machine of your process, then walk through it as a group: "}
                                                <em>{"Is this what really happens? What\u{2019}s missing? What breaks?"}</em>
                                                {" Comments feed back into the next iteration until the model matches reality."}
                                            </p>
                                            <p class="guide-text">
                                                <strong>{"Generate \u{2014} "}</strong>
                                                {"Use any approved LLM to generate a TLA+ state machine of your process. The "}<strong>{"Prompts"}</strong>{" tab has starter templates to help \u{2014} or use your own. Paste the output into the editor and click "}<strong>{"Parse"}</strong>{"."}
                                            </p>
                                            <p class="guide-text">
                                                <strong>{"Simulate \u{2014} "}</strong>
                                                {"Click transitions to walk the state machine step by step. At each state, ask: does this match reality? What\u{2019}s missing?"}
                                            </p>
                                            <p class="guide-text">
                                                <strong>{"Comment \u{2014} "}</strong>
                                                {"Click any state to leave feedback. Engineers, PMs, ops, domain experts \u{2014} everyone\u{2019}s input gets embedded in the next iteration prompt."}
                                            </p>
                                            <p class="guide-text">
                                                <strong>{"Iterate \u{2014} "}</strong>
                                                {"Copy the "}<em>{"Iterate"}</em>{" prompt (spec + all comments bundled). The LLM classifies feedback, finds gaps, and outputs a revised spec. Paste it back and repeat."}
                                            </p>
                                            <p class="guide-text" style="margin-top: 4px; padding-top: 6px; border-top: 1px solid var(--border);">
                                                <strong>{"Versions"}</strong>{" \u{2014} Snapshots, import/export, full backup. "}
                                                <strong>{"Share"}</strong>{" \u{2014} URL with spec + comments. "}
                                                <strong>{"Reference"}</strong>{" \u{2014} Transition table. "}
                                                <strong>{"Diagram"}</strong>{" \u{2014} Mermaid state diagram."}
                                            </p>
                                            <p class="guide-text" style="font-size: 12px; opacity: 0.7; margin-top: 4px;">
                                                {"Built on "}
                                                <a href="https://lamport.azurewebsites.net/pubs/state-machine.pdf" target="_blank" rel="noopener">{"Lamport\u{2019}s Computation and State Machines"}</a>
                                                {". Every system is a state machine \u{2014} this tool helps you find the one hiding in yours."}
                                            </p>
                                        </div>
                                    </details>
                                </section>

                                // ── Simulate ──
                                <section class="section">
                                    <div class="section-bar">
                                        <h2 class="section-title">{"Simulate"}<span class="sim-state-inline">{&current_sim}</span></h2>
                                        <button class="btn btn-ghost" onclick={on_reset}>{"Reset"}</button>
                                    </div>
                                    { if available.is_empty() {
                                        html! { <p class="help-text">{"Terminal state \u{2014} no transitions. Reset to start over."}</p> }
                                    } else { html! {
                                        <div class="action-grid">
                                            { for available.into_iter().map(|action| {
                                                let an = action.name.clone();
                                                let tos = action.to.clone();
                                                let cmt = action.comment.clone();
                                                let ss = sim_state.clone();
                                                let ct = comment_target.clone();
                                                let onclick = {
                                                    let tos = tos.clone();
                                                    Callback::from(move |_| {
                                                        if let Some(t) = tos.first() {
                                                            ss.set(t.clone());
                                                            ct.set(Some(t.clone()));
                                                        }
                                                    })
                                                };
                                                let to_text = if tos.is_empty() { "?".into() } else { tos.join(", ") };
                                                html! {
                                                    <div class="action-card" onclick={onclick}>
                                                        <div class="action-name">{&an}</div>
                                                        <div class="action-target">{format!("\u{2192} {}", to_text)}</div>
                                                        { cmt.map(|c| html! { <div class="action-desc">{c}</div> }).unwrap_or_default() }
                                                    </div>
                                                }
                                            }) }
                                        </div>
                                    } } }
                                </section>

                                // ── States ──
                                <section class="section">
                                    <h2 class="section-title">{"States"}</h2>
                                    <div class="state-grid">
                                        { for parsed.states.iter().map(|state| {
                                            let is_current = *state == current_sim;
                                            let is_selected = (*comment_target).as_ref() == Some(state);
                                            let class = if is_current && is_selected { "state-chip current selected" }
                                                else if is_current { "state-chip current" }
                                                else if is_selected { "state-chip selected" }
                                                else { "state-chip" };
                                            let count = (*comments).iter().filter(|c| c.target == *state).count();
                                            let target = comment_target.clone();
                                            let sn = state.clone();
                                            let onclick = Callback::from(move |_| { target.set(Some(sn.clone())); });
                                            html! {
                                                <div class={class} onclick={onclick}>
                                                    <span class="state-label">{state}</span>
                                                    { if count > 0 { html! { <span class="chip-badge">{count.to_string()}</span> } } else { html!{} } }
                                                </div>
                                            }
                                        }) }
                                    </div>
                                </section>

                                // ── Comment panel ──
                                { if let Some(ref ts) = *comment_target {
                                    let sc: Vec<&UserComment> = (*comments).iter().filter(|c| c.target == *ts).collect();
                                    html! {
                                        <section class="section comment-panel">
                                            <div class="section-bar">
                                                <h2 class="section-title">{ts.clone()}</h2>
                                                <button class="btn btn-ghost" onclick={on_close_comment}>{"\u{2715}"}</button>
                                            </div>
                                            { if !sc.is_empty() { html! {
                                                <div class="comment-thread">
                                                    { for sc.into_iter().map(|c| html! {
                                                        <div class="comment-msg">
                                                            { if let Some(ref cat) = c.category { html! {
                                                                <span class="comment-category">{cat}</span>
                                                            }} else { html!{} } }
                                                            <span class="comment-who">{&c.author}</span>
                                                            <span class="comment-body">{&c.text}</span>
                                                        </div>
                                                    }) }
                                                </div>
                                            }} else { html!{} } }
                                            <div class="category-selector">
                                                <span class="category-label">{"Tag:"}</span>
                                                {
                                                    {
                                                        let cats: Vec<&str> = vec!["Missing step", "Not what happens", "Failure mode", "Workaround", "Naming", "Scope question"];
                                                        let comment_category = comment_category.clone();
                                                        html! { for cats.into_iter().map(move |cat| {
                                                            let cc = comment_category.clone();
                                                            let cat_str = cat.to_string();
                                                            let is_active = (*cc) == Some(cat_str.clone());
                                                            let onclick = {
                                                                let cc = cc.clone();
                                                                let cat_str = cat_str.clone();
                                                                Callback::from(move |_| {
                                                                    if *cc == Some(cat_str.clone()) {
                                                                        cc.set(None);
                                                                    } else {
                                                                        cc.set(Some(cat_str.clone()));
                                                                    }
                                                                })
                                                            };
                                                            html! {
                                                                <button class={classes!("category-tag", is_active.then_some("active"))} onclick={onclick}>{cat}</button>
                                                            }
                                                        })}
                                                    }
                                                }
                                            </div>
                                            <div class="comment-cta-inline">
                                                { category_cta(&*comment_category, &lib) }
                                            </div>
                                            <div class="comment-compose">
                                                <input class="input-sm" placeholder="Name" value={(*comment_author).clone()} oninput={on_author} />
                                                <textarea class="input-area" placeholder={category_placeholder(&*comment_category, &lib)} value={(*comment_draft).clone()} oninput={on_comment_draft} />
                                                <button class="btn btn-primary" onclick={on_submit_comment}>{"Add"}</button>
                                            </div>
                                        </section>
                                    }
                                } else { html!{} } }

                                // ── All feedback ──
                                { if !(*comments).is_empty() { html! {
                                    <section class="section">
                                        <h2 class="section-title">{format!("All feedback ({})", (*comments).len())}</h2>
                                        { for parsed.states.iter().filter_map(|state| {
                                            let sc: Vec<&UserComment> = (*comments).iter().filter(|c| c.target == *state).collect();
                                            if sc.is_empty() { None } else { Some(html! {
                                                <div class="feedback-group">
                                                    <div class="feedback-state">{state}</div>
                                                    { for sc.into_iter().map(|c| html! {
                                                        <div class="comment-msg">
                                                            { if let Some(ref cat) = c.category { html! {
                                                                <span class="comment-category">{cat}</span>
                                                            }} else { html!{} } }
                                                            <span class="comment-who">{&c.author}</span>
                                                            <span class="comment-body">{&c.text}</span>
                                                        </div>
                                                    }) }
                                                </div>
                                            })}
                                        }) }
                                    </section>
                                }} else { html!{} } }
                            </div>
                        }} else { html!{} } }

                        // ═══ REFERENCE TAB ═══
                        { if panel.is_none() && tab == "reference" { html! {
                            <div class="tab-panel">
                                { if !grouped.is_empty() { html! {
                                    <section class="section">
                                        <h2 class="section-title">{"Transition reference"}</h2>
                                        { for grouped.into_iter().map(|(state, actions)| html! {
                                            <details class="transition-group">
                                                <summary class="transition-summary">
                                                    <span class="tg-state">{&state}</span>
                                                    <span class="tg-count">{format!("{} transition{}", actions.len(), if actions.len() == 1 { "" } else { "s" })}</span>
                                                </summary>
                                                <div class="transition-list">
                                                    { for actions.into_iter().map(render_action) }
                                                </div>
                                            </details>
                                        }) }
                                    </section>
                                }} else { html! { <p class="help-text">{"Parse a TLA+ spec to see transitions."}</p> } } }
                            </div>
                        }} else { html!{} } }

                        // ═══ DIAGRAM TAB ═══
                        { if panel.is_none() && tab == "diagram" { html! {
                            <div class="tab-panel diagram-panel">
                                { if parsed.states.is_empty() {
                                    html! { <p class="help-text">{"Parse a TLA+ spec to see the state diagram."}</p> }
                                } else {
                                    html! { <div id="mermaid-target" class="mermaid-container"></div> }
                                } }
                            </div>
                        }} else { html!{} } }

                    </div>
                </div>
            </div>
        </div>
    }
}

// ─── Helpers ───

fn build_mermaid(parsed: &model::ParsedMachine) -> String {
    let mut out = String::from("stateDiagram-v2\n");
    if let Some(first) = parsed.states.first() {
        out.push_str(&format!("    [*] --> {}\n", sanitize_mermaid(first)));
    }
    for action in &parsed.actions {
        for from in &action.from {
            for to in &action.to {
                let label = &action.name;
                out.push_str(&format!("    {} --> {} : {}\n", sanitize_mermaid(from), sanitize_mermaid(to), label));
            }
        }
    }
    // Terminal states (no outgoing transitions)
    let from_set: std::collections::HashSet<&str> = parsed.actions.iter().flat_map(|a| a.from.iter().map(|s| s.as_str())).collect();
    for state in &parsed.states {
        if !from_set.contains(state.as_str()) {
            out.push_str(&format!("    {} --> [*]\n", sanitize_mermaid(state)));
        }
    }
    out
}

fn sanitize_mermaid(s: &str) -> String {
    s.replace(' ', "_").replace('-', "_").replace('/', "_")
}

fn show_toast(msg: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(body) = doc.body() {
                let tip = doc.create_element("span").unwrap();
                tip.set_class_name("copy-toast");
                tip.set_text_content(Some(msg));
                if let Some(active) = doc.active_element() {
                    let rect = active.get_bounding_client_rect();
                    let left = rect.left() + rect.width() / 2.0;
                    let top = rect.top() - 8.0;
                    let _ = tip.set_attribute("style", &format!("left:{:.0}px;top:{:.0}px", left, top));
                }
                let _ = body.append_child(&tip);
                let tip2 = tip.clone();
                let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let _ = tip2.remove();
                });
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(), 1400
                );
            }
        }
    }
}

fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        let _ = clipboard.write_text(text);
        let _ = window.alert_with_message(
            "\u{26A0}\u{FE0F} Copied to clipboard.\n\nThis contains your spec, comments, and system design details. Be mindful when sharing \u{2014} it may expose sensitive information about your system."
        );
    }
}

fn render_action(action: Action) -> Html {
    let class = if contains_any(&action.name, &["block", "error", "fail", "violate", "reject", "walk"]) {
        "t-card t-warn"
    } else if contains_any(&action.name, &["win", "collect", "close", "complete", "finish"]) {
        "t-card t-success"
    } else {
        "t-card"
    };
    let from_text = if action.from.is_empty() { "\u{27F5} (inferred)".into() } else { format!("\u{27F5} {}", action.from.join(", ")) };
    let to_text = if action.to.is_empty() { "\u{27F6} (derived)".into() } else { format!("\u{27F6} {}", action.to.join(", ")) };
    html! {
        <div class={class}>
            <div class="t-name">{action.name}</div>
            <div class="t-flow">
                <span class="t-from">{from_text}</span>
                <span class="t-to">{to_text}</span>
            </div>
            { action.comment.map(|c| html! { <div class="t-comment">{c}</div> }).unwrap_or_default() }
        </div>
    }
}

fn group_actions_by_from(actions: Vec<Action>) -> BTreeMap<String, Vec<Action>> {
    let mut map: BTreeMap<String, Vec<Action>> = BTreeMap::new();
    for action in actions {
        if action.from.is_empty() {
            map.entry("Cross-cutting".to_string()).or_default().push(action.clone());
        }
        for from in &action.from {
            map.entry(from.clone()).or_default().push(action.clone());
        }
    }
    map
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    let lower = haystack.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn hash_source(s: &str) -> u64 {
    let mut h: u64 = 5381;
    for byte in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(byte as u64);
    }
    h
}

fn main() {
    console_error_panic_hook::set_once();
    let root = window()
        .and_then(|win| win.document())
        .and_then(|document| document.get_element_by_id("app"));
    if let Some(root) = root {
        yew::Renderer::<App>::with_root(root).render();
    } else {
        yew::Renderer::<App>::new().render();
    }
}
