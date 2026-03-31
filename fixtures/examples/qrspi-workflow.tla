---- MODULE QRSPIWorkflow ----

\* QRSPI: THE HUMAN-AI PARTNERSHIP IN ACTION
\*
\* Questions, Research, Spec/Design, Plan, Implement — five phases
\* that keep the engineer in the driver's seat while the AI handles
\* volume. The insight: the engineer's judgment is the scarce
\* resource. Their ability to ask good questions, evaluate
\* tradeoffs, and make design calls is what the AI can't do alone.
\*
\* By splitting a monolithic planning prompt into focused steps,
\* QRSPI gives the engineer a review gate at every phase where
\* their expertise matters most.
\*
\* From Dex Horthy's evolution of RPI (Research, Plan, Implement):
\* "Don't use prompts for control flow — use control flow for control
\* flow. Don't outsource the thinking. Seek leverage."
\*
\* Reference implementation: https://github.com/jaeyunha/QRSPI-workflow

VARIABLE taskState

TaskStages == {
  "TicketReceived",
  "QuestionsGenerated",
  "ResearchComplete",
  "DesignDrafted",
  "DesignApproved",
  "StructureOutlined",
  "StructureApproved",
  "PlanWritten",
  "PlanApproved",
  "Implementing",
  "CodeReview",
  "Shipped",
  "QuestionsInsufficient",
  "DesignRejected",
  "StructureRejected",
  "PlanRejected",
  "ImplementationBlocked"
}

\* ================================================================
\* THE FIVE PHASES — Where human judgment meets AI leverage
\* ================================================================

(* PHASE 1: QUESTIONS
   The engineer translates a ticket into 5-12 precise research
   questions. This step matters because the right questions
   determine whether the research phase turns up useful facts
   or noise. The questions act as a query planner — touching all
   relevant code areas without revealing implementation intent.
   Keeping the researcher unaware of the goal keeps the facts
   objective. *)
GenerateQuestions ==
  /\ taskState = "TicketReceived"
  /\ taskState' = "QuestionsGenerated"

(* PHASE 2: RESEARCH
   The AI answers the questions in a fresh context — the ticket is
   hidden. Without knowing what is being built, the model can't
   inject opinions into what should be objective facts. The output
   is a facts-only document with file:line references. This is
   where AI earns its keep: tireless, thorough research that would
   take a person hours, done in minutes. *)
ConductResearch ==
  /\ taskState = "QuestionsGenerated"
  /\ taskState' = "ResearchComplete"

(* PHASE 3: DESIGN
   Now the engineer reunites the ticket with the research. They
   see patterns the AI missed, connect dots across domains, and
   make the design choices that shape everything downstream.
   Catching a wrong pattern in a 200-line design doc costs one
   conversation. Catching it in 1000 lines of code costs a
   rewrite. *)
DraftDesign ==
  /\ taskState = "ResearchComplete"
  /\ taskState' = "DesignDrafted"

(* The engineer reviews the design: current state, desired end
   state, patterns to follow, patterns to avoid, and the design
   decisions with alternatives considered. Approval here means
   alignment is locked — the team has chosen its direction, and
   every subsequent step serves that choice. *)
ApproveDesign ==
  /\ taskState = "DesignDrafted"
  /\ taskState' = "DesignApproved"

(* PHASE 4: STRUCTURE
   The engineer outlines vertical implementation phases — tracer
   bullets, not horizontal layers. Each phase is a DB + API + UI
   slice that can be tested independently. Knowing what to build
   first so the team learns the most from the least effort is a
   skill that comes from experience. *)
OutlineStructure ==
  /\ taskState = "DesignApproved"
  /\ taskState' = "StructureOutlined"

(* The engineer confirms phase ordering and test checkpoints.
   Vertical slices mean each phase delivers real value — if
   Phase 2 hits trouble, Phase 1 is already working. *)
ApproveStructure ==
  /\ taskState = "StructureOutlined"
  /\ taskState' = "StructureApproved"

