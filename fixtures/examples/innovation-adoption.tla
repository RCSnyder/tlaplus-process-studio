---- MODULE InnovationAdoption ----

\* GROWING YOUR ORGANIZATION'S CAPABILITIES
\*
\* Clayton Christensen showed that steel minimills didn't compete
\* with integrated mills head-on. They started with rebar — the
\* lowest-margin product incumbents were happy to give up — then
\* moved to angle iron, then structural steel, then sheet steel.
\* Each step upmarket was funded by the previous one.
\*
\* César Hidalgo's "The Infinite Alphabet" adds a deeper layer:
\* knowledge doesn't transfer by moving information — it transfers
\* by growing the capacity to recombine it. An organization's
\* ability to absorb new technology depends on the skills, tools,
\* and tacit know-how already embedded in its people. You can't
\* adopt what you can't recombine. The beachhead strategy works
\* because each adoption grows the alphabet, making the next one
\* possible.
\*
\* Personal computers, digital photography, streaming, AI agents —
\* each followed the same arc: start where the opportunity is
\* clearest, prove value through the people closest to the work,
\* and compound what you learn into the next step.
\*
\* The comments below tell you what to do at each stage and why.

VARIABLE adoptionState

AdoptionStages == {
  "SignalDetected",
  "LandscapeMapped",
  "ConstraintsIdentified",
  "BeachheadSelected",
  "PrototypeBuilt",
  "ValueMeasured",
  "PilotDeployed",
  "FeedbackIntegrated",
  "ProcessRedesigned",
  "TeamTrained",
  "ScaleApproved",
  "ProductionDeployed",
  "NextBeachheadIdentified",
  "CapabilityCompounding",
  "HypeCaptured",
  "PilotPurgatory",
  "VendorLocked",
  "ShadowAdoption",
  "ResistanceEncountered",
  "TransformationTheater",
  "BudgetCut"
}

\* ================================================================
\* THE ADOPTION PATH — Growing capability one step at a time
\* ================================================================

(* Someone on your team notices a shift — not from a vendor pitch,
   but from a signal in the work itself. A colleague automated a
   task that used to take a day. Another team shipped in half the
   expected time. A new tool keeps coming up in conversation.

   WHAT TO DO: Collect signals for 2-4 weeks. Note who is excited,
   what problems they're solving, and which claims seem grounded.
   Read primary sources — papers, docs, changelogs. *)
MapTheLandscape ==
  /\ adoptionState = "SignalDetected"
  /\ adoptionState' = "LandscapeMapped"

(* Survey the field. Who are the builders? What works today vs.
   what is still a promising demo? Where is the technology on its
   growth curve — early, accelerating, or maturing?

   WHAT TO DO: Build a landscape map with three columns —
   (1) works now, reliable; (2) works now, needs care;
   (3) impressive but not production-ready. Start in column 1. *)
IdentifyConstraints ==
  /\ adoptionState = "LandscapeMapped"
  /\ adoptionState' = "ConstraintsIdentified"

(* Know your own terrain. What are the real constraints —
   regulatory, technical, cultural, economic? Where are talented
   people doing repetitive work that doesn't use their potential?

   Hidalgo's insight: your organization already has an "alphabet"
   of capabilities. You can only adopt what you can recombine
   with letters you already know. Map the alphabet before
   choosing where to grow it.

   WHAT TO DO: Interview 5-10 people who do the daily work.
   Ask: "What part of your job is repetitive but requires context
   only you have?" That intersection — repetitive AND contextual —
   is where new technology can free people for higher-value work.
   Document the current process. *)
SelectBeachhead ==
  /\ adoptionState = "ConstraintsIdentified"
  /\ adoptionState' = "BeachheadSelected"

(* The minimill strategy: pick the use case where the team will
   learn the most with the least risk. Not the CEO's pet project.
   Not the mission-critical system. Pick the "rebar."

   WHAT TO DO: Choose a use case that (a) runs often enough to
   generate learning fast, (b) has a well-understood current
   process so you can measure improvement, (c) involves people
   genuinely curious about trying something new, (d) has stakes
   that allow honest experimentation. Write a one-page brief:
   current state, proposed change, success metric, timeline.
   Two weeks max. *)
BuildPrototype ==
  /\ adoptionState = "BeachheadSelected"
  /\ adoptionState' = "PrototypeBuilt"

(* Build a working thing, not a slide deck. It doesn't need
   polish — it needs to do the task end-to-end once, so the
   team can see what actually happens when the technology meets
   the real work.

   WHAT TO DO: Timebox to 1-2 weeks. Use off-the-shelf
   components — Christensen showed that the best innovations
   are "technologically straightforward, consisting of off-the-shelf
   components put together in a product architecture that was
   often simpler than prior approaches." Ship something that
   works. *)
MeasureValue ==
  /\ adoptionState = "PrototypeBuilt"
  /\ adoptionState' = "ValueMeasured"

