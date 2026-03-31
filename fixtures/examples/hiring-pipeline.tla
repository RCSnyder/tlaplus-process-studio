---- MODULE HiringPipeline ----

\* FINDING REMARKABLE PEOPLE
\*
\* Hiring is one of the most consequential things an organization
\* does. Every great team was built one conversation at a time.
\*
\* Behind every offer is a chain of judgment calls: a recruiter
\* who sensed potential, an interviewer who asked the right
\* question, a hiring manager who went to bat for someone.
\* This state machine makes that process visible.

VARIABLE candidateState

CandidateStages == {
  "Applied",
  "Screened",
  "PhoneInterview",
  "TechnicalAssessment",
  "OnsiteInterview",
  "HiringCommittee",
  "OfferExtended",
  "OfferAccepted",
  "OfferDeclined",
  "BackgroundCheck",
  "Onboarding",
  "Ghosted",
  "Rejected",
  "Withdrawn",
  "ReqCancelled",
  "CounterOffered"
}

\* ================================================================
\* THE JOURNEY — Building trust one conversation at a time
\* ================================================================

(* Every resume is a real person who decided to try here.
   They deserve a genuine response. *)
ScreenApplication ==
  /\ candidateState = "Applied"
  /\ candidateState' = "Screened"

(* The first real conversation. A good recruiter listens for
   what drives this person and where their energy goes. In
   thirty minutes, both sides start figuring out whether
   this could work. *)
ConductPhoneScreen ==
  /\ candidateState = "Screened"
  /\ candidateState' = "PhoneInterview"

(* A window into how someone thinks, not just what they know.
   The best assessments — take-home, live session, portfolio
   review — create space for a candidate to show how they
   break down problems and navigate tradeoffs. *)
SendTechnicalAssessment ==
  /\ candidateState = "PhoneInterview"
  /\ candidateState' = "TechnicalAssessment"

(* Both sides are evaluating now. The candidate is learning as
   much as the team: do these people care about their work?
   Would I grow here? *)
BringOnsite ==
  /\ candidateState = "TechnicalAssessment"
  /\ candidateState' = "OnsiteInterview"

(* Every interviewer noticed something different. Structured
   scorecards turn those individual observations into a team
   decision — which is why the debrief matters more than any
   single interview. *)
ConveneHiringCommittee ==
  /\ candidateState = "OnsiteInterview"
  /\ candidateState' = "HiringCommittee"

(* The offer says: we think you'll make this team better.
   Compensation and title matter, but the message underneath
   is — we see what you can do. *)
ExtendOffer ==
  /\ candidateState = "HiringCommittee"
  /\ candidateState' = "OfferExtended"

(* The candidate chose this team over every other option.
   That commitment was earned by everyone who made the process
   feel human and genuine along the way. *)
AcceptOffer ==
  /\ candidateState = "OfferExtended"
  /\ candidateState' = "OfferAccepted"

(* A routine step that protects everyone involved — the team,
   the candidate, and the organization. Done thoroughly and
   respectfully, it is simply due diligence. *)
RunBackgroundCheck ==
  /\ candidateState = "OfferAccepted"
  /\ candidateState' = "BackgroundCheck"

(* Day one. The hiring process worked — now a different process
   begins: helping this person do the best work of their career. *)
StartOnboarding ==
  /\ candidateState = "BackgroundCheck"
  /\ candidateState' = "Onboarding"

\* ================================================================
\* GROWTH EDGES — Where the process teaches us
\* ================================================================

(* Silence costs more than a rejection. A person is waiting
   to hear back. Teams that close this gap build the kind
   of reputation that attracts better candidates next time. *)
GhostCandidate ==
  /\ candidateState = "Applied"
  /\ candidateState' = "Ghosted"

(* A clear, respectful "no" at screening frees the candidate
   to pursue the right fit. *)
RejectAtScreen ==
  /\ candidateState = "Screened"
  /\ candidateState' = "Rejected"

(* The candidate invested real time and effort. A few sentences
   about what stood out and what fell short can shape the
   trajectory of a career. Organizations that give feedback
   build goodwill that compounds. *)
RejectAfterTechnical ==
  /\ candidateState = "TechnicalAssessment"
  /\ candidateState' = "Rejected"

(* Someone spent a full day showing who they are. A thoughtful
   response earns trust — even from people who won't join
   this time. *)
RejectAfterOnsite ==
  /\ candidateState = "OnsiteInterview"
  /\ candidateState' = "Rejected"

(* The committee decides this is not the right match. When done
   with care and clear criteria, this protects both the team
   and the candidate from a fit that would not serve either. *)
CommitteeDeclines ==
  /\ candidateState = "HiringCommittee"
  /\ candidateState' = "Rejected"

(* When a candidate declines, it's worth asking: what gave them
   pause? Was the process too slow, the role unclear, the team
   dynamic off? Each decline is a mirror. *)
DeclineOffer ==
  /\ candidateState = "OfferExtended"
  /\ candidateState' = "OfferDeclined"

(* The organization saying: we want you enough to stretch.
   The willingness to negotiate is itself a signal. *)
MakeCounterOffer ==
  /\ candidateState = "OfferDeclined"
  /\ candidateState' = "CounterOffered"

ReconsiderAfterCounter ==
  /\ candidateState = "CounterOffered"
  /\ candidateState' = "OfferAccepted"

FinalDecline ==
  /\ candidateState = "CounterOffered"
  /\ candidateState' = "OfferDeclined"

(* Life shifted, another opportunity arrived, or the timing was
   wrong. Responding well keeps the door open — the best hires
   often come back around. *)
WithdrawDuringProcess ==
  /\ candidateState = "TechnicalAssessment"
  /\ candidateState' = "Withdrawn"

(* Budget changes, reorgs, or hiring freezes — beyond anyone's
   control. How the organization communicates this to candidates
   in the pipeline says a lot about its character. *)
CancelRequisition ==
  /\ candidateState = "HiringCommittee"
  /\ candidateState' = "ReqCancelled"

Init == candidateState = "Applied"

Next ==
  \/ ScreenApplication
  \/ ConductPhoneScreen
  \/ SendTechnicalAssessment
  \/ BringOnsite
  \/ ConveneHiringCommittee
  \/ ExtendOffer
  \/ AcceptOffer
  \/ RunBackgroundCheck
  \/ StartOnboarding
  \/ GhostCandidate
  \/ RejectAtScreen
  \/ RejectAfterTechnical
  \/ RejectAfterOnsite
  \/ CommitteeDeclines
  \/ DeclineOffer
  \/ MakeCounterOffer
  \/ ReconsiderAfterCounter
  \/ FinalDecline
  \/ WithdrawDuringProcess
  \/ CancelRequisition

====