(* PHASE 5: PLAN
   The hard thinking is done. Design decisions are made, phase
   ordering is approved, patterns are chosen. The plan adds
   tactical specifics — the mechanical details where AI is
   most useful. *)
WritePlan ==
  /\ taskState = "StructureApproved"
  /\ taskState' = "PlanWritten"

ApprovePlan ==
  /\ taskState = "PlanWritten"
  /\ taskState' = "PlanApproved"

(* IMPLEMENTATION
   The AI writes code; the engineer steers architecture and
   catches edge cases. Each vertical slice is built, tested,
   and verified before the next. The engineer's review is what
   turns generated code into production quality. *)
BeginImplementation ==
  /\ taskState = "PlanApproved"
  /\ taskState' = "Implementing"

SubmitForReview ==
  /\ taskState = "Implementing"
  /\ taskState' = "CodeReview"

ShipIt ==
  /\ taskState = "CodeReview"
  /\ taskState' = "Shipped"

\* ================================================================
\* LEARNING LOOPS — Where course-correction builds mastery
\* ================================================================

(* The research revealed gaps the questions didn't cover. Each
   round of better questions makes the engineer sharper at
   scoping problems. Learning to ask good questions is one of
   the most transferable skills there is. *)
QuestionsNeedRework ==
  /\ taskState = "ResearchComplete"
  /\ taskState' = "QuestionsInsufficient"

RegenerateQuestions ==
  /\ taskState = "QuestionsInsufficient"
  /\ taskState' = "QuestionsGenerated"

(* Design review surfaces a better approach. The engineer sees
   the bigger picture — team conventions, maintenance burden,
   long-term cost. Catching this now, in a conversation instead
   of a rewrite, is the whole point of the review gate. *)
RejectDesign ==
  /\ taskState = "DesignDrafted"
  /\ taskState' = "DesignRejected"

ReviseDesign ==
  /\ taskState = "DesignRejected"
  /\ taskState' = "DesignDrafted"

(* Structure review refines the build order or adds test
   checkpoints the first pass missed. The instinct for what to
   build first is honed by experience. *)
RejectStructure ==
  /\ taskState = "StructureOutlined"
  /\ taskState' = "StructureRejected"

ReviseStructure ==
  /\ taskState = "StructureRejected"
  /\ taskState' = "StructureOutlined"

(* Tactical details need adjustment — wrong commands, missing
   edge cases, a better library choice. Domain knowledge catches
   what automation can't. *)
RejectPlan ==
  /\ taskState = "PlanWritten"
  /\ taskState' = "PlanRejected"

RevisePlan ==
  /\ taskState = "PlanRejected"
  /\ taskState' = "PlanWritten"

(* Implementation reveals something the design couldn't predict.
   Every experienced engineer knows this moment: the code teaches
   you something the diagram couldn't. Going back to design with
   that knowledge isn't setback — it's how good software gets
   made. *)
ImplementationBlocked ==
  /\ taskState = "Implementing"
  /\ taskState' = "ImplementationBlocked"

RevisitDesignFromBlock ==
  /\ taskState = "ImplementationBlocked"
  /\ taskState' = "DesignDrafted"

(* Code review sends the work back. A colleague's fresh eyes
   catch what familiarity made invisible. *)
RequestChanges ==
  /\ taskState = "CodeReview"
  /\ taskState' = "Implementing"

Init == taskState = "TicketReceived"

Next ==
  \/ GenerateQuestions
  \/ ConductResearch
  \/ DraftDesign
  \/ ApproveDesign
  \/ OutlineStructure
  \/ ApproveStructure
  \/ WritePlan
  \/ ApprovePlan
  \/ BeginImplementation
  \/ SubmitForReview
  \/ ShipIt
  \/ QuestionsNeedRework
  \/ RegenerateQuestions
  \/ RejectDesign
  \/ ReviseDesign
  \/ RejectStructure
  \/ ReviseStructure
  \/ RejectPlan
  \/ RevisePlan
  \/ ImplementationBlocked
  \/ RevisitDesignFromBlock
  \/ RequestChanges

====
