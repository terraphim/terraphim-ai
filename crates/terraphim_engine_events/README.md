# terraphim_engine_events

Shared `EngineEvent` vocabulary for Terraphim engine surfaces. This crate is
the Desktop P1 freeze of the shared TACP/`EngineEvent` contract: the Zed and
VS Code consumers (approved R1 alignment) pin their compatibility sets
against these types and the golden serialisation vectors in
`tests/golden/`.

It currently carries the evolution lifecycle family (`evo.*`) from the
Terraphim Agent Communication Protocol specification, section 5.1
(`terraphim/agent-communication-protocol`, issue #28, commit dc31489):

| Wire name     | Variant             |
|---------------|---------------------|
| `evo.propose` | `EvolutionProposed` |
| `evo.approve` | `EvolutionApproved` |
| `evo.reject`  | `EvolutionRejected` |
| `evo.applied` | `EvolutionApplied`  |

Serialisation convention: internally tagged on `type` with the exact dotted
TACP message-type name, snake_case payload fields, and
`terraphim_types::shared_learning::TrustLevel` re-used (not redeclared) for
`trust_level`.

Normative constraint 3 of spec 5.1 (`evo.applied` MUST NOT be emitted before
a matching `evo.approve`) is enforced in the type system:
`EvolutionApplied` has private fields and its only constructor,
`EvolutionApplied::from_approval`, requires an `EvolutionApprove` reference.

The `terraphim_app_server` crate is expected to adopt this vocabulary as the
seed of its full `EngineEvent` enum; see Gitea issue
terraphim/terraphim-ai#3232.
