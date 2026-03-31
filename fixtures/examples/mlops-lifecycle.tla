---- MODULE MLOpsLifecycle ----

\* BUILDING INTELLIGENCE RESPONSIBLY
\*
\* Machine learning is not magic — it's the product of decisions
\* at every stage. Someone frames the right question. An engineer
\* builds a pipeline the team can trust. A domain expert knows
\* what the numbers actually mean. The model is only as good as
\* the judgment guiding it.
\*
\* This state machine maps the craft of taking a model from idea
\* to production.

VARIABLE modelState

ModelStages == {
  "ProblemFramed",
  "DataCollected",
  "DataValidated",
  "DataPoisoned",
  "FeatureEngineered",
  "ModelTrained",
  "ModelEvaluated",
  "EvaluationFailed",
  "ModelRegistered",
  "StagingDeployed",
  "StagingTestFailed",
  "CanaryDeployed",
  "CanaryRolledBack",
  "ProductionDeployed",
  "Monitoring",
  "DriftDetected",
  "IncidentDeclared",
  "ModelRetired"
}

\* ================================================================
\* THE PATH TO PRODUCTION — Human wisdom at every stage
\* ================================================================

(* It starts with someone who understands the business well
   enough to ask: what prediction would change how we act?
   The problem frame is the most important decision in the
   entire lifecycle — and it's entirely a judgment call. *)
CollectData ==
  /\ modelState = "ProblemFramed"
  /\ modelState' = "DataCollected"

(* Schema checks, distribution analysis, null rates, freshness —
   guardrails built by engineers who know that clean data saves
   everyone downstream. Automated tools handle the volume;
   judgment defines what "clean" means for this problem. *)
ValidateData ==
  /\ modelState = "DataCollected"
  /\ modelState' = "DataValidated"

(* Feature engineering is where domain expertise becomes the
   model's edge. The data scientist who knows which signals
   matter — and which are noise — brings hard-won intuition
   to this step. Feature stores carry that work consistently
   from training into production. *)
EngineerFeatures ==
  /\ modelState = "DataValidated"
  /\ modelState' = "FeatureEngineered"

(* Every run records hyperparameters, data versions, and results.
   A colleague six months from now can see exactly what was tried
   and build from there. This is knowledge infrastructure — the
   investment that makes the next person's work faster. *)
TrainModel ==
  /\ modelState = "FeatureEngineered"
  /\ modelState' = "ModelTrained"

(* Statistical metrics tell part of the story; someone who
   understands the business tells the rest. Does this model
   serve all user groups fairly? Does it perform where it
   matters most? Good evaluation blends quantitative rigor
   with domain knowledge. *)
EvaluateModel ==
  /\ modelState = "ModelTrained"
  /\ modelState' = "ModelEvaluated"

(* The model registry is the handoff between data science and
   engineering. Version, lineage, and approval metadata say:
   this model was built with care and reviewed with rigor.
   Ready for the next team to carry forward. *)
RegisterModel ==
  /\ modelState = "ModelEvaluated"
  /\ modelState' = "ModelRegistered"

(* Integration tests for latency, schema correctness, and
   graceful degradation — designed by engineers who know that
   software in production is software others depend on. *)
DeployToStaging ==
  /\ modelState = "ModelRegistered"
  /\ modelState' = "StagingDeployed"

(* Route a small percentage of traffic to the new model and
   watch. The team observes real-world behavior before
   committing fully. Patience here protects users and builds
   team confidence. *)
DeployCanary ==
  /\ modelState = "StagingDeployed"
  /\ modelState' = "CanaryDeployed"

(* Full production rollout. The model is serving real people
   because a chain of skilled people guided it here. *)
PromoteToProduction ==
  /\ modelState = "CanaryDeployed"
  /\ modelState' = "ProductionDeployed"

(* The world changes, user behavior evolves, and the model needs
   people watching over it. Monitoring is the team's commitment
   to keeping the system worthy of the trust users place in it. *)
BeginMonitoring ==
  /\ modelState = "ProductionDeployed"
  /\ modelState' = "Monitoring"

\* ================================================================
\* LEARNING + RECOVERY — Where resilience builds trust
\* ================================================================

(* Validation rules caught a data quality issue before it
   reached modeling. That just saved the team days of wasted
   work — the safety net doing its job. *)
DataPoisonDetected ==
  /\ modelState = "DataCollected"
  /\ modelState' = "DataPoisoned"

RemediateData ==
  /\ modelState = "DataPoisoned"
  /\ modelState' = "DataCollected"

(* The model didn't meet the bar. Going back with better
   features, a different approach, or new hyperparameters
   is the scientific method at work. Each iteration deepens
   the team's understanding of the problem. *)
EvaluationFails ==
  /\ modelState = "ModelEvaluated"
  /\ modelState' = "EvaluationFailed"

RetrainAfterFailure ==
  /\ modelState = "EvaluationFailed"
  /\ modelState' = "FeatureEngineered"

(* Staging tests caught an integration issue before production.
   The test suite just earned its keep. *)
StagingTestFails ==
  /\ modelState = "StagingDeployed"
  /\ modelState' = "StagingTestFailed"

FixAndRedeploy ==
  /\ modelState = "StagingTestFailed"
  /\ modelState' = "ModelRegistered"

(* The canary detected degradation and rolled back. This is
   exactly what the team designed for: a small experiment that
   protected users while generating real learning about
   production behavior. *)
RollbackCanary ==
  /\ modelState = "CanaryDeployed"
  /\ modelState' = "CanaryRolledBack"

InvestigateCanaryFailure ==
  /\ modelState = "CanaryRolledBack"
  /\ modelState' = "ModelTrained"

(* The world changed and the monitoring caught it. Drift is
   natural in any model serving a living system. Detecting it
   early is a sign of operational maturity. *)
DetectDrift ==
  /\ modelState = "Monitoring"
  /\ modelState' = "DriftDetected"

RetrainOnNewData ==
  /\ modelState = "DriftDetected"
  /\ modelState' = "DataCollected"

(* Something serious. Fallback systems, clear escalation paths,
   and practiced incident response — built by people who planned
   for this. A well-handled incident builds more trust than one
   that never happened. *)
DeclareIncident ==
  /\ modelState = "Monitoring"
  /\ modelState' = "IncidentDeclared"

RecoverFromIncident ==
  /\ modelState = "IncidentDeclared"
  /\ modelState' = "ModelRegistered"

(* Every model has a lifecycle. Graceful retirement means the
   model served its purpose, the team learned from running it,
   and that knowledge carries forward. On to the next one. *)
RetireModel ==
  /\ modelState = "Monitoring"
  /\ modelState' = "ModelRetired"

Init == modelState = "ProblemFramed"

Next ==
  \/ CollectData
  \/ ValidateData
  \/ EngineerFeatures
  \/ TrainModel
  \/ EvaluateModel
  \/ RegisterModel
  \/ DeployToStaging
  \/ DeployCanary
  \/ PromoteToProduction
  \/ BeginMonitoring
  \/ DataPoisonDetected
  \/ RemediateData
  \/ EvaluationFails
  \/ RetrainAfterFailure
  \/ StagingTestFails
  \/ FixAndRedeploy
  \/ RollbackCanary
  \/ InvestigateCanaryFailure
  \/ DetectDrift
  \/ RetrainOnNewData
  \/ DeclareIncident
  \/ RecoverFromIncident
  \/ RetireModel

====
