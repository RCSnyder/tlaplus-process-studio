mod model;
mod parser;
mod layout;

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{window, HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use layout::compute_state_diagram_layout;
use model::Action;
use parser::parse_tla;

// ─── Sample TLA+ ───

const SAMPLE_TLA: &str = r#"---- MODULE CollaborativeModeling ----

(*
    Meta-model: the collaborative workflow of building and iterating a
    business-process state machine in TLA+ Process Studio.
*)

VARIABLE processState

ProcessStates == {
    "NoSpec",
    "DraftingPrompt",
    "WaitingOnLLM",
    "SpecLoaded",
    "Simulating",
    "Commenting",
    "ReviewingComments",
    "Iterating",
    "ConsensusReached",
    "Archived"
}

(* The facilitator opens the tool for the first time. *)
StartFresh ==
    /\ processState = "NoSpec"
    /\ processState' = "DraftingPrompt"

(* Copy the bootstrap prompt and customize it for the domain. *)
DraftBootstrapPrompt ==
    /\ processState = "DraftingPrompt"
    /\ processState' = "WaitingOnLLM"

(* LLM returns a draft spec; paste it and parse. *)
ReceiveSpec ==
    /\ processState = "WaitingOnLLM"
    /\ processState' = "SpecLoaded"

(* Walk through the modeled process. *)
BeginSimulation ==
    /\ processState = "SpecLoaded"
    /\ processState' = "Simulating"

(* Move through transitions and inspect each state. *)
StepThroughTransition ==
    /\ processState = "Simulating"
    /\ processState' = "Commenting"

(* Capture stakeholder feedback on a state. *)
LeaveComment ==
    /\ processState = "Commenting"
    /\ processState' = "Simulating"

(* Review all collected comments and conflicts. *)
FinishReview ==
    /\ processState = "Simulating"
    /\ processState' = "ReviewingComments"

(* Decide to revise model based on evidence. *)
DecideToIterate ==
    /\ processState = "ReviewingComments"
    /\ processState' = "Iterating"

(* Copy state + comments or iterate prompt and send to LLM. *)
SendIterationToLLM ==
    /\ processState = "Iterating"
    /\ processState' = "WaitingOnLLM"

(* Team agrees model reflects real process behavior. *)
ReachConsensus ==
    /\ processState = "ReviewingComments"
    /\ processState' = "ConsensusReached"

(* Save and share the finalized model. *)
ArchiveModel ==
    /\ processState = "ConsensusReached"
    /\ processState' = "Archived"

(* Reopen an archived model for another cycle. *)
ReopenModel ==
    /\ processState = "Archived"
    /\ processState' = "SpecLoaded"

(* Load a shared URL or imported version directly. *)
LoadSharedSpec ==
    /\ processState = "NoSpec"
    /\ processState' = "SpecLoaded"

(* INVARIANT: state is always one of the known stages. *)
TypeCorrectness ==
    processState \in ProcessStates => TRUE

Init == processState = "NoSpec"

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

// ─── Example specs (embedded at compile time) ───

const EXAMPLE_SPECS: &[(&str, &str)] = &[
    ("Collaborative Modeling", SAMPLE_TLA),
    ("Innovation Adoption", include_str!("../fixtures/examples/innovation-adoption.tla")),
    ("Team Delivery", include_str!("../fixtures/examples/team-delivery.tla")),
    ("QRSPI Workflow", include_str!("../fixtures/examples/qrspi-workflow.tla")),
    ("MLOps Lifecycle", include_str!("../fixtures/examples/mlops-lifecycle.tla")),
    ("Meeting Lifecycle", include_str!("../fixtures/examples/meeting-lifecycle.tla")),
    ("Hiring Pipeline", include_str!("../fixtures/examples/hiring-pipeline.tla")),
];

// ─── Storage keys ───

const STORAGE_KEY: &str = "tla_studio_comments";
const STORAGE_SOURCE_KEY: &str = "tla_studio_source";
const STORAGE_SNAPSHOTS_KEY: &str = "tla_studio_snapshots";
const STORAGE_THEME_KEY: &str = "tla_studio_theme";

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

When you are ready to produce the spec, output ONE complete TLA+ module and NOTHING ELSE. Do not wrap it in markdown fences. Do not prepend analysis. Do not append a summary after `====`.

The `(* ... *)` comments on each state and transition ARE the plain-English narrative — stakeholders will read these directly in the visual tool. Make every comment tell a complete, readable story.

## Comment style

Write comments the way a thoughtful colleague talks — warm, direct, and grounded in what actually happens. Comments should respect the reader's intelligence and the expertise of the people doing the work.

**Do:**
- Describe what happens at each step and why it matters in plain language
- Acknowledge the skill, judgment, and experience involved — briefly, not reverently
- Vary sentence length and structure naturally
- Keep it concrete and specific to the process

**Don't:**
- Use preachy superlatives ("one of the most generous acts", "irreplaceable human gift")
- Write in groups of exactly three parallel examples
- Use "This is not just X — it is Y" constructions
- Add filler like "worth celebrating", "is a gift", "at its finest"
- Pad with hedge phrases ("It's important to note that...")
- Use corporate-speak ("leverage", "utilize", "facilitate")

The comments should read like they were written by someone who actually does this work, not by someone admiring it from a distance.

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

## Parser-safe syntax (CRITICAL)

This app uses a regex-based parser. The following constraints are REQUIRED for the visual tool to work. Violating any of them will produce a broken or empty diagram.

1. **Single VARIABLE only**: declare exactly ONE variable, and it MUST end in `State` (e.g. `orderState`, `hiringState`). Do NOT declare multiple variables.
2. **State set name**: the set of states MUST be named ending in `Stages` or `States` (e.g. `OrderStages`, `HiringStates`). Any other name (like `Phases` or `Statuses`) will NOT be detected.
3. **State names**: use PascalCase without spaces inside the quoted strings (e.g. `"AwaitingApproval"`, `"OrderShipped"`). No spaces, no snake_case, no abbreviations. These are displayed directly to stakeholders — PascalCase reads clearly.
4. **Transition body**: each action MUST be EXACTLY two conjuncts — no more, no less:
   ```
   ActionName ==
     /\ varState = "FromState"
     /\ varState' = "ToState"
   ```
   Do NOT add guards, UNCHANGED, additional conjuncts, or any other TLA+ operators to the body.
5. **Forbidden operators in transition bodies**: Do NOT use any of these inside action definitions:
   - `IF / THEN / ELSE`
   - `CASE`
   - `LET .. IN`
   - `\E` or `\A` (quantifiers)
   - `CHOOSE`
   - `UNCHANGED`
   - Sequences `<<...>>` or functions `[x \in S |-> ...]`
   If a transition has a condition, model it as two separate named transitions (one for each branch).
6. **Action names**: PascalCase, letters/digits/underscores only (must match regex `\w+`). No spaces, hyphens, or special characters.
7. **No helper operators**: every `Name ==` definition is parsed as an action. Do NOT define helper predicates, constants, or sub-expressions using `==`. The only `Name ==` definitions allowed are: the state set, individual actions, Init, and Next.
8. **Init**: must be exactly `Init == varState = "InitialState"` — no /\ prefix, no extra conjuncts.
9. **Next**: must be a flat disjunction of action names: `Next == \/ Action1 \/ Action2 \/ ...`
10. **Comments**: use multi-line `(* ... *)` immediately before each action definition (no blank lines between the closing `*)` and the action name). These become the narrative text stakeholders read in the tool.
11. **Module delimiters**: `---- MODULE Name ----` at the top and `====` at the bottom are required.

This simplified form is designed for stakeholder collaboration. When the model is later ported to a full TLA+ spec, guards, multiple variables, fairness conditions, and richer operators can be added — but for this visual tool, stick to the simple pattern above.

Requirements:
- State names: PascalCase noun phrases (`"AwaitingApproval"`, `"OrderShipped"`, not `"state_3"`)
- Transition names: PascalCase verb phrases (`"SubmitForReview"`, `"TimeoutToEscalation"`)
- Every state MUST be reachable — no orphans
- Every non-terminal state MUST have at least one outbound transition
- Add rich (* comments *) above EVERY transition — these are the primary way stakeholders read the model. Each comment should tell a complete story: what triggers the transition, who does it, what they need, what can go wrong, work type, and technology support rating
- Mark ADAPTATION transitions where the team's actual practice differs from documented process
- Mark EXTRA WORK transitions that exist to fix or clean up upstream issues
- Where perspectives differ, add (* PERSPECTIVES DIFFER: ... *) comments capturing both views
- Put failure/recovery transitions AFTER the happy path so the spec reads top-to-bottom as a narrative
- Include at least one invariant (even if it's just type-correctness)
- ---- MODULE and ==== delimiters are required

## Final self-check before you answer

Before you output the final module, silently verify ALL of these are true. If any fail, rewrite before answering.

- The response contains exactly one `---- MODULE ... ----` block and ends with `====`
- There is exactly one `VARIABLE`, and its name ends in `State`
- There is exactly one state set, and its name ends in `Stages` or `States`
- Every quoted state name is PascalCase with no spaces
- Every transition action body contains exactly two conjuncts: one current-state test and one next-state assignment
- `Init == varState = "InitialState"` appears exactly once
- `Next ==` is a flat disjunction of action names only
- No helper operators, no extra prose, no markdown fences, no text before the module, no text after `====`"#;

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

Generate a COMPLETE revised TLA+ module (not a diff). Output ONLY the module. Do not include analysis, bullet lists, markdown fences, or any prose before the module or after `====`.

The `(* ... *)` comments on each transition ARE the plain-English narrative — stakeholders read these directly in the visual tool. Make every comment tell a complete, readable story.

## Comment style

Write comments the way a thoughtful colleague talks — warm, direct, and grounded in what actually happens. Comments should respect the reader's intelligence and the expertise of the people doing the work.

- Describe what happens and why it matters, in plain language
- Acknowledge skill and judgment briefly, not reverently
- Vary sentence length and structure naturally
- Stay concrete and specific to the process
- Avoid preachy superlatives, groups of exactly three parallel examples, "This is not just X — it is Y", filler like "worth celebrating" or "is a gift", corporate-speak like "leverage" or "utilize"
- Write like someone who does this work, not someone admiring it

Also check abstraction level: if the model is growing beyond ~15 states, suggest breaking it into sub-processes rather than adding more states. Fewer states that everyone can see and argue about beats a comprehensive model no one can follow.

Follow the exact format:
- ---- MODULE Name ---- and ==== delimiters
- State set named ending in `Stages` or `States` (e.g. `OrderStages`) — other suffixes will NOT be detected by the parser
- EXACTLY one VARIABLE, ending in `State` (e.g. `orderState`) — must match the bootstrap output, do NOT rename it
- State names: PascalCase without spaces in quoted strings (e.g. `"AwaitingApproval"`, not `"Awaiting Approval"`)
- Each transition MUST be EXACTLY two conjuncts — no more, no less:
  ```
  ActionName ==
    /\ varState = "FromState"
    /\ varState' = "ToState"
  ```
- Do NOT use IF/THEN/ELSE, CASE, LET..IN, \E, \A, CHOOSE, UNCHANGED, or sequences in transition bodies. If a transition has a condition, model it as two separate named transitions.
- Do NOT define helper operators — every `Name ==` is treated as an action by the parser. Only the state set, actions, Init, and Next should use `==`.
- Action names: PascalCase, alphanumeric/underscore only (must match `\w+`)
- Init must be exactly: `Init == varState = "InitialState"`
- Next must be a flat disjunction: `Next == \/ Action1 \/ Action2 \/ ...`
- (* comments *) immediately before every transition (no blank lines between `*)` and the action name) — these are the primary way stakeholders read the model. Each comment should tell a complete story: what triggers it, who does it, work type, technology support rating
- Mark ADAPTATION transitions where the team's practice differs from documentation
- Mark EXTRA WORK transitions that exist to fix upstream issues
- (* PERSPECTIVES DIFFER: ... *) comments capturing multiple views
- (* REVIEW: ... *) comments on anything uncertain
- All invariants at the bottom
- Happy path transitions first, then branches, then failure/recovery paths

## Important
- Do NOT remove states or transitions that no comment complained about — preserve existing correct structure
- Do NOT resolve SCOPE QUESTION or DIFFERENT PERSPECTIVES comments yourself — note them for the humans to discuss
- If a comment is ambiguous, add the most likely interpretation AND flag it with a (* REVIEW: ... *) comment
- Preserve all existing (* comments *) and add new ones for new transitions
- When stakeholders see things differently, model ALL perspectives and flag with (* PERSPECTIVES DIFFER: ... *)
- NEVER introduce IF/THEN/ELSE, CASE, LET..IN, quantifiers, UNCHANGED, helper operators, or extra conjuncts — the parser will break. If a transition needs a branch, make it two separate named transitions.
- NEVER use spaces in state names — use PascalCase (e.g. `"AwaitingApproval"`)
- NEVER rename the state variable or the state set — keep the exact names from the previous version

## Final self-check before you answer

Before you output the revised module, silently verify ALL of these are true. If any fail, rewrite before answering.

- The response contains exactly one `---- MODULE ... ----` block and ends with `====`
- The existing state variable name is preserved exactly
- The existing state set name is preserved exactly
- Every transition body contains exactly two conjuncts
- `Init` appears exactly once and still matches the state variable
- `Next` is a flat disjunction of action names only
- No prose appears before the module or after `====`
- No markdown fences or explanatory text are present

Here is the current state machine and collected feedback:

"#;

const AGENT_QUERY_INSTRUCTIONS: &str = r#"# TLA+ Process Studio — Agent Interface v1

This app is 100% client-side. No server, no API, no auth.
All persistent state lives in localStorage. Nothing is ever transmitted over the network.
You are the LLM. The user brings you here. Your job is to iterate the model.

─────────────────────────────────────────
## STABLE SELECTORS (use these, not CSS classes)
─────────────────────────────────────────

  data-field="spec"           The TLA+ source editor (textarea)
  data-action="parse"         Parse button — triggers re-parse + diagram update
  data-action="save-snapshot" Save current spec+comments as a named version
  data-parser-status          On the app root — values: "ok" | "warnings" | "no-spec"
  data-module                 On the app root — current parsed module name
  data-state-count            On the app root — number of parsed states (string)

─────────────────────────────────────────
## READ API
─────────────────────────────────────────

// Full TLA+ specification (persisted, survives reload)
localStorage.getItem("tla_studio_source")

// All stakeholder comments
JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
// Shape: [{target: "StateName", author: "Name", text: "...", category?: "...", chain?: [...]}]

// Named snapshots (version history)
JSON.parse(localStorage.getItem("tla_studio_snapshots") || "[]")
// Shape: [{name: "...", source: "...", comments: [...], timestamp: ms, hash: n}]

// Editor live value (may differ from localStorage if not yet parsed)
document.querySelector("[data-field='spec']").value

// Parser status of the current loaded spec
document.querySelector("[data-parser-status]").dataset.parserStatus
// → "ok" | "warnings" | "no-spec"

─────────────────────────────────────────
## WRITE API
─────────────────────────────────────────

// Write a new or revised TLA+ spec into the editor
// (MUST use the native setter pattern — direct .value = does not fire Yew events)
const ta = document.querySelector("[data-field='spec']");
const setter = Object.getOwnPropertyDescriptor(
  window.HTMLTextAreaElement.prototype, 'value'
).set;
setter.call(ta, newSpecString);
ta.dispatchEvent(new Event('input', { bubbles: true }));

// Trigger parse + diagram update
document.querySelector("[data-action='parse']").click();

// Save current state as a named snapshot
document.querySelector("[data-action='save-snapshot']").click();
// (Snapshot name comes from the header input; leave it to default to the module name)

─────────────────────────────────────────
## RECOMMENDED AGENT LOOP
─────────────────────────────────────────

  STEP 1 — Navigate to the page URL
            If the URL contains a fragment (#share=...) it has preloaded spec+comments — let it settle before reading
  STEP 2 — Read spec:     localStorage.getItem("tla_studio_source")
  STEP 3 — Read comments: JSON.parse(localStorage.getItem("tla_studio_comments") || "[]")
            Group by `target` (state name) to understand which states have feedback
  STEP 4 — Analyze. Generate a revised TLA+ spec that addresses the comments.
            Follow the parser rules below exactly or the diagram will not render.
            Write ONLY the TLA+ module. No markdown fences. No explanatory prose.
  STEP 5 — Write revised spec using the native setter pattern (see WRITE API above)
  STEP 6 — Click parse:   document.querySelector("[data-action='parse']").click()
            Wait for the DOM to settle (100–300ms)
  STEP 7 — Check status:  document.querySelector("[data-parser-status]").dataset.parserStatus
            If "warnings": read the warnings banner text, fix the spec, repeat from STEP 5.
            If "ok": proceed to STEP 8.
            If "no-spec": the write failed — try the setter pattern again.
  STEP 8 — Save (optional): document.querySelector("[data-action='save-snapshot']").click()
  STEP 9 — Report to user. Repeat from STEP 2 if further iteration is requested.

─────────────────────────────────────────
## PARSER RULES (must follow or diagram breaks)
─────────────────────────────────────────

  - Exactly ONE variable ending in `State` (e.g. processState)
  - Exactly ONE state set ending in `States` or `Stages`
  - State names: quoted PascalCase strings, no spaces (e.g. "InReview")
  - Every transition must be exactly two conjuncts:
        ActionName ==
            /\ varState = "FromState"
            /\ varState' = "ToState"
  - Must include: Init == varState = "InitialState"
  - Must include: Next == \/ Action1 \/ Action2 ...
  - Invariants must use `=>` (e.g. varState \in AllStates => TRUE)
    - No IF/THEN/ELSE, CASE, LET/IN, UNCHANGED, or quantifiers in transition bodies
    - No helper operators using `==`
    - No text before the module and no text after `====`"#;

const BASIC_SYNTAX_PROMPT: &str = r#"You are converting a freeform business process description into parser-safe TLA+ for TLA+ Process Studio.

The user can describe the process however they want (messy notes, bullets, narrative, partial steps). You must infer states and transitions and output valid TLA+ in the exact format below.

## Strict output rules (required)
1. Output ONLY one complete TLA+ module, no extra prose.
2. Use exactly ONE variable and it must end with `State`.
3. Use exactly ONE state set named ending with `Stages` or `States`.
4. State names must be quoted PascalCase strings (no spaces).
5. Every transition must be exactly:
     ActionName ==
         /\ varState = "FromState"
         /\ varState' = "ToState"
6. Allowed source condition forms:
     - varState = "FromState"
     - varState \in {"A", "B", ...}
7. Do NOT use IF/THEN/ELSE, CASE, LET/IN, quantifiers, UNCHANGED, helper operators, or extra conjuncts in transition bodies.
8. Include both:
     - Init == varState = "InitialState"
     - Next == \/ Action1 \/ Action2 \/ ...
9. Keep `(* ... *)` narrative comments immediately above each transition.
10. End with `====`.

## Final self-check before you answer
- One module only
- No markdown fences
- No explanation before the module
- No summary after `====`
- Exactly one VARIABLE ending in `State`
- Exactly one state set ending in `Stages` or `States`
- Every action body has exactly two conjuncts
- Init appears once
- Next is a flat disjunction of action names only

## If user input is ambiguous
- Choose the most plausible flow.
- Add a `(* REVIEW: ... *)` comment above affected transitions.
- Prefer fewer states (5-12) and a clear happy path first.

## Output template
---- MODULE ProcessName ----

VARIABLE processState

ProcessStages == {
    "StateA",
    "StateB"
}

(* Narrative comment for stakeholders. *)
ActionName ==
    /\ processState = "StateA"
    /\ processState' = "StateB"

Init == processState = "StateA"

Next ==
    \/ ActionName

====

Now convert the user's freeform description into a complete module that follows every rule above."#;

// ─── Data types ───

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct UserComment {
    target: String,
    author: String,
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chain: Option<Vec<String>>,
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

fn load_theme() -> Option<String> {
    if let Some(storage) = get_storage() {
        if let Ok(Some(val)) = storage.get_item(STORAGE_THEME_KEY) {
            if val == "light" || val == "dark" {
                return Some(val);
            }
        }
    }
    None
}

fn save_theme(theme: Option<&str>) {
    if let Some(storage) = get_storage() {
        match theme {
            Some(val) => {
                let _ = storage.set_item(STORAGE_THEME_KEY, val);
            }
            None => {
                let _ = storage.remove_item(STORAGE_THEME_KEY);
            }
        }
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

fn build_state_comments_bundle(source: &str, comments: &[UserComment], states: &[String]) -> String {
    let mut out = String::new();
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
        out.push_str("\n(No comments collected yet)\n");
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

    let sim_chain = use_state(|| vec![parsed.start_state()]);
    let comments = use_state(|| {
        if let Some(ref p) = *shared { p.comments.clone() }
        else { load_comments() }
    });
    let comment_target = use_state(|| Some(parsed.start_state()));
    let comment_draft = use_state(String::new);
    let comment_author = use_state(|| "Anonymous".to_string());
    let editing_comment_idx = use_state(|| Option::<usize>::None);
    let editing_comment_text = use_state(String::new);
    let active_tab = use_state(|| "model".to_string());
    let active_panel = use_state(|| Option::<String>::None);
    let snapshots = use_state(load_snapshots);
    let snap_name = use_state(String::new);
    let import_text = use_state(String::new);
    let import_msg = use_state(|| Option::<String>::None);
    let ws_import_text = use_state(String::new);
    let ws_import_msg = use_state(|| Option::<String>::None);
    let theme = use_state(load_theme);

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

    // ── Load example spec ──
    let on_example = {
        let source = source.clone();
        let last_source_hash = last_source_hash.clone();
        let comments = comments.clone();
        let snapshots = snapshots.clone();
        let sim_chain = sim_chain.clone();
        let comment_target = comment_target.clone();
        Callback::from(move |e: Event| {
            let select: HtmlInputElement = e.target_unchecked_into();
            let idx_str = select.value();
            // Reset the select back to the placeholder
            select.set_value("");
            if let Ok(idx) = idx_str.parse::<usize>() {
                if let Some(&(_, spec)) = EXAMPLE_SPECS.get(idx) {
                    // Auto-save current state before loading example
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
                        let auto_count = snaps.iter().filter(|s| s.name.starts_with("Auto-save ")).count();
                        if auto_count > 20 {
                            if let Some(pos) = snaps.iter().position(|s| s.name.starts_with("Auto-save ")) {
                                snaps.remove(pos);
                            }
                        }
                        save_snapshots(&snaps);
                        snapshots.set(snaps);
                    }
                    let new_src = spec.to_string();
                    save_source(&new_src);
                    let new_hash = hash_source(&new_src);
                    last_source_hash.set(new_hash);
                    comments.set(Vec::new());
                    clear_stored_comments();
                    let machine = parse_tla(&new_src);
                    sim_chain.set(vec![machine.start_state()]);
                    comment_target.set(None);
                    source.set(new_src);
                }
            }
        })
    };

    // ── Parse ──
    let on_parse = {
        let source = source.clone();
        let last_source_hash = last_source_hash.clone();
        let comments = comments.clone();
        let snapshots = snapshots.clone();
        let sim_chain = sim_chain.clone();
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
            sim_chain.set(vec![machine.start_state()]);
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
        let sim_chain_for_comment = sim_chain.clone();
        Callback::from(move |_| {
            let text = (**comment_draft).trim().to_string();
            if text.is_empty() { return; }
            if let Some(ref target) = *comment_target {
                let mut next = (*comments).clone();
                next.push(UserComment {
                    target: target.clone(),
                    author: (*comment_author).clone(),
                    text,
                    category: None,
                    chain: Some((*sim_chain_for_comment).clone()),
                });
                save_comments(&next);
                comments.set(next);
                comment_draft.set(String::new());
            }
        })
    };
    // ── Comment edit / delete ──
    let on_edit_comment = {
        let editing_comment_idx = editing_comment_idx.clone();
        let editing_comment_text = editing_comment_text.clone();
        let comments = comments.clone();
        Callback::from(move |idx: usize| {
            if let Some(c) = (*comments).get(idx) {
                editing_comment_text.set(c.text.clone());
                editing_comment_idx.set(Some(idx));
            }
        })
    };
    let on_edit_comment_input = {
        let editing_comment_text = editing_comment_text.clone();
        Callback::from(move |e: InputEvent| {
            let ta: HtmlTextAreaElement = e.target_unchecked_into();
            editing_comment_text.set(ta.value());
        })
    };
    let on_save_edit = {
        let comments = comments.clone();
        let editing_comment_idx = editing_comment_idx.clone();
        let editing_comment_text = editing_comment_text.clone();
        Callback::from(move |_| {
            if let Some(idx) = *editing_comment_idx {
                let text = (*editing_comment_text).trim().to_string();
                if !text.is_empty() {
                    let mut next = (*comments).clone();
                    if let Some(c) = next.get_mut(idx) {
                        c.text = text;
                    }
                    save_comments(&next);
                    comments.set(next);
                }
                editing_comment_idx.set(None);
            }
        })
    };
    let on_cancel_edit = {
        let editing_comment_idx = editing_comment_idx.clone();
        Callback::from(move |_| {
            editing_comment_idx.set(None);
        })
    };
    let on_delete_comment = {
        let comments = comments.clone();
        Callback::from(move |idx: usize| {
            let confirmed = web_sys::window()
                .and_then(|w| w.confirm_with_message("Delete this comment?").ok())
                .unwrap_or(false);
            if !confirmed { return; }
            let mut next = (*comments).clone();
            if idx < next.len() {
                next.remove(idx);
                save_comments(&next);
                comments.set(next);
            }
        })
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
        let sim_chain = sim_chain.clone();
        let parsed = parsed.clone();
        let ct = comment_target.clone();
        Callback::from(move |_| {
            let start = parsed.start_state();
            ct.set(Some(start.clone()));
            sim_chain.set(vec![start]);
        })
    };
    let on_back = {
        let sim_chain = sim_chain.clone();
        let ct = comment_target.clone();
        Callback::from(move |_| {
            let mut chain = (*sim_chain).clone();
            if chain.len() > 1 {
                chain.pop();
                if let Some(last) = chain.last() {
                    ct.set(Some(last.clone()));
                }
                sim_chain.set(chain);
            }
        })
    };

    // ── Tabs (clicking a tab closes any open panel) ──
    let on_tab_model = { let t = active_tab.clone(); let p = active_panel.clone(); Callback::from(move |_| { t.set("model".to_string()); p.set(None); }) };
    let on_tab_prompts = { let t = active_tab.clone(); let p = active_panel.clone(); Callback::from(move |_| { t.set("prompts".to_string()); p.set(None); }) };
    let on_tab_versions = { let t = active_tab.clone(); let p = active_panel.clone(); Callback::from(move |_| { t.set("versions".to_string()); p.set(None); }) };

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

    // ── Quick copy: source + comments (for LLM iteration) ──
    let on_copy_state_comments = {
        let source = source.clone();
        let comments = comments.clone();
        let states = parsed.states.clone();
        Callback::from(move |_| {
            let text = build_state_comments_bundle(&source, &comments, &states);
            copy_to_clipboard(&text);
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
        let default_module_name = parsed.module_name.clone();
        Callback::from(move |_| {
            let ts = format_ts(now_ms());
            let base_name = if (*snap_name).trim().is_empty() {
                default_module_name.trim()
            } else {
                (*snap_name).trim()
            };
            let name = if base_name.is_empty() {
                format!("Snapshot {}", ts)
            } else {
                format!("{} ({})", base_name, ts)
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
            show_toast("Saved! Check Versions tab");
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
        let sim_chain = sim_chain.clone();
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
                    sim_chain.set(vec![machine.start_state()]);
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
    let active_bootstrap = BOOTSTRAP_PROMPT;
    let active_iteration = ITERATION_PROMPT;
    let snap_name_value = if (*snap_name).trim().is_empty() {
        parsed.module_name.clone()
    } else {
        (*snap_name).clone()
    };
    let export_text = build_export(&source, &comments, &parsed.states, active_iteration);
    let agent_meta_text = build_agent_meta(&source, &comments, &parsed.states, *last_source_hash);
    let _file_artifact = build_file_artifact("current", &source, &comments, *last_source_hash);
    let current_sim = sim_chain.last().cloned().unwrap_or_else(|| parsed.start_state());
    let available: Vec<&Action> = parsed.actions.iter().filter(|a| a.from.contains(&current_sim)).collect();
    let available_targets: std::collections::HashSet<String> = available.iter()
        .flat_map(|a| a.to.iter().cloned()).collect();
    let visited_set: std::collections::HashSet<String> = (*sim_chain).iter().cloned().collect();
    // Show the TLA+ comment for the transition that brought us to this node.
    // For the start state (chain length 1), show the comment of the first
    // outgoing action so the initial node still has context.
    let current_node_comment: Option<&str> = if sim_chain.len() >= 2 {
        let prev = &sim_chain[sim_chain.len() - 2];
        parsed.actions.iter()
            .find(|a| a.from.contains(prev) && a.to.contains(&current_sim))
            .and_then(|a| a.comment.as_deref())
    } else {
        // Start state — show the first outgoing action's comment as intro context
        parsed.actions.iter()
            .find(|a| a.from.contains(&current_sim))
            .and_then(|a| a.comment.as_deref())
    };
    let storage_count = (*comments).len();
    let tab = (*active_tab).clone();
    let panel = (*active_panel).clone();
    let can_submit_comment = !(**comment_draft).trim().is_empty();
    let compose_placeholder = "Comment for Future LLM Audit. Include: missing step, mismatch with reality, failure mode, workaround, naming issue, or scope question.".to_string();

    // Callback for diagram node clicks — advances chain (only available targets are clickable)
    let on_diagram_click = {
        let sc = sim_chain.clone();
        let ct = comment_target.clone();
        Callback::from(move |name: String| {
            let mut chain = (*sc).clone();
            chain.push(name.clone());
            sc.set(chain);
            ct.set(Some(name));
        })
    };

    let on_toggle_theme = {
        let theme = theme.clone();
        Callback::from(move |_| {
            let next = match theme.as_deref() {
                None => Some("light"),
                Some("light") => Some("dark"),
                Some("dark") => None,
                _ => None,
            };
            save_theme(next);
            theme.set(next.map(|s| s.to_string()));
        })
    };

    // Trigger diagram zoom init when Model tab is active
    {
        let tab2 = tab.clone();
        let panel2 = panel.clone();
        let parsed_states_len = parsed.states.len();
        use_effect_with((tab2, panel2, parsed_states_len), move |(tab, panel, len)| {
            if tab == "model" && panel.is_none() && *len > 0 {
                // Small delay to let the DOM render the SVG first
                let window = web_sys::window().unwrap();
                let closure = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let window = web_sys::window().unwrap();
                    let init_fn = js_sys::Reflect::get(&window, &JsValue::from_str("initDiagramZoom")).ok();
                    if let Some(func) = init_fn {
                        if let Ok(f) = func.dyn_into::<js_sys::Function>() {
                            let _ = f.call0(&JsValue::NULL);
                        }
                    }
                });
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.unchecked_ref(),
                    50,
                );
            }
            || ()
        });
    }

    // ─── RENDER ───
    let parser_status = if parsed.states.is_empty() {
        "no-spec"
    } else if parsed.warnings.is_empty() {
        "ok"
    } else {
        "warnings"
    };
    html! {
        <div class="app-shell"
            data-theme={(*theme).clone()}
            data-parser-status={parser_status}
            data-module={parsed.module_name.clone()}
            data-state-count={parsed.states.len().to_string()}>

            // ── Topbar ──
            <header class="topbar">
                <div class="topbar-left">
                    <h1 class="app-title">{"TLA+ Process Studio"}</h1>
                </div>
                <div class="topbar-right">
                    <span class="privacy-badge">{"100% client-side \u{00B7} nothing leaves your browser"}</span>
                    <button class="tbtn theme-toggle" onclick={on_toggle_theme}>
                        {
                            match theme.as_deref() {
                                None => "Theme: System",
                                Some("light") => "Theme: Light",
                                Some("dark") => "Theme: Dark",
                                _ => "Theme: System",
                            }
                        }
                    </button>
                </div>
            </header>

            // ── Workspace ──
            <div class="workspace">

                // ── Left pane: editor ──
                <div class="pane pane-left">
                    <div class="pane-header">
                        <span class="pane-label">{"Source"}</span>
                        <div class="toolbar">
                            <select class="example-select" onchange={on_example}>
                                <option value="" selected=true disabled=true>{"Load example\u{2026}"}</option>
                                { for EXAMPLE_SPECS.iter().enumerate().map(|(i, (name, _))| html! {
                                    <option value={i.to_string()}>{*name}</option>
                                }) }
                            </select>
                            <button class="btn btn-primary" data-action="parse" onclick={on_parse}>{"Parse"}</button>
                            { if storage_count > 0 {
                                html! { <button class="btn btn-danger" onclick={on_clear_comments}>{"Clear comments"}</button> }
                            } else { html!{} } }
                        </div>
                    </div>
                    <div class="pane-body">
                        <textarea class="code-area editor-area" data-field="spec" value={(*source).clone()} oninput={on_source} spellcheck="false" />
                    </div>
                </div>

                // ── Right pane ──
                <div class="pane pane-right">
                    <div class="tab-bar" role="tablist">
                        <button role="tab" aria-selected={if tab == "model" && panel.is_none() { "true" } else { "false" }} class={if tab == "model" && panel.is_none() { "tab tab-active" } else { "tab" }} onclick={on_tab_model}>{"Model"}</button>
                        <button role="tab" aria-selected={if tab == "prompts" && panel.is_none() { "true" } else { "false" }} class={if tab == "prompts" && panel.is_none() { "tab tab-active" } else { "tab" }} onclick={on_tab_prompts}>{"Prompts"}</button>
                        <button role="tab" aria-selected={if tab == "versions" && panel.is_none() { "true" } else { "false" }} class={if tab == "versions" && panel.is_none() { "tab tab-active" } else { "tab" }} onclick={on_tab_versions}>{format!("Versions{}", if (*snapshots).is_empty() { String::new() } else { format!(" ({})", (*snapshots).len()) })}</button>
                        <div class="tab-spacer" />
                        <input class="input-sm header-snap-name" placeholder={parsed.module_name.clone()} value={snap_name_value.clone()} oninput={on_snap_name.clone()} />
                        <div class="header-save-wrap">
                            <button class="tbtn" data-action="save-snapshot" onclick={on_save_snapshot.clone()}>{"\u{1F4BE} Save"}</button>
                        </div>
                        <button class="tbtn" onclick={on_copy_state_comments}>{"\u{1F4DD} Copy state + comments"}</button>
                        <button class="tbtn" onclick={on_share}>{"\u{1F517} Share"}</button>
                    </div>
                    <div class="tab-content" role="tabpanel">

                        // ═══ PANEL: Prompts (New spec + Iterate + Agent) ═══
                        { if panel.is_none() && tab == "prompts" {
                            let bootstrap_text = active_bootstrap.to_string();
                            let bootstrap_copy = active_bootstrap.to_string();
                            let syntax_text = BASIC_SYNTAX_PROMPT.to_string();
                            let syntax_copy = BASIC_SYNTAX_PROMPT.to_string();
                            let boot_desc = "Interviews you about actors, flows, failures & safety rules, then generates a TLA+ spec.";
                            let syntax_desc = "Give this to an LLM when users freeform describe a process and you need parser-safe TLA+ output.";
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
                                            <h3 class="prompt-col-title">{"Basic syntax"}</h3>
                                            <div class="copy-wrap">
                                                <button class="btn btn-primary" onclick={Callback::from(move |_| copy_to_clipboard(&syntax_copy))}>{"Copy"}</button>
                                            </div>
                                        </div>
                                        <span class="prompt-col-desc">{syntax_desc}</span>
                                        <textarea class="code-area panel-textarea" readonly=true value={syntax_text} />
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
                                        <span class="prompt-col-desc">{"Current spec + comments bundled with the full agent interface docs. Give to any agentic tool (MCP Playwright, browser-use, etc.) to automate iteration without leaving the app."}</span>
                                        <textarea class="code-area panel-textarea" readonly=true value={agent_meta_text} />
                                    </section>
                                </div>
                            </div>
                        }} else { html!{} } }

                        // ═══ PANEL: Versions ═══
                        { if panel.is_none() && tab == "versions" {
                            let kb = storage_size_kb();
                            let size_class = if kb > 4096.0 { "size-pill size-warn" } else { "size-pill" };
                            let size_label = if kb >= 1024.0 { format!("{:.1} MB stored", kb / 1024.0) } else { format!("{:.0} KB stored", kb) };
                            html! {
                            <div class="tab-panel panel-fill">
                                <section class="section panel-stretch">
                                    <div class="section-bar">
                                        <h2 class="section-title" style="margin:0">{"Versions"}<span class={size_class}>{size_label}</span></h2>
                                        <div class="toolbar">
                                            <input class="input-sm snap-name-input" placeholder="Name (optional)" value={snap_name_value.clone()} oninput={on_snap_name} />
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
                                                let sim_chain = sim_chain.clone();
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
                                                    sim_chain.set(vec![machine.start_state()]);
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
                            <div class="tab-panel model-panel">

                                // ── Parser warnings banner ──
                                { if !parsed.warnings.is_empty() {
                                    let source_text = (*source).clone();
                                    let warnings_text = parsed.warnings.join("\n");
                                    let repair_prompt = format!(
                                        "Fix this TLA+ spec so it parses correctly in TLA+ Process Studio.\n\nParser issues:\n{}\n\nCurrent source:\n```tla\n{}\n```",
                                        warnings_text,
                                        source_text,
                                    );
                                    let on_copy_warnings = Callback::from(move |_: MouseEvent| {
                                        copy_to_clipboard(&repair_prompt);
                                    });
                                    html! {
                                    <div class="parser-warnings">
                                        <div class="parser-warnings-header">
                                            <span class="parser-warnings-icon">{"\u{26A0}"}</span>
                                            <strong>{"Parser issues"}</strong>
                                            <span class="parser-warnings-hint">{" \u{2014} paste these back into your LLM to fix"}</span>
                                            <button class="parser-warnings-copy" onclick={on_copy_warnings}>{"\u{1F4CB} Copy source + issues"}</button>
                                        </div>
                                        <ul class="parser-warnings-list">
                                            { for parsed.warnings.iter().map(|w| html! {
                                                <li class="parser-warning-item">{w}</li>
                                            }) }
                                        </ul>
                                    </div>
                                }} else { html!{} } }

                                // ── Simulate column (col 1) ──
                                <section class="section section-simulate">
                                    <div class="section-bar">
                                        <h2 class="section-title">{"Simulate"}<span class="sim-state-inline">{&current_sim}</span></h2>
                                        <div class="simulate-controls">
                                            { if sim_chain.len() > 1 { html! {
                                                <button class="btn btn-ghost" onclick={on_back.clone()}> {"\u{2190} Back"} </button>
                                            }} else { html!{} } }
                                            <button class="btn btn-ghost" onclick={on_reset.clone()}>{"\u{21BA} Reset"}</button>
                                        </div>
                                    </div>
                                    // Chain breadcrumb
                                    { if sim_chain.len() > 1 { html! {
                                        <div class="chain-breadcrumb">
                                            { for sim_chain.iter().enumerate().map(|(i, s)| {
                                                let is_last = i == sim_chain.len() - 1;
                                                html! {
                                                    <span class="chain-item">
                                                        { if i > 0 { html! { <span class="chain-arrow">{"\u{2192}"}</span> } } else { html!{} } }
                                                        <span class={if is_last { "chain-step chain-step-current" } else { "chain-step" }}>{s}</span>
                                                    </span>
                                                }
                                            }) }
                                        </div>
                                    }} else { html!{} } }
                                    { if available.is_empty() {
                                        html! { <p class="help-text">{"Terminal state \u{2014} no transitions. Reset to start over."}</p> }
                                    } else { html! {
                                        <div class="action-grid">
                                            { for available.into_iter().map(|action| {
                                                let an = action.name.clone();
                                                let tos = action.to.clone();
                                                let cmt = action.comment.clone();
                                                let sc = sim_chain.clone();
                                                let ct = comment_target.clone();
                                                let onclick = {
                                                    let tos = tos.clone();
                                                    let sc = sc.clone();
                                                    let ct = ct.clone();
                                                    Callback::from(move |_: MouseEvent| {
                                                        if let Some(t) = tos.first() {
                                                            let mut chain = (*sc).clone();
                                                            chain.push(t.clone());
                                                            sc.set(chain);
                                                            ct.set(Some(t.clone()));
                                                        }
                                                    })
                                                };
                                                let to_text = if tos.is_empty() { "?".into() } else { tos.join(", ") };
                                                let onkeydown = {
                                                    let tos = tos.clone();
                                                    let sc = sc.clone();
                                                    let ct = ct.clone();
                                                    Callback::from(move |e: KeyboardEvent| {
                                                        if e.key() == "Enter" || e.key() == " " {
                                                            e.prevent_default();
                                                            if let Some(t) = tos.first() {
                                                                let mut chain = (*sc).clone();
                                                                chain.push(t.clone());
                                                                sc.set(chain);
                                                                ct.set(Some(t.clone()));
                                                            }
                                                        }
                                                    })
                                                };
                                                html! {
                                                    <div class="action-card" role="button" tabindex="0" onclick={onclick} onkeydown={onkeydown}>
                                                        <div class="action-name">{&an}</div>
                                                        <div class="action-target">{format!("\u{2192} {}", to_text)}</div>
                                                        { cmt.map(|c| html! { <div class="action-desc">{c}</div> }).unwrap_or_default() }
                                                    </div>
                                                }
                                            }) }
                                        </div>
                                    } } }
                                </section>

                                // ── Middle column: Comments + Invariants stacked ──
                                <div class="model-mid-column">
                                <section class="section section-comments">
                                    { if let Some(ref ts) = *comment_target {
                                        html! { <>
                                            <div class="section-bar">
                                                <h2 class="section-title">{"Feedback"}<span class="sim-state-inline">{ts.clone()}</span></h2>
                                            </div>
                                            { if let Some(desc) = current_node_comment { html! {
                                                <div class="state-description">{desc}</div>
                                            }} else { html!{} } }
                                            <div class="compose-card">
                                                <textarea class="compose-textarea" placeholder={compose_placeholder.clone()} value={(*comment_draft).clone()} oninput={on_comment_draft} />
                                                <div class="compose-submit">
                                                    <input class="compose-name" placeholder="Your name" value={(*comment_author).clone()} oninput={on_author} />
                                                    <button class={classes!("btn", "btn-primary")} disabled={!can_submit_comment} onclick={on_submit_comment}>{"Add feedback"}</button>
                                                </div>
                                            </div>
                                        </> }
                                    } else {
                                        html! {
                                            <div class="comment-cta">
                                                <span class="help-text" style="margin:0">{"Select a state to review."}</span>
                                            </div>
                                        }
                                    } }
                                </section>

                                // ── All Feedback ──
                                <section class="section section-all-feedback">
                                    <div class="section-bar">
                                        <h2 class="section-title">{format!("All Feedback ({})", comments.len())}</h2>
                                    </div>
                                    { if comments.is_empty() { html! {
                                        <p class="help-text" style="font-size:12.5px">{"Comments from all states will appear here as reviewers add feedback."}</p>
                                    }} else { html! {
                                        <div class="all-feedback-list">
                                            { for comments.iter().enumerate().map(|(idx, c)| {
                                                let is_editing = *editing_comment_idx == Some(idx);
                                                let on_edit = {
                                                    let cb = on_edit_comment.clone();
                                                    Callback::from(move |_: MouseEvent| cb.emit(idx))
                                                };
                                                let on_del = {
                                                    let cb = on_delete_comment.clone();
                                                    Callback::from(move |_: MouseEvent| cb.emit(idx))
                                                };
                                                if is_editing {
                                                    html! {
                                                        <div class="feedback-item comment-editing">
                                                            <textarea class="comment-edit-textarea" value={(*editing_comment_text).clone()} oninput={on_edit_comment_input.clone()} />
                                                            <div class="comment-edit-actions">
                                                                <button class="btn btn-primary btn-sm" onclick={on_save_edit.clone()}>{"Save"}</button>
                                                                <button class="btn btn-ghost btn-sm" onclick={on_cancel_edit.clone()}>{"Cancel"}</button>
                                                            </div>
                                                        </div>
                                                    }
                                                } else {
                                                    html! {
                                                        <div class="feedback-item">
                                                            <div class="feedback-header">
                                                                <span class="feedback-state">{&c.target}</span>
                                                                <span class="feedback-author">{&c.author}</span>
                                                                { if let Some(ref cat) = c.category { html! {
                                                                    <span class="comment-category">{cat}</span>
                                                                }} else { html!{} } }
                                                                { if let Some(ref chain) = c.chain { if chain.len() > 1 { html! {
                                                                    <span class="comment-chain-tag">{chain.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" \u{2192} ")}</span>
                                                                }} else { html!{} } } else { html!{} } }
                                                                <span class="comment-actions">
                                                                    <button class="comment-action-btn" onclick={on_edit} title="Edit">{"\u{270E}"}</button>
                                                                    <button class="comment-action-btn comment-action-delete" onclick={on_del} title="Delete">{"\u{2715}"}</button>
                                                                </span>
                                                            </div>
                                                            <span class="feedback-text">{&c.text}</span>
                                                        </div>
                                                    }
                                                }
                                            }) }
                                        </div>
                                    } } }
                                </section>
                                </div> // model-mid-column

                                // ── State Diagram (col 3) ──
                                <section class="section section-diagram">
                                    <div class="diagram-header">
                                        <span class="states-legend">
                                            <span class="legend-dot legend-current" />{" Current"}
                                            <span class="legend-dot legend-available" />{" Available"}
                                            <span class="legend-dot legend-visited" />{" Visited"}
                                        </span>
                                    </div>
                                    <div class="diagram-view">
                                        {render_state_diagram(&parsed, &current_sim, &available_targets, &visited_set, &*comments, on_diagram_click)}
                                        <div class="diagram-zoom-bar">
                                            <button class="zoom-in" title="Zoom in">{"+"}</button>
                                            <button class="zoom-out" title="Zoom out">{"\u{2212}"}</button>
                                            <button class="zoom-fit" title="Fit to view">{"\u{21BA}"}</button>
                                        </div>
                                    </div>
                                </section>
                            </div>
                        }} else { html!{} } }

                    </div>
                </div>
            </div>
        </div>
    }
}

// ─── Helpers ───

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum PortSide {
    Top,
    Bottom,
    Left,
    Right,
}

fn snap_svg(value: f64) -> f64 {
    (value * 2.0).round() / 2.0
}

fn node_port(cx: f64, cy: f64, w: f64, h: f64, side: PortSide, align: i8) -> (f64, f64) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let align_offset = |span: f64| (align as f64) * (span / 3.0);
    let point = match side {
        PortSide::Top => (cx + align_offset((w - 34.0).max(18.0)), cy - hh),
        PortSide::Bottom => (cx + align_offset((w - 34.0).max(18.0)), cy + hh),
        PortSide::Left => (cx - hw, cy),
        PortSide::Right => (cx + hw, cy),
    };
    (snap_svg(point.0), snap_svg(point.1))
}

fn horizontal_port_align(from_x: f64, to_x: f64, threshold: f64) -> i8 {
    if to_x > from_x + threshold {
        1
    } else if to_x < from_x - threshold {
        -1
    } else {
        0
    }
}

fn dedupe_polyline(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(points.len());
    for (raw_x, raw_y) in points {
        let x = snap_svg(raw_x);
        let y = snap_svg(raw_y);
        let is_distinct = out.last()
            .map(|&(px, py)| (px - x).abs() > 0.1 || (py - y).abs() > 0.1)
            .unwrap_or(true);
        if is_distinct {
            out.push((x, y));
        }
    }
    out
}

fn polyline_path(points: &[(f64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut out = format!("M {:.1},{:.1}", points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        out.push_str(&format!(" L {:.1},{:.1}", x, y));
    }
    out
}

fn trim_polyline_end(points: &[(f64, f64)], distance: f64) -> Vec<(f64, f64)> {
    if points.len() < 2 || distance <= 0.0 {
        return points.to_vec();
    }

    let mut trimmed = points.to_vec();
    let mut remaining = distance;

    while trimmed.len() >= 2 && remaining > 0.0 {
        let last_idx = trimmed.len() - 1;
        let a = trimmed[last_idx - 1];
        let b = trimmed[last_idx];
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let seg_len = dx.abs() + dy.abs();

        if seg_len <= 0.1 {
            trimmed.pop();
            continue;
        }

        if seg_len > remaining {
            let ratio = (seg_len - remaining) / seg_len;
            trimmed[last_idx] = (
                snap_svg(a.0 + dx * ratio),
                snap_svg(a.1 + dy * ratio),
            );
            break;
        }

        remaining -= seg_len;
        trimmed.pop();
    }

    trimmed
}

fn self_loop_geometry(
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    total_w: f64,
    label: &str,
) -> (Vec<(f64, f64)>, f64, f64, (f64, f64, f64, f64)) {
    let label_size = edge_label_size(label);
    let edge_x = cx + w / 2.0 - 2.0;
    let start_y = cy - h * 0.16;
    let end_y = cy + h * 0.18;
    let top_y = cy - h / 2.0 - 26.0;
    let outer_x = (cx + w / 2.0 + 58.0 + label_size.0 * 0.12)
        .min(total_w - label_size.0 / 2.0 - 12.0);

    let points = trim_polyline_end(&dedupe_polyline(vec![
        (edge_x, start_y),
        (edge_x, top_y),
        (outer_x, top_y),
        (outer_x, end_y),
        (edge_x, end_y),
    ]), 2.0);

    let label_x = snap_svg(
        ((edge_x + outer_x) / 2.0)
            .clamp(label_size.0 / 2.0 + 12.0, total_w - label_size.0 / 2.0 - 12.0),
    );
    let label_y = snap_svg((top_y - 8.0).max(label_size.1 / 2.0 + 8.0));

    let left = edge_x.min(outer_x) - 12.0;
    let right = outer_x.max(label_x + label_size.0 / 2.0) + 12.0;
    let top = (top_y - 10.0).min(label_y - label_size.1 / 2.0 - 6.0);
    let bottom = end_y + 12.0;
    let envelope = (
        snap_svg((left + right) / 2.0),
        snap_svg((top + bottom) / 2.0),
        snap_svg((right - left) / 2.0),
        snap_svg((bottom - top) / 2.0),
    );

    (points, label_x, label_y, envelope)
}

fn ranges_overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> bool {
    let (amin, amax) = if a0 <= a1 { (a0, a1) } else { (a1, a0) };
    let (bmin, bmax) = if b0 <= b1 { (b0, b1) } else { (b1, b0) };
    amin <= bmax && bmin <= amax
}

fn segment_hits_rect(a: (f64, f64), b: (f64, f64), rect: (f64, f64, f64, f64), margin: f64) -> bool {
    let (cx, cy, hw, hh) = rect;
    let left = cx - hw - margin;
    let right = cx + hw + margin;
    let top = cy - hh - margin;
    let bottom = cy + hh + margin;

    if (a.1 - b.1).abs() < 0.1 {
        a.1 >= top && a.1 <= bottom && ranges_overlap(a.0, b.0, left, right)
    } else if (a.0 - b.0).abs() < 0.1 {
        a.0 >= left && a.0 <= right && ranges_overlap(a.1, b.1, top, bottom)
    } else {
        false
    }
}

fn polyline_collision_count(points: &[(f64, f64)], rects: &[(f64, f64, f64, f64)], margin: f64) -> usize {
    let mut collisions = 0;
    for seg in points.windows(2) {
        let a = seg[0];
        let b = seg[1];
        for &rect in rects {
            if segment_hits_rect(a, b, rect, margin) {
                collisions += 1;
            }
        }
    }
    collisions
}

fn polyline_length(points: &[(f64, f64)]) -> f64 {
    points.windows(2)
        .map(|seg| (seg[1].0 - seg[0].0).abs() + (seg[1].1 - seg[0].1).abs())
        .sum()
}

fn edge_label_size(label: &str) -> (f64, f64) {
    ((label.len() as f64 * 6.2).max(28.0) + 8.0, 14.0)
}

fn summarize_edge_labels(labels: &[String]) -> String {
    if labels.len() <= 2 {
        labels.join(" · ")
    } else {
        format!("{} · {} +{}", labels[0], labels[1], labels.len() - 2)
    }
}

fn label_viewport_overflow(center: (f64, f64), size: (f64, f64), bounds: (f64, f64), margin: f64) -> f64 {
    let (cx, cy) = center;
    let (half_w, half_h) = (size.0 / 2.0, size.1 / 2.0);
    let left = cx - half_w - margin;
    let right = cx + half_w + margin;
    let top = cy - half_h - margin;
    let bottom = cy + half_h + margin;

    (0.0 - left).max(0.0)
        + (right - bounds.0).max(0.0)
        + (0.0 - top).max(0.0)
        + (bottom - bounds.1).max(0.0)
}

fn clamp_label_center(
    center: (f64, f64),
    base: (f64, f64),
    size: (f64, f64),
    bounds: (f64, f64),
    anchored_to_horizontal: bool,
) -> (f64, f64) {
    let drift_x = if anchored_to_horizontal { 38.0 } else { 18.0 };
    let drift_y = if anchored_to_horizontal { 14.0 } else { 34.0 };
    let mut x = center.0.clamp(base.0 - drift_x, base.0 + drift_x);
    let mut y = center.1.clamp(base.1 - drift_y, base.1 + drift_y);
    let margin_x = size.0 / 2.0 + 8.0;
    let margin_y = size.1 / 2.0 + 6.0;
    x = x.clamp(margin_x, bounds.0 - margin_x);
    y = y.clamp(margin_y, bounds.1 - margin_y);
    (snap_svg(x), snap_svg(y))
}

fn overlap_span(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    let (amin, amax) = if a0 <= a1 { (a0, a1) } else { (a1, a0) };
    let (bmin, bmax) = if b0 <= b1 { (b0, b1) } else { (b1, b0) };
    (amax.min(bmax) - amin.max(bmin)).max(0.0)
}

fn segment_interference_score(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64), margin: f64) -> f64 {
    let a_horizontal = (a.1 - b.1).abs() < 0.1;
    let c_horizontal = (c.1 - d.1).abs() < 0.1;

    if a_horizontal && c_horizontal {
        if (a.1 - c.1).abs() <= margin {
            let overlap = overlap_span(a.0, b.0, c.0, d.0);
            if overlap > 0.0 {
                return 1.0 + overlap / 24.0;
            }
        }
    } else if !a_horizontal && !c_horizontal {
        if (a.0 - c.0).abs() <= margin {
            let overlap = overlap_span(a.1, b.1, c.1, d.1);
            if overlap > 0.0 {
                return 1.0 + overlap / 24.0;
            }
        }
    } else {
        let (h0, h1, v0, v1) = if a_horizontal { (a, b, c, d) } else { (c, d, a, b) };
        let h_min_x = h0.0.min(h1.0);
        let h_max_x = h0.0.max(h1.0);
        let v_min_y = v0.1.min(v1.1);
        let v_max_y = v0.1.max(v1.1);
        if v0.0 >= h_min_x - margin
            && v0.0 <= h_max_x + margin
            && h0.1 >= v_min_y - margin
            && h0.1 <= v_max_y + margin
        {
            return 1.8;
        }
    }

    let endpoint_gap = [
        (a.0 - c.0).abs() + (a.1 - c.1).abs(),
        (a.0 - d.0).abs() + (a.1 - d.1).abs(),
        (b.0 - c.0).abs() + (b.1 - c.1).abs(),
        (b.0 - d.0).abs() + (b.1 - d.1).abs(),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min);

    if endpoint_gap < margin * 1.5 {
        0.35
    } else {
        0.0
    }
}

fn polyline_edge_interference(points: &[(f64, f64)], existing_paths: &[Vec<(f64, f64)>], margin: f64) -> f64 {
    let mut score = 0.0;
    for seg in points.windows(2) {
        let a = seg[0];
        let b = seg[1];
        for path in existing_paths {
            for other in path.windows(2) {
                score += segment_interference_score(a, b, other[0], other[1], margin);
            }
        }
    }
    score
}

fn label_overlaps_rect(center: (f64, f64), size: (f64, f64), rect: (f64, f64, f64, f64), margin: f64) -> bool {
    let (cx, cy) = center;
    let (hw, hh) = (size.0 / 2.0, size.1 / 2.0);
    let label_left = cx - hw;
    let label_right = cx + hw;
    let label_top = cy - hh;
    let label_bottom = cy + hh;

    let (rx, ry, rw, rh) = rect;
    let rect_left = rx - rw - margin;
    let rect_right = rx + rw + margin;
    let rect_top = ry - rh - margin;
    let rect_bottom = ry + rh + margin;

    label_left <= rect_right
        && label_right >= rect_left
        && label_top <= rect_bottom
        && label_bottom >= rect_top
}

fn polyline_label_anchor(
    points: &[(f64, f64)],
    label: &str,
    rects: &[(f64, f64, f64, f64)],
    bounds: (f64, f64),
) -> (f64, f64, bool) {
    if points.len() < 2 {
        return (0.0, 0.0, true);
    }

    let label_size = edge_label_size(label);
    let total_len = polyline_length(points).max(1.0);
    let path_mid = total_len / 2.0;
    let mut traversed = 0.0;
    let mut best_anchor = (points[0].0, points[0].1 - 6.0);
    let mut best_anchor_is_horizontal = true;
    let mut best_score = f64::INFINITY;
    let segment_count = points.len().saturating_sub(1);
    let mut best_anchor_by_tier: [Option<((f64, f64), bool, f64)>; 4] = [None, None, None, None];

    for (seg_idx, seg) in points.windows(2).enumerate() {
        let a = seg[0];
        let b = seg[1];
        let seg_len = (b.0 - a.0).abs() + (b.1 - a.1).abs();
        if seg_len <= 12.0 {
            traversed += seg_len;
            continue;
        }

        let is_horizontal = (a.1 - b.1).abs() < 0.1;
        let is_terminal_segment = seg_idx == 0 || seg_idx + 1 == segment_count;
        let samples = [0.4_f64, 0.5_f64, 0.6_f64];
        for t in samples {
            let x = a.0 + (b.0 - a.0) * t;
            let y = a.1 + (b.1 - a.1) * t;
            let along = traversed + seg_len * t;
            let center_penalty = (along - path_mid).abs();
            let endpoint_distance = seg_len * t.min(1.0 - t);

            let candidates = if is_horizontal {
                vec![(x, y - 7.0), (x, y + 10.0)]
            } else {
                let side_offset = label_size.0 / 2.0 + 6.0;
                vec![(x - side_offset, y - 2.0), (x + side_offset, y - 2.0)]
            };

            for anchor in candidates {
                let overlaps = rects.iter()
                    .filter(|&&rect| label_overlaps_rect(anchor, label_size, rect, 5.0))
                    .count() as f64;
                let overflow = label_viewport_overflow(anchor, label_size, bounds, 6.0);
                let orientation_bias = if is_horizontal { -20.0 } else { 6.0 };
                let length_bonus = -seg_len * if is_horizontal { 0.38 } else { 0.16 };
                let terminal_penalty = if is_terminal_segment { 42.0 } else { 0.0 };
                let endpoint_penalty = if endpoint_distance < 18.0 { (18.0 - endpoint_distance) * 4.0 } else { 0.0 };
                let score = overlaps * 10_000.0
                    + overflow * 250.0
                    + center_penalty * 0.34
                    + orientation_bias
                    + length_bonus
                    + terminal_penalty
                    + endpoint_penalty;
                let tier = match (is_horizontal, is_terminal_segment) {
                    (true, false) => 0,
                    (true, true) => 1,
                    (false, false) => 2,
                    (false, true) => 3,
                };
                let replace_tier = best_anchor_by_tier[tier]
                    .map(|(_, _, best)| score < best)
                    .unwrap_or(true);
                if replace_tier {
                    best_anchor_by_tier[tier] = Some((anchor, is_horizontal, score));
                }
                if score < best_score {
                    best_score = score;
                    best_anchor = anchor;
                    best_anchor_is_horizontal = is_horizontal;
                }
            }
        }

        traversed += seg_len;
    }

    for tier_choice in best_anchor_by_tier {
        if let Some((anchor, is_horizontal, score)) = tier_choice {
            if score < 10_000.0 {
                return (anchor.0, anchor.1, is_horizontal);
            }
        }
    }

    (best_anchor.0, best_anchor.1, best_anchor_is_horizontal)
}

fn render_state_diagram(
    parsed: &model::ParsedMachine,
    current_state: &str,
    available_targets: &std::collections::HashSet<String>,
    visited: &std::collections::HashSet<String>,
    comments: &[UserComment],
    on_click: Callback<String>,
) -> Html {
    if parsed.states.is_empty() {
        return html! { <p class="help-text" style="text-align:center;padding:40px 20px">
            {"Parse a spec to see the state diagram."}
        </p> };
    }

    let graph_layout = compute_state_diagram_layout(parsed);
    let start = graph_layout.start.clone();
    let depth = &graph_layout.depths;
    let pos = &graph_layout.positions;
    let node_rect_map = &graph_layout.node_rects;
    let edge_groups = &graph_layout.edge_groups;
    let total_w = graph_layout.total_w;
    let total_h = graph_layout.total_h;
    let node_w = graph_layout.node_w;
    let node_h = graph_layout.node_h;
    let side_label_width = graph_layout.side_label_width;
    let pad = graph_layout.pad;
    let init_space = graph_layout.init_space;
    let v_gap = graph_layout.v_gap;
    let view_bounds = (total_w, total_h);

    let mut bidir = std::collections::HashSet::new();
    for (from, to) in edge_groups.keys() {
        if from != to && edge_groups.contains_key(&(to.clone(), from.clone())) {
            bidir.insert((from.clone(), to.clone()));
        }
    }

    let viewbox = format!("0 0 {:.0} {:.0}", total_w, total_h);

    // Init arrow: filled dot with arrow to start state
    let init_arrow = pos.get(&start).map(|&(sx, sy)| {
        let dot_y = sy - node_h / 2.0 - 22.0;
        html! {
            <g class="dia-init">
                <circle cx={format!("{:.1}", sx)} cy={format!("{:.1}", dot_y)} r="5" />
                <line
                    x1={format!("{:.1}", sx)} y1={format!("{:.1}", dot_y + 5.0)}
                    x2={format!("{:.1}", sx)} y2={format!("{:.1}", sy - node_h / 2.0)}
                />
            </g>
        }
    }).unwrap_or(html!{});

    // Terminal indicators: arrow to double-circle
    let from_set: std::collections::HashSet<&str> = parsed.actions.iter()
        .flat_map(|a| a.from.iter().map(|s| s.as_str())).collect();
    let terminal_html: Vec<Html> = parsed.states.iter()
        .filter(|s| !from_set.contains(s.as_str()))
        .filter_map(|state| pos.get(state).map(|&(x, y)| {
            let dot_y = y + node_h / 2.0 + 22.0;
            html! {
                <g class="dia-terminal">
                    <line
                        x1={format!("{:.1}", x)} y1={format!("{:.1}", y + node_h / 2.0)}
                        x2={format!("{:.1}", x)} y2={format!("{:.1}", dot_y - 6.0)}
                    />
                    <circle cx={format!("{:.1}", x)} cy={format!("{:.1}", dot_y)} r="5" class="terminal-outer" />
                    <circle cx={format!("{:.1}", x)} cy={format!("{:.1}", dot_y)} r="3" class="terminal-inner" />
                </g>
            }
        })).collect();

    // Build edge data (path string + initial label position)
    struct EdgeData {
        path: String,
        label: String,
        lx: f64,
        ly: f64,
        base_lx: f64,
        base_ly: f64,
        label_is_horizontal: bool,
        is_available: bool,
    }
    let mut edge_data: Vec<EdgeData> = Vec::new();
    let mut placed_paths: Vec<Vec<(f64, f64)>> = Vec::new();
    let row_top = |level: usize| pad + init_space + (level as f64) * (node_h + v_gap);
    let row_bottom = |level: usize| row_top(level) + node_h;
    let channel_y = |upper_level: usize| row_bottom(upper_level) + v_gap / 2.0;
    let min_left = node_rect_map.values().map(|&(x, _, hw, _)| x - hw).fold(total_w, f64::min);
    let max_right = node_rect_map.values().map(|&(x, _, hw, _)| x + hw).fold(0.0, f64::max);
    let side_label_half = side_label_width / 2.0;
    let left_lane_x = (min_left - 28.0).max(side_label_half + 10.0);
    let right_lane_x = (max_right + 28.0).min(total_w - side_label_half - 10.0);
    let self_loop_envelopes: std::collections::BTreeMap<String, (f64, f64, f64, f64)> = edge_groups
        .iter()
        .filter(|((from, to), _)| from == to)
        .filter_map(|((state, _), labels)| {
            pos.get(state).map(|&(x, y)| {
                let loop_label = summarize_edge_labels(labels);
                let (_, _, _, envelope) = self_loop_geometry(x, y, node_w, node_h, total_w, &loop_label);
                (state.clone(), envelope)
            })
        })
        .collect();

    for ((from, to), labels) in edge_groups {
        let label = summarize_edge_labels(labels);
        let (&(fx, fy), &(tx, ty)) = match (pos.get(from), pos.get(to)) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };
        let is_available = from.as_str() == current_state;
        let from_level = *depth.get(from).unwrap_or(&0);
        let to_level = *depth.get(to).unwrap_or(&0);
        let mut avoid_rects: Vec<(f64, f64, f64, f64)> = node_rect_map.iter()
            .filter(|(state, _)| *state != from && *state != to)
            .map(|(_, &rect)| rect)
            .collect();
        avoid_rects.extend(self_loop_envelopes.values().copied());

        if from == to {
            let (points, lx, ly, _) = self_loop_geometry(fx, fy, node_w, node_h, total_w, &label);
            let path = polyline_path(&points);
            edge_data.push(EdgeData {
                path,
                label,
                lx,
                ly,
                base_lx: lx,
                base_ly: ly,
                label_is_horizontal: true,
                is_available,
            });
            placed_paths.push(points);
        } else {
            let is_bidir = bidir.contains(&(from.clone(), to.clone()));
            let lane_shift = if is_bidir {
                if from < to { -10.0 } else { 10.0 }
            } else {
                0.0
            };
            let align_threshold = node_w * 0.18;
            let mut candidates: Vec<(Vec<(f64, f64)>, f64)> = Vec::new();

            if from_level == to_level {
                let start_side = if tx >= fx { PortSide::Right } else { PortSide::Left };
                let end_side = if tx >= fx { PortSide::Left } else { PortSide::Right };
                let start = node_port(fx, fy, node_w, node_h, start_side, 0);
                let end = node_port(tx, ty, node_w, node_h, end_side, 0);
                let horizontal_clearance = (end.0 - start.0).abs();
                let same_rank_lane_offset = if is_bidir { 40.0 } else { 28.0 };
                let above_y = row_top(from_level) - same_rank_lane_offset + lane_shift;
                let below_y = row_bottom(from_level) + same_rank_lane_offset + lane_shift;
                let prefer_above = from < to;
                let stub_extent = ((horizontal_clearance / 2.0) - 8.0).clamp(0.0, 16.0);
                let start_stub_x = start.0 + if matches!(start_side, PortSide::Right) { stub_extent } else { -stub_extent };
                let end_stub_x = end.0 + if matches!(end_side, PortSide::Right) { stub_extent } else { -stub_extent };

                if !is_bidir && horizontal_clearance >= 14.0 {
                    candidates.push((
                        dedupe_polyline(vec![start, end]),
                        -14.0,
                    ));
                }

                candidates.push((
                    dedupe_polyline(vec![
                        start,
                        (start_stub_x, start.1),
                        (start_stub_x, above_y),
                        (end_stub_x, above_y),
                        (end_stub_x, end.1),
                        end,
                    ]),
                    if prefer_above { 0.0 } else { 10.0 },
                ));
                candidates.push((
                    dedupe_polyline(vec![
                        start,
                        (start_stub_x, start.1),
                        (start_stub_x, below_y),
                        (end_stub_x, below_y),
                        (end_stub_x, end.1),
                        end,
                    ]),
                    if prefer_above { 10.0 } else { 0.0 },
                ));
            } else if to_level > from_level {
                let start = node_port(fx, fy, node_w, node_h, PortSide::Bottom, horizontal_port_align(fx, tx, align_threshold));
                let end = node_port(tx, ty, node_w, node_h, PortSide::Top, horizontal_port_align(tx, fx, align_threshold));
                let exit_y = channel_y(from_level) + lane_shift;
                let entry_y = channel_y(to_level - 1) + lane_shift;

                if to_level == from_level + 1 {
                    candidates.push((
                        dedupe_polyline(vec![start, (start.0, exit_y), (end.0, exit_y), end]),
                        0.0,
                    ));
                }

                candidates.push((
                    dedupe_polyline(vec![start, (start.0, exit_y), (left_lane_x, exit_y), (left_lane_x, entry_y), (end.0, entry_y), end]),
                    6.0,
                ));
                candidates.push((
                    dedupe_polyline(vec![start, (start.0, exit_y), (right_lane_x, exit_y), (right_lane_x, entry_y), (end.0, entry_y), end]),
                    6.0,
                ));
            } else {
                let start = node_port(fx, fy, node_w, node_h, PortSide::Top, horizontal_port_align(fx, tx, align_threshold));
                let end = node_port(tx, ty, node_w, node_h, PortSide::Bottom, horizontal_port_align(tx, fx, align_threshold));
                let exit_y = channel_y(from_level - 1) + lane_shift;
                let entry_y = channel_y(to_level) + lane_shift;

                if from_level == to_level + 1 {
                    candidates.push((
                        dedupe_polyline(vec![start, (start.0, exit_y), (end.0, exit_y), end]),
                        0.0,
                    ));
                }

                candidates.push((
                    dedupe_polyline(vec![start, (start.0, exit_y), (left_lane_x, exit_y), (left_lane_x, entry_y), (end.0, entry_y), end]),
                    6.0,
                ));
                candidates.push((
                    dedupe_polyline(vec![start, (start.0, exit_y), (right_lane_x, exit_y), (right_lane_x, entry_y), (end.0, entry_y), end]),
                    6.0,
                ));
            }

            let mut best_points: Vec<(f64, f64)> = Vec::new();
            let mut best_score = f64::INFINITY;
            for (points, bias) in candidates {
                let collisions = polyline_collision_count(&points, &avoid_rects, 8.0) as f64;
                let edge_interference = polyline_edge_interference(&points, &placed_paths, 10.0);
                let score = collisions * 10_000.0 + edge_interference * 120.0 + polyline_length(&points) + bias;
                if score < best_score {
                    best_score = score;
                    best_points = points;
                }
            }

            let draw_points = trim_polyline_end(&best_points, 2.0);
            let d = polyline_path(&draw_points);
            let all_rects: Vec<(f64, f64, f64, f64)> = node_rect_map.values().copied().collect();
            let (base_lx, base_ly_raw, label_is_horizontal) = polyline_label_anchor(&draw_points, &label, &all_rects, view_bounds);
            let base_ly = base_ly_raw - 6.0;
            edge_data.push(EdgeData {
                path: d,
                label,
                lx: base_lx,
                ly: base_ly,
                base_lx,
                base_ly,
                label_is_horizontal,
                is_available,
            });
            placed_paths.push(draw_points);
        }
    }

    // Resolve overlapping labels: greedy nudge pass (label-vs-label AND label-vs-node)
    {
        let char_est = 6.5_f64;
        let label_h = 12.0_f64;
        let pad_x = 6.0_f64;
        let pad_y = 2.0_f64;

        // Collect node rects for avoidance  (cx, cy, half-w, half-h)
        let node_rects: Vec<(f64, f64, f64, f64)> = node_rect_map.values().copied().collect();

        for _ in 0..6 {
            // Push labels away from nodes
            for e in edge_data.iter_mut() {
                let ew = e.label.len() as f64 * char_est / 2.0 + pad_x;
                let eh = label_h / 2.0 + pad_y;
                for &(nx, ny, nhw, nhh) in &node_rects {
                    let dx = e.lx - nx;
                    let dy = e.ly - ny;
                    let overlap_x = ew + nhw - dx.abs();
                    let overlap_y = eh + nhh - dy.abs();
                    if overlap_x > 0.0 && overlap_y > 0.0 {
                        if overlap_y <= overlap_x {
                            let sign = if dy >= 0.0 { 1.0 } else { -1.0 };
                            e.ly += (overlap_y + 2.0) * sign;
                        } else {
                            let sign = if dx >= 0.0 { 1.0 } else { -1.0 };
                            e.lx += (overlap_x + 2.0) * sign;
                        }
                    }
                }
            }
            // Push labels away from each other
            for i in 0..edge_data.len() {
                for j in (i + 1)..edge_data.len() {
                    let wi = edge_data[i].label.len() as f64 * char_est + pad_x;
                    let wj = edge_data[j].label.len() as f64 * char_est + pad_x;
                    let hi = label_h + pad_y;
                    let hj = label_h + pad_y;
                    let dx = edge_data[j].lx - edge_data[i].lx;
                    let dy = edge_data[j].ly - edge_data[i].ly;
                    let overlap_x = (wi + wj) / 2.0 - dx.abs();
                    let overlap_y = (hi + hj) / 2.0 - dy.abs();
                    if overlap_x > 0.0 && overlap_y > 0.0 {
                        if overlap_y <= overlap_x {
                            let shift = overlap_y / 2.0 + 1.0;
                            let sign = if dy >= 0.0 { 1.0 } else { -1.0 };
                            edge_data[i].ly -= shift * sign;
                            edge_data[j].ly += shift * sign;
                        } else {
                            let shift = overlap_x / 2.0 + 1.0;
                            let sign = if dx >= 0.0 { 1.0 } else { -1.0 };
                            edge_data[i].lx -= shift * sign;
                            edge_data[j].lx += shift * sign;
                        }
                    }
                }
            }

            for e in edge_data.iter_mut() {
                let size = edge_label_size(&e.label);
                let (lx, ly) = clamp_label_center(
                    (e.lx, e.ly),
                    (e.base_lx, e.base_ly),
                    size,
                    view_bounds,
                    e.label_is_horizontal,
                );
                e.lx = lx;
                e.ly = ly;
            }
        }
    }

    // Layer active edges separately so the current route sits above background connections.
    let inactive_edge_paths_html: Vec<Html> = edge_data.iter()
        .filter(|e| !e.is_available)
        .map(|e| html! { <path class="dia-epath" d={e.path.clone()} /> })
        .collect();
    let active_edge_underlay_html: Vec<Html> = edge_data.iter()
        .filter(|e| e.is_available)
        .map(|e| html! { <path class="dia-epath-active-underlay" d={e.path.clone()} /> })
        .collect();
    let active_edge_paths_html: Vec<Html> = edge_data.iter()
        .filter(|e| e.is_available)
        .map(|e| html! { <path class="dia-epath available" d={e.path.clone()} /> })
        .collect();
    let inactive_edge_labels_html: Vec<Html> = edge_data.iter()
        .filter(|e| !e.is_available)
        .map(|e| {
            html! {
                <text x={format!("{:.1}", e.lx)} y={format!("{:.1}", e.ly)} class="dia-elabel">{e.label.clone()}</text>
            }
        })
        .collect();
    let active_edge_labels_html: Vec<Html> = edge_data.iter()
        .filter(|e| e.is_available)
        .map(|e| {
            html! {
                <text x={format!("{:.1}", e.lx)} y={format!("{:.1}", e.ly)} class="dia-elabel available">{e.label.clone()}</text>
            }
        })
        .collect();

    // Build node HTML
    let nodes_html: Vec<Html> = parsed.states.iter().map(|state| {
        let (x, y) = pos.get(state).copied().unwrap_or((0.0, 0.0));
        let is_current = state.as_str() == current_state;
        let is_available = available_targets.contains(state.as_str());
        let is_visited = visited.contains(state.as_str()) && !is_current;
        let count = comments.iter().filter(|c| c.target == *state).count();
        let mut cls = String::from("dia-node");
        if is_current { cls.push_str(" current"); }
        if is_available { cls.push_str(" available"); }
        if is_visited && !is_available { cls.push_str(" visited"); }
        let name = state.clone();
        // Only available transition targets are clickable
        let onclick = if is_available {
            let cb = on_click.reform(move |_: MouseEvent| name.clone());
            Some(cb)
        } else {
            None
        };
        html! {
            <g class={cls} onclick={onclick}>
                <rect
                    x={format!("{:.1}", x - node_w / 2.0)}
                    y={format!("{:.1}", y - node_h / 2.0)}
                    width={format!("{:.1}", node_w)}
                    height={format!("{:.1}", node_h)}
                    rx="6"
                />
                <text x={format!("{:.1}", x)} y={format!("{:.1}", y + 4.5)} class="dia-nlabel">{state}</text>
                { if count > 0 { html! {
                    <g class="dia-badge">
                        <circle
                            cx={format!("{:.1}", x + node_w / 2.0 - 2.0)}
                            cy={format!("{:.1}", y - node_h / 2.0 + 2.0)}
                            r="8"
                        />
                        <text
                            x={format!("{:.1}", x + node_w / 2.0 - 2.0)}
                            y={format!("{:.1}", y - node_h / 2.0 + 6.0)}
                        >{count.to_string()}</text>
                    </g>
                }} else { html!{} } }
            </g>
        }
    }).collect();

    html! {
        <svg viewBox={viewbox} preserveAspectRatio="xMidYMid meet" class="state-diagram">
            <defs>
                <marker id="dia-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto" markerUnits="userSpaceOnUse">
                    <polygon points="0 0, 8 3, 0 6" style="fill: var(--border); stroke: var(--surface-alt); stroke-width: 1.1" />
                </marker>
                <marker id="dia-arrow-available" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto" markerUnits="userSpaceOnUse">
                    <polygon points="0 0, 8 3, 0 6" style="fill: var(--green); stroke: var(--surface-alt); stroke-width: 1.1" />
                </marker>
            </defs>
            {init_arrow}
            { for terminal_html }
            <g class="dia-edges">{ for inactive_edge_paths_html }</g>
            <g class="dia-edges dia-edges-active-underlay">{ for active_edge_underlay_html }</g>
            <g class="dia-edges dia-edges-active">{ for active_edge_paths_html }</g>
            { for nodes_html }
            <g class="dia-edge-labels">{ for inactive_edge_labels_html }</g>
            <g class="dia-edge-labels dia-edge-labels-active">{ for active_edge_labels_html }</g>
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_label_center, dedupe_polyline, polyline_edge_interference, segment_hits_rect, trim_polyline_end};

    #[test]
    fn dedupe_polyline_snaps_to_half_pixels() {
        let points = dedupe_polyline(vec![(10.24, 4.74), (10.21, 4.71), (14.74, 4.74)]);
        assert_eq!(points, vec![(10.0, 4.5), (14.5, 4.5)]);
    }

    #[test]
    fn trim_polyline_end_pulls_back_last_segment() {
        let points = vec![(0.0, 0.0), (12.0, 0.0), (12.0, 10.0)];
        let trimmed = trim_polyline_end(&points, 2.0);
        assert_eq!(trimmed, vec![(0.0, 0.0), (12.0, 0.0), (12.0, 8.0)]);
    }

    #[test]
    fn segment_hits_rect_detects_axis_aligned_overlap() {
        let hits = segment_hits_rect((0.0, 5.0), (10.0, 5.0), (5.0, 5.0, 2.0, 2.0), 0.0);
        assert!(hits);
    }

    #[test]
    fn polyline_edge_interference_penalizes_crossing_routes() {
        let candidate = vec![(0.0, 5.0), (10.0, 5.0)];
        let existing = vec![vec![(6.0, 0.0), (6.0, 10.0)]];
        assert!(polyline_edge_interference(&candidate, &existing, 1.0) > 1.0);
    }

    #[test]
    fn clamp_label_center_keeps_label_near_anchor_and_inside_bounds() {
        let clamped = clamp_label_center((140.0, 2.0), (100.0, 20.0), (60.0, 14.0), (160.0, 120.0), true);
        assert_eq!(clamped, (122.0, 13.0));
    }
}

fn show_toast(msg: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(body) = doc.body() {
                let tip = doc.create_element("span").unwrap();
                tip.set_class_name("copy-toast");
                tip.set_text_content(Some(msg));
                let _ = tip.set_attribute("role", "status");
                let _ = tip.set_attribute("aria-live", "polite");
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