(* Measure with rigor. Time reclaimed, quality improved, new
   capabilities unlocked — let the numbers tell the story.

   WHAT TO DO: Run the prototype on 10-20 real tasks alongside
   the existing process. Record time, quality, and user experience
   for both. If the numbers show value, great. If not, pick a
   different beachhead. Both outcomes move you forward. *)
DeployPilot ==
  /\ adoptionState = "ValueMeasured"
  /\ adoptionState' = "PilotDeployed"

(* The numbers show value. Expand to a controlled pilot with
   real users, real workflows, and real volume.

   WHAT TO DO: Define the pilot cohort (5-15 people), duration
   (4-8 weeks), success criteria (quantitative), and learning
   goals (qualitative). Assign one person as pilot lead to
   observe, document, and champion progress. Check in weekly. *)
IntegrateFeedback ==
  /\ adoptionState = "PilotDeployed"
  /\ adoptionState' = "FeedbackIntegrated"

(* The pilot generated something more valuable than data: the
   team now knows how the technology behaves when real people
   use it in real conditions.

   WHAT TO DO: Debrief every pilot participant individually.
   Three questions: (1) What did you stop doing? (2) What new
   opportunity appeared? (3) Would you go back to the old
   process? Compile into a pattern map. *)
RedesignProcess ==
  /\ adoptionState = "FeedbackIntegrated"
  /\ adoptionState' = "ProcessRedesigned"

(* This is the step that separates real transformation from
   surface-level change. The minimill didn't use an electric arc
   furnace inside a traditional integrated mill — it built a
   different kind of mill around the new capability.

   WHAT TO DO: Redesign the workflow from scratch with your team.
   Don't ask "how do we add technology to step 4?" Ask: "If we
   were building this today, knowing what we know, what would it
   look like?" Document it as a state machine. Compare to the
   original. The difference is the real value. *)
TrainTeam ==
  /\ adoptionState = "ProcessRedesigned"
  /\ adoptionState' = "TeamTrained"

(* Build internal expertise, not vendor dependency. Teams that
   understand the technology deeply enough to extend it, improve
   it, and teach others are the ones that thrive.

   Hidalgo's "infinite alphabet" in action: training grows the
   team's combinatorial capacity. Someone who understands the
   technology AND the domain AND the customer can recombine
   those into solutions nobody planned for.

   WHAT TO DO: Identify 2-3 people per team as internal
   champions. Give them at least 20% of their week to learn,
   experiment, and build. Create a shared knowledge base.
   Within 90 days, the team should be solving new problems
   with the technology on their own. *)
ApproveScale ==
  /\ adoptionState = "TeamTrained"
  /\ adoptionState' = "ScaleApproved"

(* Present the results — not as a pitch, but as a report from
   the people who did the work.

   WHAT TO DO: Three parts: (1) pilot results with real numbers,
   (2) redesigned process with projected impact at scale,
   (3) investment needed vs. opportunity cost of not expanding.
   The most compelling moment: what the pilot participants said
   when asked "would you go back?" *)
DeployToProduction ==
  /\ adoptionState = "ScaleApproved"
  /\ adoptionState' = "ProductionDeployed"

(* Go live. The technology is part of the production workflow.

   WHAT TO DO: Deploy incrementally — team by team, not all at
   once. Each team gets a two-week onboarding period with
   internal champions embedded. Monitor the same metrics you
   measured in the pilot. *)
IdentifyNextBeachhead ==
  /\ adoptionState = "ProductionDeployed"
  /\ adoptionState' = "NextBeachheadIdentified"

(* The minimill pattern: once rebar is mastered, move to angle
   iron. The capabilities built in the first beachhead make the
   second one faster. The confidence makes the third one routine.

   WHAT TO DO: Look at the next-most-valuable opportunity — the
   one that was too complex before but is now feasible because
   of what the team learned. Select it using the same criteria,
   with higher ambition. *)
CompoundCapability ==
  /\ adoptionState = "NextBeachheadIdentified"
  /\ adoptionState' = "CapabilityCompounding"

(* Multiple beachheads reinforcing each other. Insights from one
   feed innovation in another. The team trained for one use case
   can bootstrap the next in half the time.

   Hidalgo's core thesis: the more letters the organization knows,
   the more words it can spell. Three capabilities don't give you
   three options — they give you the combinations of three. The
   organizations that pull ahead aren't the ones with the biggest
   budgets; they're the ones with the richest alphabets.

   WHAT TO DO: Shift from project-by-project adoption to platform
   thinking. What shared infrastructure or capability layers can
   serve multiple teams? This is how minimills became Nucor —
   not by doing one thing well, but by building the culture that
   builds the capability. *)
ExpandUpmarket ==
  /\ adoptionState = "CapabilityCompounding"
  /\ adoptionState' = "NextBeachheadIdentified"

\* ================================================================
\* GROWTH EDGES — Where organizations learn the most
\* ================================================================

(* Excitement outruns understanding. Someone saw a demo or read
   an article and caught the vision. That energy is real — it
   means people care about the future. The job is to channel it
   into the disciplined process that turns aspiration into results.

   THE OPPORTUNITY: Christensen showed the most successful adopters
   pair vision with patience. The hype energy is the fuel; the
   beachhead strategy is the engine. *)
