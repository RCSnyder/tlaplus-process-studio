---- MODULE TeamDelivery ----

\* THE CRAFT OF SHIPPING SOFTWARE
\*
\* Working software is the result of a chain of decisions.
\* Someone spots a gap. An engineer picks the right abstraction.
\* A reviewer catches the edge case nobody saw. A tester asks
\* "but what if?" — and the system gets better.
\*
\* This state machine maps the journey from insight to running
\* code in a user's hands.

VARIABLE workState

WorkStages == {
  "Backlog",
  "Triaged",
  "SprintReady",
  "InProgress",
  "CodeReview",
  "ChangesRequested",
  "QATesting",
  "QARejected",
  "StagingVerified",
  "ProductionReleased",
  "HotfixNeeded",
  "Blocked",
  "Parked",
  "Cancelled"
}

\* ================================================================
\* THE CRAFT OF DELIVERY — From insight to impact
\* ================================================================

(* Every feature starts as someone's observation: a pattern in
   support tickets, a customer frustration, a chance to simplify.
   The backlog is where that awareness gets written down. *)
TriageWork ==
  /\ workState = "Backlog"
  /\ workState' = "Triaged"

(* The team gathers around a problem and asks: what does "done"
   look like? What do we know? What might surprise us? A well-
   refined ticket carries enough shared context that any teammate
   can pick it up and run. *)
RefineForSprint ==
  /\ workState = "Triaged"
  /\ workState' = "SprintReady"

(* An engineer takes ownership. They bring their own way of
   thinking about edge cases, patterns they've seen before,
   intuition built over years. Good work comes from clarity
   about what to build and the freedom to build it well. *)
StartWork ==
  /\ workState = "SprintReady"
  /\ workState' = "InProgress"

(* Code written, tests passing, PR opened. The pull request is
   an invitation to collaborate — someone else's eyes on the
   problem almost always make it better. *)
SubmitForReview ==
  /\ workState = "InProgress"
  /\ workState' = "CodeReview"

(* A colleague reads the code, shares their perspective, and
   helps the author see what they couldn't see alone. Review
   conversations strengthen both the code and the team. *)
PassReview ==
  /\ workState = "CodeReview"
  /\ workState' = "QATesting"

(* Automated tests confirm what we expect. Human testers discover
   what we never imagined — they ask "does this actually feel
   right to use?" That judgment protects the people who depend
   on this software every day. *)
PassQA ==
  /\ workState = "QATesting"
  /\ workState' = "StagingVerified"

(* The work reaches the people it was built for. A good release
   is quiet — it just works, because everyone along the way
   got their part right. *)
ReleaseToProd ==
  /\ workState = "StagingVerified"
  /\ workState' = "ProductionReleased"

\* ================================================================
\* FEEDBACK LOOPS — Where the team grows stronger
\* ================================================================

(* When a reviewer suggests a different approach, both people
   learn something. The author sees a new angle; the reviewer
   deepens their understanding of the codebase. Healthy teams
   see review rounds as part of the work, not rework. *)
RequestCodeChanges ==
  /\ workState = "CodeReview"
  /\ workState' = "ChangesRequested"

AddressChanges ==
  /\ workState = "ChangesRequested"
  /\ workState' = "CodeReview"

(* When QA catches an issue, the process is working as designed.
   Someone with judgment just protected every user who would
   have hit that bug. *)
FailQA ==
  /\ workState = "QATesting"
  /\ workState' = "QARejected"

FixQAIssues ==
  /\ workState = "QARejected"
  /\ workState' = "InProgress"

\* ================================================================
\* NAVIGATING CHANGE — Where adaptability shines
\* ================================================================

(* The path forward needs something that isn't ready yet — a
   cross-team decision, a leadership call, a missing resource.
   Flagging a blocker early saves the whole team from a bigger
   problem later. *)
HitBlocker ==
  /\ workState = "InProgress"
  /\ workState' = "Blocked"

ResolveBlocker ==
  /\ workState = "Blocked"
  /\ workState' = "InProgress"

(* Sometimes the right move is to pause. Parking work with its
   context preserved means the team can pivot without losing
   what they've built. It'll be here when the time is right. *)
ParkWork ==
  /\ workState = "InProgress"
  /\ workState' = "Parked"

ResumePreviousWork ==
  /\ workState = "Parked"
  /\ workState' = "SprintReady"

(* Production needs urgent attention. The team's deep knowledge
   of the system is what makes a fast, safe fix possible —
   that understanding lives in the people who built it. *)
ProductionIncident ==
  /\ workState = "ProductionReleased"
  /\ workState' = "HotfixNeeded"

ShipHotfix ==
  /\ workState = "HotfixNeeded"
  /\ workState' = "ProductionReleased"

(* Not every idea should become code. Knowing when to stop is
   part of the job. The essence of strategy is deciding what
   not to do — cancellation protects the team's capacity for
   the work that matters most. *)
CancelWork ==
  /\ workState = "Triaged"
  /\ workState' = "Cancelled"

Init == workState = "Backlog"

Next ==
  \/ TriageWork
  \/ RefineForSprint
  \/ StartWork
  \/ SubmitForReview
  \/ PassReview
  \/ PassQA
  \/ ReleaseToProd
  \/ RequestCodeChanges
  \/ AddressChanges
  \/ FailQA
  \/ FixQAIssues
  \/ HitBlocker
  \/ ResolveBlocker
  \/ ParkWork
  \/ ResumePreviousWork
  \/ ProductionIncident
  \/ ShipHotfix
  \/ CancelWork

====
