---- MODULE MeetingLifecycle ----

\* THE ART OF BRINGING MINDS TOGETHER
\*
\* "The most obvious, important realities are often the ones
\*  that are hardest to see and talk about." — David Foster Wallace
\*
\* When a meeting works, it multiplies what the people in the room
\* can do — shared understanding, clear decisions, real momentum.
\* When it doesn't work, everyone knows and nobody says so. This
\* state machine maps both paths.

VARIABLE meetingState

MeetingStages == {
  "NeedIdentified",
  "AgendaDrafted",
  "Scheduled",
  "Gathering",
  "InDiscussion",
  "DecisionReached",
  "ActionsAssigned",
  "FollowUpSent",
  "NoAgenda",
  "Derailed",
  "NoDecision",
  "ActionsForgotten",
  "MeetingAboutTheMeeting"
}

\* ================================================================
\* THE MEETING THAT MULTIPLIES — When every voice matters
\* ================================================================

(* Someone notices that a decision needs making, a problem needs
   more than one perspective, or a topic just won't resolve over
   email. Knowing when to call the meeting is half the battle. *)
DraftAgenda ==
  /\ meetingState = "NeedIdentified"
  /\ meetingState' = "AgendaDrafted"

(* An agenda tells participants what to prepare and what to
   expect. A meeting scheduled with context lets people arrive
   ready to contribute. *)
ScheduleWithContext ==
  /\ meetingState = "AgendaDrafted"
  /\ meetingState' = "Scheduled"

(* People arrive and settle in. A good facilitator uses these
   first moments to set the tone. The small talk isn't wasted —
   it's what makes honest discussion possible. *)
PeopleArrive ==
  /\ meetingState = "Scheduled"
  /\ meetingState' = "Gathering"

(* The facilitator does invisible work: drawing out quieter
   voices, keeping discussion focused, making sure dissent gets
   heard. Reading a room is a skill you can only learn by
   doing it. *)
BeginDiscussion ==
  /\ meetingState = "Gathering"
  /\ meetingState' = "InDiscussion"

(* The group reaches a decision — or honestly acknowledges they
   need to learn more before they can. Saying "we need more
   information, and here's who will get it" is a real outcome,
   not a deferral. *)
ReachDecision ==
  /\ meetingState = "InDiscussion"
  /\ meetingState' = "DecisionReached"

(* Each person leaves knowing what they own and by when. This is
   where a conversation becomes momentum. *)
AssignActions ==
  /\ meetingState = "DecisionReached"
  /\ meetingState' = "ActionsAssigned"

(* Good notes extend the meeting's value to people who weren't
   in the room. Send the follow-up within 24 hours, while the
   clarity is still fresh. *)
SendFollowUp ==
  /\ meetingState = "ActionsAssigned"
  /\ meetingState' = "FollowUpSent"

\* ================================================================
\* LEARNING EDGES — Where intentionality makes the difference
\* ================================================================

(* No agenda. Every team has been here. Recognizing the pattern
   is the first step toward fixing it. *)
SkipAgenda ==
  /\ meetingState = "NeedIdentified"
  /\ meetingState' = "NoAgenda"

(* The team shows up anyway. Next time, a few minutes of
   preparation will turn that goodwill into focused impact. *)
ScheduleAnyway ==
  /\ meetingState = "NoAgenda"
  /\ meetingState' = "Scheduled"

(* The conversation wanders. Sometimes it means someone has
   something important on their mind that isn't on the agenda.
   A good facilitator notes the tangent and steers back. *)
Derail ==
  /\ meetingState = "InDiscussion"
  /\ meetingState' = "Derailed"

(* The group catches itself and refocuses. Getting back on
   track is a team skill that improves with practice. *)
AttemptRecovery ==
  /\ meetingState = "Derailed"
  /\ meetingState' = "InDiscussion"

(* The discussion reveals the question is bigger than expected,
   or the right people aren't in the room. Acknowledging that
   beats forcing a premature decision. *)
EndWithoutDecision ==
  /\ meetingState = "InDiscussion"
  /\ meetingState' = "NoDecision"

(* A follow-up meeting, with better context and a sharper
   question. Each round teaches the team something about how
   they decide together. *)
ScheduleAnotherMeeting ==
  /\ meetingState = "NoDecision"
  /\ meetingState' = "MeetingAboutTheMeeting"

RecycleToNewMeeting ==
  /\ meetingState = "MeetingAboutTheMeeting"
  /\ meetingState' = "NeedIdentified"

(* Follow-up slipped — usually because the team is stretched,
   not because nobody cares. The simplest fix: one person who
   sends the recap. *)
ForgetActions ==
  /\ meetingState = "ActionsAssigned"
  /\ meetingState' = "ActionsForgotten"

RediscoverNeed ==
  /\ meetingState = "ActionsForgotten"
  /\ meetingState' = "NeedIdentified"

Init == meetingState = "NeedIdentified"

Next ==
  \/ DraftAgenda
  \/ ScheduleWithContext
  \/ PeopleArrive
  \/ BeginDiscussion
  \/ ReachDecision
  \/ AssignActions
  \/ SendFollowUp
  \/ SkipAgenda
  \/ ScheduleAnyway
  \/ Derail
  \/ AttemptRecovery
  \/ EndWithoutDecision
  \/ ScheduleAnotherMeeting
  \/ RecycleToNewMeeting
  \/ ForgetActions
  \/ RediscoverNeed

====