GetCapturedByHype ==
  /\ adoptionState = "SignalDetected"
  /\ adoptionState' = "HypeCaptured"

RecoverFromHype ==
  /\ adoptionState = "HypeCaptured"
  /\ adoptionState' = "LandscapeMapped"

(* The pilot works in its protected environment — the team built
   something genuinely valuable. The next step takes courage:
   connecting that value to the broader organization.

   THE OPPORTUNITY: The pilot team has firsthand stories and real
   data that no slide deck can match. Let them champion the
   expansion. *)
EnterPilotPurgatory ==
  /\ adoptionState = "PilotDeployed"
  /\ adoptionState' = "PilotPurgatory"

EscapePilotPurgatory ==
  /\ adoptionState = "PilotPurgatory"
  /\ adoptionState' = "FeedbackIntegrated"

(* A vendor made the first step easy, and the team got results
   fast. Useful. The growth edge: the deepest value comes from
   internalizing the capability — building your own team's
   understanding so they can extend and own what they started.

   THE OPPORTUNITY: Every vendor-assisted deployment is a learning
   opportunity. Study what the vendor built, gradually build
   internal expertise alongside it. Speed now, independence
   later. *)
GetVendorLocked ==
  /\ adoptionState = "PrototypeBuilt"
  /\ adoptionState' = "VendorLocked"

RecoverFromVendorLock ==
  /\ adoptionState = "VendorLocked"
  /\ adoptionState' = "TeamTrained"

(* Teams across the organization are already adopting on their
   own — which is strong evidence the technology creates real
   value. People don't adopt tools they find useless.

   THE OPPORTUNITY: The hardest part — proving it works — is
   already done. Governance doesn't have to slow things down;
   it can help teams learn from each other's discoveries. *)
AllowShadowAdoption ==
  /\ adoptionState = "LandscapeMapped"
  /\ adoptionState' = "ShadowAdoption"

GovernShadowAdoption ==
  /\ adoptionState = "ShadowAdoption"
  /\ adoptionState' = "ConstraintsIdentified"

(* The people in the existing support network have concerns —
   and those concerns come from real expertise. Middle managers
   understand coordination complexity. IT understands risk. Legal
   understands liability. These voices are institutional wisdom
   that makes change sustainable.

   THE OPPORTUNITY: The most durable transformations bring the
   existing experts along. Their knowledge of how the organization
   actually works is essential. Involve them early, listen, and
   resistance becomes guidance. *)
EncounterResistance ==
  /\ adoptionState = "ProcessRedesigned"
  /\ adoptionState' = "ResistanceEncountered"

OvercomeResistance ==
  /\ adoptionState = "ResistanceEncountered"
  /\ adoptionState' = "TeamTrained"

(* The organization created titles, committees, and quarterly
   reviews around innovation — which means leadership is
   invested. The job is to connect that investment to real
   workflows and real people.

   THE OPPORTUNITY: The infrastructure for change already exists.
   Redirect the energy from reporting into doing: pick one use
   case, measure one outcome, and let results generate the
   momentum. *)
PerformTransformationTheater ==
  /\ adoptionState = "ScaleApproved"
  /\ adoptionState' = "TransformationTheater"

AbandonTheater ==
  /\ adoptionState = "TransformationTheater"
  /\ adoptionState' = "BeachheadSelected"

(* Budget constraints are real. The learning and prototypes the
   team built aren't lost — the knowledge lives in the people.
   When resources return, the team that stayed curious will be
   ready to move faster than expected.

   THE OPPORTUNITY: Christensen's research shows constrained times
   are when the most creative solutions emerge. A smaller budget
   forces focus on the highest-value beachhead. *)
CutBudget ==
  /\ adoptionState = "PilotDeployed"
  /\ adoptionState' = "BudgetCut"

RestartAfterCut ==
  /\ adoptionState = "BudgetCut"
  /\ adoptionState' = "SignalDetected"

Init == adoptionState = "SignalDetected"

Next ==
  \/ MapTheLandscape
  \/ IdentifyConstraints
  \/ SelectBeachhead
  \/ BuildPrototype
  \/ MeasureValue
  \/ DeployPilot
  \/ IntegrateFeedback
  \/ RedesignProcess
  \/ TrainTeam
  \/ ApproveScale
  \/ DeployToProduction
  \/ IdentifyNextBeachhead
  \/ CompoundCapability
  \/ ExpandUpmarket
  \/ GetCapturedByHype
  \/ RecoverFromHype
  \/ EnterPilotPurgatory
  \/ EscapePilotPurgatory
  \/ GetVendorLocked
  \/ RecoverFromVendorLock
  \/ AllowShadowAdoption
  \/ GovernShadowAdoption
  \/ EncounterResistance
  \/ OvercomeResistance
  \/ PerformTransformationTheater
  \/ AbandonTheater
  \/ CutBudget
  \/ RestartAfterCut

====
