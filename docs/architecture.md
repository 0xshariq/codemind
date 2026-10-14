# CodeMind Architecture

> **Hackathon scope only**
>
> This document is the implementation architecture for the CodeMind
> submission to Graph Hacks: Context for AI Agents, powered by FalkorDB.
>
> It intentionally describes the **frozen hackathon architecture**, not
> the complete post-hackathon CodeMind vision. Long-term capabilities
> that were discussed but explicitly deferred are called out throughout
> the document.

------------------------------------------------------------------------

## 1. Introduction

### 1.1 What is CodeMind?

**CodeMind is an AI codebase investigation agent that uses FalkorDB to
reason across code relationships, explain the path behind its
conclusions, and remember validated investigation context across
sessions.**

CodeMind is not positioned as a generic coding assistant.

Its central job is to answer questions such as:

> **"What could this change affect?"**

For example:

> "What happens in my Nizaam Core codebase when I change the control
> plane?"

The important distinction is that CodeMind does not ask the LLM to guess
the answer from a few files. It gives the agent a persistent, queryable
representation of the codebase and its testing relationships.

The core reasoning path is:

``` text
User question
     ↓
LLM understands the investigation request
     ↓
Semantic JavaScript tools
     ↓
Rust CodeMind Engine
     ↓
CoIE → canonical code facts
     ↓
TIE → test intelligence
     ↓
ChIE → change intelligence
     ↓
FalkorDB-backed graph evidence
     ↓
Structured result
     ↓
LLM explanation
     ↓
Chat-first UI
```

The graph is therefore part of the reasoning system, not a visualization
placed beside an otherwise ordinary chatbot.

### 1.2 What makes CodeMind different?

The hackathon version is built around three ideas:

1.  **Graph-grounded investigation**
    -   The agent follows real code relationships.
    -   Multi-hop traversal is bounded and evidence-backed.
    -   Important conclusions can show the path that produced them.
2.  **Deterministic engineering intelligence**
    -   Rust owns canonical code, test and change intelligence.
    -   The LLM orchestrates and explains.
    -   The LLM does not manufacture graph facts.
3.  **Persistent engineering memory**
    -   Review episodes, findings, qualified semantic/procedural
        memories and handoffs can persist.
    -   A later session can retrieve prior context.
    -   Retrieved memory is checked against current graph evidence
        before it is trusted.

------------------------------------------------------------------------

# 2. The Problem CodeMind Solves

Traditional AI coding assistants are good at reading local code and
generating or explaining code. They become less reliable when the
question is fundamentally about **relationships across a codebase**.

A developer asking:

> "If I change this function, what could break?"

may need to understand:

``` text
changed symbol
   ↓
callers
   ↓
dependent modules
   ↓
other symbols
   ↓
tests
   ↓
historical evidence
   ↓
risk and validation requirements
```

This is not just a text-generation problem.

The answer depends on connected facts:

-   Which symbols call the changed symbol?
-   Which files import the affected module?
-   Which tests target or exercise affected code?
-   Which historical changes touched the same area?
-   Which relationships are unresolved?
-   Which evidence belongs to the same repository snapshot?
-   Which conclusions are derived from the available evidence?

CodeMind makes these relationships queryable through FalkorDB and gives
the LLM controlled semantic access to them.

### 2.1 The core product question

The hero question for the hackathon is:

> **"What could this change affect?"**

The answer should contain:

-   impact
-   risk
-   test impact
-   evidence
-   graph paths
-   limitations
-   optionally relevant memory

Impact, risk, confidence and review severity remain separate concepts.

------------------------------------------------------------------------

# 3. Purpose of the Hackathon Architecture

The hackathon implementation has two primary goals.

### Goal 1: Track 01

Demonstrate that an agent can:

1.  understand a change investigation request,
2.  use semantic tools,
3.  query connected code/test data,
4.  perform bounded multi-hop reasoning,
5.  derive impact and risk,
6.  recommend relevant tests,
7.  explain the result using graph evidence.

### Goal 2: Track 02

Demonstrate that the agent can:

1.  persist a review episode,
2.  persist useful findings,
3.  create at least one qualified reusable memory,
4.  persist a structured handoff,
5.  retrieve memory in a fresh session,
6.  validate old memory against current evidence,
7.  use the validated memory in a later investigation.

### Track submission decision

The intended submission targets **Track 01 — Agents That Act on Connected Data** and **Track 02 — Agent Memory and Coordination**. Do **not** target Track 03. The product story should stay focused on graph-grounded code investigation and persistent engineering context rather than expanding into a company-knowledge product.

The implementation deliberately does **not** attempt to build the full
long-term CodeMind product.

------------------------------------------------------------------------

# 4. Hackathon Scope Boundary

## 4.1 What must exist

  ------------------------------------------------------------------------
  Area                                Hackathon requirement
  ----------------------------------- ------------------------------------
  Code intelligence                   Repository/snapshot, directories,
                                      files, symbols, supported
                                      modules/packages,
                                      containment/definition/import/call
                                      relationships and diagnostics

  History                             Snapshot/source-revision provenance
                                      sufficient to identify analyzed
                                      state

  Test intelligence                   Test declarations, relevant suites
                                      and supported test-to-code mappings

  Change intelligence                 ChangeSet, ChangeRecord,
                                      AnalysisRun, ImpactAnalysis,
                                      RiskAssessment and Evidence, with
                                      `AFFECTS` and `RECOMMENDS` where
                                      supported

  Memory                              Review episodes, findings, one
                                      qualified semantic memory, one
                                      reusable procedure, persisted
                                      handoff and cross-session retrieval

  Languages                           Rust, JavaScript/TypeScript and
                                      Python

  Graph DB                            FalkorDB as the primary graph
                                      database

  Agent                               LLM-driven semantic tool
                                      orchestration

  UI                                  AI-agent/chat-first experience with
                                      structured engineering components

  Progress                            Live Rust engine progress through
                                      gRPC → Agent Runtime → SSE

  Demo                                Real graph reasoning plus persistent
                                      memory/coordination
  ------------------------------------------------------------------------

## 4.2 Explicitly deferred

The following remain long-term architecture, not hackathon requirements:

-   full Git diff/PR ingestion
-   advanced hypothetical change simulation
-   sophisticated incremental/historical graph handling
-   rich commit/developer/PR/issue intelligence
-   runtime test execution by CodeMind
-   broad runtime coverage ingestion
-   autonomous agent swarms
-   vector-based memory retrieval
-   automatic confidence-based memory promotion
-   complex memory consolidation
-   elaborate long-term analytics
-   Kubernetes/distributed workers
-   Redis/Kafka/Temporal-style orchestration
-   separate CoIE/TIE/ChIE microservices
-   full CodeRabbit/Greptile-style review workspace
-   MCP as a core dependency

The architecture remains extensible, but these capabilities must not
expand the hackathon implementation beyond the frozen MVP.

------------------------------------------------------------------------

# 5. High-Level Architecture

## 5.1 Canonical architecture

``` text
                         ┌─────────────────────────────┐
                         │          Browser            │
                         │   CodeMind AI Agent UI      │
                         └──────────────┬──────────────┘
                                        │
                                  HTTP / SSE
                                        │
                                        ▼
                         ┌─────────────────────────────┐
                         │          Next.js            │
                         │   UI + Route Handlers       │
                         │                             │
                         │   Application Boundary      │
                         └──────────────┬──────────────┘
                                        │
                                        ▼
                         ┌─────────────────────────────┐
                         │       Agent Runtime         │
                         │                             │
                         │ LLM orchestration           │
                         │ Semantic Tool Registry      │
                         │ Workflows                   │
                         │ Memory policy               │
                         │ Handoffs                    │
                         │ Operation coordination      │
                         └──────────────┬──────────────┘
                                        │
                                  gRPC / Protobuf
                                        │
                                        ▼
                         ┌─────────────────────────────┐
                         │      Rust CodeMind         │
                         │          Engine             │
                         │                             │
                         │  ┌─────┐ ┌─────┐ ┌─────┐  │
                         │  │CoIE │ │ TIE │ │ChIE │  │
                         │  └─────┘ └─────┘ └─────┘  │
                         │                             │
                         │ deterministic intelligence  │
                         │ source processing            │
                         │ graph operations             │
                         │ memory persistence boundary │
                         └──────────────┬──────────────┘
                                        │
                               Graph Storage Port
                                        │
                                        ▼
                         ┌─────────────────────────────┐
                         │          FalkorDB           │
                         │                             │
                         │  Code / History             │
                         │  Test                       │
                         │  Change                     │
                         │  Memory                     │
                         │  Operation-related graph    │
                         └─────────────────────────────┘
```

## 5.2 Dependency direction

The architecture has one-way dependency flow:

``` text
UI
 ↓
Route Handler
 ↓
Agent Runtime
 ↓
gRPC / Protobuf
 ↓
Rust Engine
 ↓
Graph Storage Port
 ↓
FalkorDB
```

The following are explicitly forbidden:

``` text
UI → Rust directly
UI → FalkorDB
LLM → FalkorDB
LLM → arbitrary Cypher
LLM → private Rust functions
Next.js → FalkorDB directly
React → database
```

## 5.3 Responsibility matrix

  -----------------------------------------------------------------------
  Component               Owns                    Does not own
  ----------------------- ----------------------- -----------------------
  Next.js UI              presentation and        intelligence
                          interaction             

  Route Handlers          HTTP/API boundary       graph traversal and
                                                  orchestration

  Agent Runtime           LLM orchestration,      canonical graph truth
                          tools, memory policy,   
                          handoffs                

  Rust Engine             CoIE/TIE/ChIE           UI
                          deterministic           
                          intelligence            

  Graph Storage Port      database abstraction    application policy

  FalkorDB adapter        actual graph            domain decisions
                          persistence             

  Protobuf                transport contract      domain ownership

  Fixtures                deterministic demo/test production graph
                          data                    

  Tests                   correctness             runtime logic
                          verification            
  -----------------------------------------------------------------------

------------------------------------------------------------------------

# 6. Application and Runtime Architecture

## 6.1 Next.js is the application boundary

Next.js owns:

-   the chat-first UI,
-   repository setup UI,
-   investigation workspace,
-   Route Handlers,
-   authentication/authorization boundary,
-   application API,
-   server-side Agent Runtime,
-   Rust gRPC client,
-   UI-safe types,
-   SSE delivery to the browser.

There is **no Express backend**.

There is no separate `apps/api` service.

## 6.2 Route Handlers are thin

Route Handlers perform:

1.  request parsing,
2.  authentication,
3.  authorization,
4.  input validation,
5.  invocation of application/runtime code,
6.  response or stream creation.

They do not contain:

-   graph traversal,
-   Cypher,
-   ChIE algorithms,
-   memory policy,
-   LLM orchestration,
-   direct FalkorDB access.

## 6.3 Agent Runtime modules

The server-side runtime is modular:

``` text
apps/web/src/lib/server/
├── agent/
├── tools/
├── workflows/
├── memory/
├── handoffs/
├── operations/
├── results/
├── rust-client/
├── auth/
└── config/
```

### `agent/`

Owns LLM orchestration and the reasoning loop.

### `tools/`

Owns:

-   semantic tool registry,
-   tool definitions,
-   schemas,
-   validation,
-   authorization context,
-   execution,
-   result validation,
-   guardrails.

### `workflows/`

Owns higher-level application workflows such as:

-   repository preparation,
-   focused investigation,
-   change analysis,
-   evidence follow-up.

### `memory/`

Owns memory policy:

-   retrieval decisions,
-   candidate proposals,
-   validation,
-   promotion policy,
-   current-evidence checking.

Rust owns memory persistence.

### `handoffs/`

Owns:

-   creation,
-   claim,
-   consumption,
-   idempotency,
-   recovery.

### `operations/`

Owns application-level operation coordination and recoverable state.

### `results/`

Separates:

``` text
canonical result
      ├── LLM projection
      └── UI projection
```

### `rust-client/`

The only application-side boundary used to communicate with the Rust
service.

### `auth/`

Application authorization and scope handling.

### `config/`

Validated environment/configuration loading.

------------------------------------------------------------------------

# 7. Protobuf and gRPC Boundary

Protobuf is the transport contract between the Agent Runtime and Rust.

``` text
proto/codemind/v1/
```

is the source of truth.

Generated Rust and TypeScript bindings are build artifacts.

``` text
.proto
  ↓
code generation
  ↓
Rust bindings + TypeScript bindings
```

They are not manually maintained as separate contracts.

The exact `.proto` message and RPC names are implementation details,
but the architecture requires:

-   typed requests,
-   typed responses,
-   operation identity,
-   repository/snapshot context,
-   result context,
-   progress events,
-   diagnostics,
-   completeness and freshness semantics,
-   stable correlation metadata.

For the canonical data-result payload, Rust serializes the canonical
structured result as JSON and places that JSON text in the agreed string
field of the typed Protobuf response. Protobuf remains the transport
contract; it does not transport TOON as the canonical result. The
JavaScript Agent Runtime parses and validates the JSON after receiving the
RPC response. Flattening and TOON encoding happen later, only on the
LLM-facing branch. Do not move either transformation into Rust, Protobuf,
the graph-storage adapter, or the browser.

------------------------------------------------------------------------

# 8. The Rust CodeMind Engine

CoIE, TIE and ChIE are **independent logical engines inside one Rust
service**.

They are not separate microservices.

``` text
Rust CodeMind Engine
│
├── CoIE
│
├── TIE
│
├── ChIE
│
├── source/
├── contract/
├── transport/
├── memory/
├── graph_storage/
└── lifecycle/
```

The three engines have distinct ownership:

  Engine   Owns                         Main purpose
  -------- ---------------------------- ---------------------------------------------
  CoIE     Code Graph + History Graph   canonical code/repository facts
  TIE      Test Graph                   test facts and testing evidence
  ChIE     Change Graph                 change-derived impact/risk/test conclusions

Ownership does not mean exclusive read access.

ChIE may read CoIE and TIE information when required. TIE may reference
CoIE entities. The rule is that consumers must preserve the owning
engine's semantics and must not overwrite or silently reinterpret
another engine's canonical facts.

------------------------------------------------------------------------

# 9. CoIE --- Code Intelligence Engine

## 9.1 Purpose

CoIE answers:

> **What exists in this repository, what is its structure, how are its
> code entities connected, and what repository-development facts can be
> established?**

CoIE is the authoritative source for:

-   repository structure,
-   source inventory,
-   symbols,
-   modules/packages,
-   code relationships,
-   source provenance,
-   snapshot context,
-   supported historical repository facts,
-   structural semantic queries.

The LLM never invents missing relationships.

## 9.2 CoIE graph architecture

CoIE has two logical graph domains:

``` text
                    CoIE
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
     Code Graph            History Graph
          │                     │
          │                     │
      current code        repository history
      structure            and provenance
```

The hackathon primarily requires the Code Graph and enough
snapshot/source-revision provenance to identify the analyzed state.

Rich commit/PR/issue/developer intelligence is deferred.

------------------------------------------------------------------------

## 9.3 CoIE Code Graph node model

The universal node catalog is:

``` text
Repository
RepositorySnapshot
Directory
File
Module
Package
Symbol
```

### Repository

Identifies the repository independently of individual files and
revisions.

### RepositorySnapshot

Identifies the source state being analyzed.

This is critical because CodeMind must never ambiguously mix facts from
different source states.

### Directory

Represents physical filesystem organization.

### File

Represents a repository file and its metadata.

### Module

Represents logical code organization.

### Package

Represents language/project package structure.

### Symbol

Represents a declaration or selected fine-grained code entity.

The graph does not create separate labels such as:

``` text
RustFunction
JavaClass
TypeScriptInterface
```

Instead, `Symbol` uses normalized and native kind information.

Conceptually:

``` text
Symbol
├── kind = contract
├── language = typescript
└── nativeKind = typescript::interface
```

------------------------------------------------------------------------

## 9.4 Symbol granularity

The hackathon uses declaration-first modeling.

### Primary entities

-   functions,
-   methods,
-   structs,
-   enums,
-   traits,
-   interfaces,
-   classes,
-   constants,
-   macros,
-   supported type declarations.

### Selected nested entities

Where useful:

-   parameters,
-   fields,
-   properties,
-   enum variants,
-   selected variables.

### Not default graph entities

The MVP does not create nodes for every:

-   expression,
-   statement,
-   branch,
-   token.

The graph must remain useful for investigation rather than becoming a
syntax dump.

------------------------------------------------------------------------

# 10. CoIE Graph Relationships

The core relationship vocabulary is language-agnostic.

## 10.1 Structural relationships

``` text
CONTAINS
DEFINES
IMPORTS
EXPORTS
ALIASES
```

### `CONTAINS`

Represents structural containment.

Examples:

``` text
Repository → RepositorySnapshot
RepositorySnapshot → Directory
Directory → File
File → Symbol
```

### `DEFINES`

Represents a declaration relationship between a file/module/package and
a symbol.

### `IMPORTS`

Represents an established import relationship.

### `EXPORTS`

Represents an established export relationship.

### `ALIASES`

Represents supported aliasing relationships.

------------------------------------------------------------------------

## 10.2 Behavioral and dependency relationships

``` text
CALLS
REFERENCES
USES_TYPE
IMPLEMENTS
INHERITS_FROM
OVERRIDES
```

These relationships are only emitted when the adapter/resolution
pipeline can establish their semantics.

### Critical rule: do not invent relationships

If:

``` text
service.process()
```

cannot be reliably resolved to a canonical symbol, CoIE must preserve
the uncertainty.

It must not select a similarly named function merely to make the graph
appear complete.

Unresolved relationships remain diagnostics/evidence, not confirmed
graph facts.

------------------------------------------------------------------------

# 11. CoIE Identity and Snapshot Semantics

The architecture distinguishes:

``` text
Repository identity
       ↓
Source snapshot
       ↓
Graph version
       ↓
Entity identity
       ↓
Source content / processing provenance
```

These are not interchangeable.

A symbol's logical identity and its representation in a specific
snapshot are separate concepts.

The exact implementation identity formula is an implementation detail,
but the hackathon requires:

-   deterministic identity,
-   snapshot awareness,
-   stable references,
-   correct overload distinction,
-   no silent mixing of source states.

------------------------------------------------------------------------

# 12. CoIE Language Support

The hackathon proves polyglot operation using:

``` text
Rust
JavaScript
TypeScript
Python
```

Tree-sitter is the initial parsing foundation.

  Language     Parser                         Extensions
  ------------ ------------------------------ ---------------
  Rust         Tree-sitter Rust               `.rs`
  JavaScript   Tree-sitter JavaScript         `.js`, `.jsx`
  TypeScript   Tree-sitter TypeScript / TSX   `.ts`, `.tsx`
  Python       Tree-sitter Python             `.py`

Tree-sitter provides syntax trees and source locations, but it is not
treated as proof of perfect semantic resolution.

Language adapters remain responsible for language-specific extraction
and supported relationship resolution.

------------------------------------------------------------------------

# 13. CoIE Language Adapter Pipeline

The source-processing pipeline is:

``` text
Repository Provider
      ↓
Immutable Repository Snapshot
      ↓
File inventory + language detection
      ↓
Language adapter
      ↓
Extraction candidates
      ↓
Canonicalization / validation
      ↓
Identity resolution
      ↓
Relationship validation
      ↓
CoIE graph writer
      ↓
Published Code Graph
```

An adapter receives:

-   immutable source bytes,
-   repository-relative path,
-   repository/snapshot context,
-   project configuration where applicable,
-   parser/grammar/adapter versions,
-   declared capabilities,
-   processing limits.

An adapter returns:

-   entity candidates,
-   relationship candidates,
-   native kinds,
-   normalized kind candidates,
-   source ranges,
-   evidence,
-   diagnostics,
-   unsupported/partial status,
-   provenance.

Adapters do not write directly to FalkorDB.

------------------------------------------------------------------------

# 14. CoIE Indexing and Graph Publication

The architecture supports:

-   full indexing,
-   incremental extraction,
-   stale-fact cleanup,
-   idempotent updates,
-   snapshot-aware graph versions,
-   atomic publication,
-   explicit graph quality.

For the hackathon, the important guarantees are:

1.  repeated ingestion must be safe,
2.  stale relationships must not remain falsely current,
3.  published graph state must be tied to a known snapshot,
4.  incomplete processing must be visible,
5.  failed unpublished work must not silently replace the last valid
    graph.

A supported-file parse failure may produce a degraded graph when the
remaining graph is still internally consistent.

The rule is:

> A degraded graph may be incomplete, but it must not knowingly present
> stale structural facts as current truth.

------------------------------------------------------------------------

# 15. CoIE Semantic Query Boundary

Other components do not query FalkorDB directly.

CoIE exposes semantic query concepts such as:

``` text
repository/snapshot context
symbol lookup
callers
callees
references
imports
implementations
inheritance
dependencies
dependents
history
evidence
diagnostics
```

Derived dependency views such as:

``` text
get_dependencies(symbol)
get_dependents(symbol)
```

are query concepts.

The MVP does not create a primitive `DEPENDS_ON` edge just to make these
queries easier.

## 15.1 Bounded traversal

CoIE owns query limits such as:

-   maximum results,
-   maximum traversal depth,
-   timeout,
-   pagination,
-   explored-node/work limits.

A truncated traversal must say that it was truncated.

A timeout must not look like an exhaustive answer.

Pagination remains bound to the original query and graph version.

## 15.2 Evidence

Important CoIE facts must be traceable to:

-   source snapshot,
-   file,
-   source range,
-   entity/relationship,
-   parser/adapter provenance,
-   resolution status.

------------------------------------------------------------------------

# 16. TIE --- Test Intelligence Engine

## 16.1 Purpose

TIE answers:

> **How is this repository's code tested and validated, and what test
> evidence can be established?**

TIE owns the Test Graph.

It references CoIE's canonical code entities but does not create
competing code identities.

ChIE consumes TIE facts and decides which tests are relevant to a
particular change.

------------------------------------------------------------------------

# 17. TIE Test Graph

The broader logical TIE catalog includes:

``` text
Test
TestSuite
Fixture
TestCase
TestRun
TestExecution
```

The hackathon uses the subset required for static test discovery and
evidence-backed test impact.

The important distinction is between:

-   test declaration,
-   intended target,
-   actual runtime exercise,
-   coverage evidence,
-   execution outcome.

These meanings must not be collapsed.

------------------------------------------------------------------------

# 18. TIE Relationships

Important relationship semantics include:

``` text
DECLARED_BY
TARGETS
EXERCISES
COVERS
USES_FIXTURE
DEPENDS_ON
BELONGS_TO
```

### `DECLARED_BY`

Connects a test to its declaring CoIE code entity.

### `TARGETS`

Represents an established or supported inferred association between a
test and code.

### `EXERCISES`

Represents an attributable runtime observation that execution reached
code.

### `COVERS`

Represents a coverage report association at its actual reported scope
and granularity.

### Critical distinction

These are not interchangeable.

For example:

``` text
CALLS
```

in CoIE does not mean:

``` text
TEST EXECUTED
```

and:

``` text
TARGETS
```

does not prove:

``` text
COVERS
```

Missing test mappings do not prove that no tests exist.

------------------------------------------------------------------------

# 19. TIE Hackathon Discovery Scope

The initial language/framework support is:

  Language     Frameworks
  ------------ --------------------------
  Rust         native Rust test harness
  JavaScript   Jest, Vitest
  TypeScript   Jest, Vitest
  Python       pytest, unittest

The MVP performs **static discovery**.

It does not execute arbitrary repository code merely to discover tests.

This protects the system from untrusted repository execution and keeps
the hackathon implementation bounded.

TIE may later ingest external test-runner results, but those results
remain clearly identified as runtime observations.

------------------------------------------------------------------------

# 20. TIE Test-to-Code Evidence

For the hackathon, the important path is:

``` text
Test
  ↓ DECLARED_BY
CoIE Symbol

Test
  ↓ TARGETS
CoIE Symbol
```

Where additional evidence exists, the architecture preserves:

``` text
Test
  ↓ EXERCISES
CoIE Symbol

Test
  ↓ COVERS
CoIE Symbol
```

ChIE uses these meanings without changing them.

------------------------------------------------------------------------

# 21. TIE Snapshot Compatibility

TIE must never silently combine:

``` text
Test Graph from snapshot A
+
Code Graph from snapshot B
```

For every cross-engine mapping, TIE needs compatible source context.

If a source snapshot is unknown:

``` text
unknown snapshot
```

must remain unknown.

It must not silently substitute the current snapshot.

If CoIE is degraded or a code identity cannot be resolved, TIE preserves
that limitation rather than manufacturing a confirmed mapping.

------------------------------------------------------------------------

# 22. TIE Execution Boundary

For the hackathon:

> **TIE does not orchestrate arbitrary test execution.**

It discovers test structure and can consume external evidence when
available.

The MVP does not need to execute the repository's tests.

This is important because running untrusted repository code introduces a
separate execution sandbox/security problem that is outside the
hackathon scope.

------------------------------------------------------------------------

# 23. ChIE --- Change Intelligence Engine

## 23.1 Purpose

ChIE is where CodeMind turns code and testing evidence into
change-specific intelligence.

ChIE answers:

> **What could be affected by the proposed change, what tests are
> relevant, what risks and evidence gaps exist, and why?**

ChIE does not own canonical code or test facts.

It consumes:

``` text
CoIE facts
+
TIE facts
+
relevant historical evidence
+
proposed change context
```

and produces:

``` text
impact
+
risk
+
test impact
+
evidence
+
limitations
```

------------------------------------------------------------------------

# 24. ChIE Hackathon Analysis Model

The MVP is **symbol-first** with file fallback.

A user selects:

``` text
Symbol
```

or, if symbol resolution is unavailable:

``` text
File
```

and asks a prospective question such as:

> "What could this change affect?"

The exact source snapshot is pinned.

The MVP does not require a real Git diff or PR.

A selected symbol means:

> **investigate the consequences of a proposed change to this entity**

not:

> **a real repository diff has already occurred**

------------------------------------------------------------------------

# 25. ChIE Processing Pipeline

``` text
Selected symbol/file
        ↓
Pin repository + snapshot context
        ↓
Resolve canonical CoIE entity
        ↓
Traverse eligible code relationships
        ↓
Determine direct + indirect impact
        ↓
Use TIE test evidence
        ↓
Evaluate risk factors
        ↓
Build Change Graph
        ↓
Publish completion event
        ↓
Agent Runtime retrieves semantic results
        ↓
LLM explains
```

------------------------------------------------------------------------

# 26. ChIE Impact Semantics

The MVP distinguishes:

  -----------------------------------------------------------------------
  Category                            Meaning
  ----------------------------------- -----------------------------------
  Changed entity                      selected proposed change target

  Direct impact                       reached through one eligible
                                      propagation step

  Indirect impact                     reached through two or more
                                      eligible propagation steps

  Supporting path                     graph relationships explaining the
                                      impact
  -----------------------------------------------------------------------

The origin entity is not automatically counted as downstream impact.

## 26.1 Initial traversal relationships

ChIE prioritizes:

``` text
CALLS
IMPORTS
```

and can use other validated CoIE relationships when their semantics are
established.

Example:

``` text
Changed Symbol
     ↓ CALLS
Function A
     ↓ IMPORTS
Module B
```

The path is preserved as evidence.

The original relationship meaning and direction remain recoverable.

------------------------------------------------------------------------

# 27. ChIE Impact Result

An `ImpactAnalysis` contains, conceptually:

-   directly affected entities,
-   indirectly affected entities,
-   file counts,
-   symbol counts,
-   module counts,
-   supporting paths,
-   unresolved relationships,
-   incomplete upstream inputs,
-   traversal truncation,
-   exact graph context.

`AFFECTS` belongs to ChIE.

It means:

> potential consequence of the proposed change.

It does **not** mean:

-   guaranteed defect,
-   proven failure,
-   runtime execution,
-   certain breakage.

------------------------------------------------------------------------

# 28. ChIE Test Impact

ChIE decides which tests are relevant.

TIE owns the test facts.

ChIE combines:

``` text
changed entity tests
+
tests associated with affected entities
+
relevant test infrastructure
+
historical/test evidence
```

The recommendation retains its reason.

Example:

``` text
Changed symbol
      ↓
Affected function
      ↓
TIE test association
      ↓
Recommended test
```

The same test must be deduplicated in the final recommendation list.

`RECOMMENDS` points to the actual TIE-owned `Test` entity.

------------------------------------------------------------------------

# 29. ChIE Risk Model

The hackathon does not use an arbitrary calibrated 0--100 risk score.

Risk uses five independent factor families:

1. structural exposure,
2. validation evidence,
3. historical instability when supported,
4. explicit component criticality / sensitive-boundary policy, and
5. supported change characteristics.

These families are deterministic inputs to a qualitative assessment. The MVP
must not invent an arbitrary composite numeric risk score.

## 29.1 Structural exposure

Includes:

-   blast radius,
-   affected entities,
-   propagation depth,
-   centrality where supported,
-   critical supported paths.

High exposure does not automatically mean high risk.

## 29.2 Validation evidence

Includes:

-   test associations,
-   compatible execution evidence where available,
-   coverage observations where available,
-   validation gaps.

## 29.3 Historical instability

Includes:

-   relevant failures,
-   regressions,
-   instability associated with affected areas.

Repeated reports must not automatically be treated as independent
failures.

## 29.4 Component criticality and sensitive boundaries

Uses explicit, trustworthy metadata or configured project policy.

The LLM's intuition or a component's name is not sufficient.

## 29.5 Change characteristics

Uses supported information about:

-   change type,
-   semantic scope,
-   change size where known.

A small change can still be risky when it touches a sensitive boundary.

------------------------------------------------------------------------

# 30. ChIE Output Semantics

Four dimensions remain separate:

``` text
Impact
  What could be affected?

Risk
  How concerning is the change?

Confidence
  How complete/reliable is the supporting evidence?

Review Severity
  How urgently should a human review it?
```

Examples:

``` text
High impact + strong validation evidence
→ potentially moderate/low risk

Small impact + sensitive security boundary
→ potentially high risk

High risk + low confidence
→ still high risk, but evidence is incomplete
```

Low confidence must never silently lower risk.

The UI should therefore avoid false precision such as:

``` text
Risk = 83.72%
```

unless a validated quantitative model is explicitly introduced later.

------------------------------------------------------------------------

# 31. ChIE Evidence and Limitations

A conclusion must be explainable through evidence.

A result can contain:

``` text
Finding
  ↓
Affected entity
  ↓
Relationship path
  ↓
Test evidence
  ↓
Historical evidence
  ↓
Risk factors
  ↓
Limitations
```

Unknown evidence is never converted to:

``` text
safe
zero
no impact
no tests
```

Examples:

``` text
No matching test mapping
```

is not the same as:

``` text
No relevant test exists
```

Likewise:

``` text
truncated traversal
```

is not:

``` text
complete analysis
```

------------------------------------------------------------------------

# 32. ChIE Change Graph

The hackathon Change Graph uses these core concepts:

``` text
ChangeSet
ChangeRecord
AnalysisRun
ImpactAnalysis
RiskAssessment
Evidence
```

Conceptually:

``` text
ChangeSet
   │
   └── CONTAINS_CHANGE → ChangeRecord

AnalysisRun
   ├── ANALYZES → ChangeSet
   ├── PRODUCES → ImpactAnalysis
   │                  └── AFFECTS → CoIE Symbol
   ├── HAS_RISK → RiskAssessment
   ├── RECOMMENDS → TIE Test
   └── SUPPORTED_BY → Evidence
```

ChIE does not duplicate CoIE symbols or TIE tests.

------------------------------------------------------------------------

# 33. Unified Cross-Engine Graph Model

The hackathon uses one physical FalkorDB graph per workspace while
preserving logical ownership.

``` text
CoIE Code Graph
──────────────────────────────
Repository
  └── HAS_SNAPSHOT
        └── CONTAINS
              └── File
                    └── DEFINES
                          └── Symbol
                                └── CALLS
                                      └── Symbol


TIE Test Graph
──────────────────────────────
Test
  ├── DECLARED_BY → CoIE Symbol
  └── TARGETS     → CoIE Symbol


ChIE Change Graph
──────────────────────────────
ChangeSet
  └── CONTAINS_CHANGE → ChangeRecord

AnalysisRun
  ├── ANALYZES     → ChangeSet
  ├── PRODUCES     → ImpactAnalysis
  │                    └── AFFECTS → CoIE Symbol
  ├── HAS_RISK     → RiskAssessment
  ├── RECOMMENDS   → TIE Test
  └── SUPPORTED_BY → Evidence
```

The same physical graph can therefore answer:

``` text
Which code entities are connected?
        ↓
Which tests are associated?
        ↓
What would this proposed change affect?
        ↓
What evidence supports that conclusion?
```

------------------------------------------------------------------------

# 34. Agent Memory and Track 02

Agent Memory is a distinct capability.

It answers:

> **What did CodeMind learn from previous investigations, and what
> reusable context can help a future investigation?**

Memory does not replace:

-   CoIE code truth,
-   TIE test truth,
-   ChIE change conclusions,
-   live operation state.

------------------------------------------------------------------------

# 35. Memory Graph Node Catalog

The logical MVP catalog is:

``` text
ConversationSession
ConversationTurn
ReviewEpisode
ReviewFinding
ReviewFeedback       (optional if time constrained)
SemanticMemory
ProcedureMemory
Handoff
EngineReference
```

The two reusable memory forms are deliberately distinct: **SemanticMemory**
records qualified reusable knowledge about an investigation/domain fact, while
**ProcedureMemory** records a reusable investigation/review procedure. Both
remain evidence-referenced, scope-authorized and subject to the separate
validity and freshness lifecycle.

The memory graph may reference canonical entities, but it does not
duplicate them.

Example:

``` text
SemanticMemory
      ↓
EngineReference
      ↓
CoIE Symbol / TIE Test / ChIE Finding
```

------------------------------------------------------------------------

# 36. Memory Scope

The hackathon defines three logical scopes:

``` text
USER
  ↓
private review context

REPOSITORY / PROJECT
  ↓
authorized project knowledge

TEAM / TENANT
  ↓
explicitly shared knowledge
```

Scope inheritance is never implicit.

A private conversation does not automatically become project memory.

Both Agent Runtime and Rust enforce scope authorization.

------------------------------------------------------------------------

# 37. Memory Validity and Freshness

These are separate.

### Lifecycle

``` text
candidate
validated
rejected
superseded
```

### Freshness

``` text
fresh
stale
unknown
```

Therefore:

``` text
validated + stale
```

is valid and meaningful.

It means the memory was previously validated but may no longer apply to
the current repository state.

------------------------------------------------------------------------

# 38. Memory Retrieval

The MVP does not require vector embeddings.

Retrieval is structured around:

``` text
scope
+
repository/project
+
target symbol/test
+
memory type
+
provenance
+
validation state
+
snapshot compatibility
```

Then:

``` text
Previous memory
      ↓
Retrieve bounded memory
      ↓
Validate against current CoIE/TIE/ChIE evidence
      ↓
Applicable / stale / rejected
      ↓
Use only applicable context
      ↓
LLM explanation
```

Fresh evidence always has authority over old memory.

------------------------------------------------------------------------

# 39. Memory Write Pipeline

Memory writing is controlled:

``` text
Review
  ↓
ReviewEpisode
  ↓
ReviewFinding
  ↓
Memory candidate
  ↓
Schema validation
  ↓
Authorization
  ↓
Scope validation
  ↓
Provenance validation
  ↓
Evidence/reference validation
  ↓
Promotion policy
  ↓
Rust persistence
  ↓
FalkorDB
```

An LLM-generated memory proposal is not automatically trusted knowledge.

The system must not turn speculation into persistent truth.

------------------------------------------------------------------------

# 40. Handoffs

A handoff is a persistent structured work packet.

Conceptually:

``` text
Impact Investigation
        ↓
      Handoff
        ↓
Test Investigation
```

A handoff contains fields such as:

``` text
handoff_id
schema_version
operation_id
review_episode_id

source_stage
target_stage

task
findings
evidence_references

source_snapshot
graph_versions

open_questions

status
idempotency_key
sequence/version
timestamps
```

The important property is that the receiving stage actually retrieves
and consumes persisted handoff state.

It is not merely an in-memory function argument.

------------------------------------------------------------------------

# 41. Handoff Lifecycle

Handoff state is independent from operation state.

``` text
Handoff:
pending → claimed → consumed
                    ↘ expired

Operation:
running → completed
        → failed
        → cancelled
        → degraded
```

These are not interchangeable.

Handoffs must be:

-   idempotent,
-   concurrency-safe,
-   authorized,
-   recoverable after disconnect,
-   protected against silent overwrite.

------------------------------------------------------------------------

# 42. Operation State

Long-running repository preparation and investigation are operations.

The authoritative operation record contains:

-   operation ID,
-   owner/authorization scope,
-   lifecycle,
-   stage statuses,
-   latest event sequence,
-   terminal outcome,
-   result reference,
-   safe error/limitation information,
-   timestamps.

The operation must survive request termination.

``` text
HTTP request ends
      ↓
operation continues
      ↓
authoritative state persists
      ↓
browser reconnects
      ↓
current state recovered
```

------------------------------------------------------------------------

# 43. Operation Lifecycle Semantics

Do not fabricate progress percentages.

Use three distinct concepts.

### Lifecycle

``` text
running
waiting
completed
failed
cancelled
```

### Outcome

``` text
success
degraded
failure
cancelled
```

### Completeness

``` text
complete
partial
truncated
unknown
```

For example:

``` text
Lifecycle: completed
Outcome: degraded
Completeness: partial
```

is a valid result.

------------------------------------------------------------------------

# 44. Progress and Real-Time UI

There are four related but distinct observability concepts.

``` text
Rust tracing
      ↓
diagnostic logs

Typed progress events
      ↓
live operation status

GraphUpdated events
      ↓
graph publication/version changes

UI messages
      ↓
safe human-readable explanations
```

They share correlation fields such as:

``` text
operation_id
request_id
trace_id
repository_id
engine
stage
event_id / sequence
timestamp
severity/event type
```

The UI never scrapes Rust logs.

------------------------------------------------------------------------

# 45. Progress Transport

The browser-facing progress path is:

``` text
Rust Engine
    ↓
gRPC progress stream
    ↓
Agent Runtime
    ↓
Next.js SSE
    ↓
Browser
```

The browser does not connect directly to Rust.

SSE does not own the operation.

A browser disconnect therefore does not mean the operation failed.

------------------------------------------------------------------------

# 46. At-Least-Once Progress Delivery

The system does not rely on exactly-once delivery.

It uses:

``` text
stable operation ID
+
monotonic sequence
+
stable event identity
+
recoverable current state
```

If the browser receives event 18, disconnects, and reconnects after
event 25, the runtime can provide authoritative state at sequence 25 and
optionally retained events after the last known sequence.

Duplicate or stale events must never regress UI state.

------------------------------------------------------------------------

# 47. Concrete Hackathon Runtime Flow

For:

> **"What happens in my Nizaam Core codebase when I change the control
> plane?"**

the intended runtime is:

``` text
User
  ↓
LLM Agent
  │
  ├── Understand request
  │
  ├── Check repository context
  │
  ├── Repository missing?
  │       └── Ask user for repository
  │
  └── Repository available
          ↓
JavaScript semantic tool
          ↓
Rust CodeMind Service
          ↓
CoIE
          ├── resolve repository
          ├── inspect source
          ├── extract symbols
          ├── resolve relationships
          └── build/update Code Graph
          ↓
TIE
          ├── inspect test structure
          ├── map tests to code
          ├── extract test relationships
          └── build/update Test Graph
          ↓
ChIE
          ├── read Code Graph
          ├── read Test Graph
          ├── read source evidence
          ├── analyze change
          ├── determine affected entities
          ├── determine test impact
          ├── evaluate risk factors
          └── build Change Graph
          ↓
ChIE publishes completion event
          ↓
Agent Runtime updates operation state
          ↓
Semantic tools retrieve:
  ├── impact
  ├── risk
  ├── test impact
  ├── evidence
  └── graph paths
          ↓
LLM reasons over structured evidence
          ↓
Next.js chat UI
```

For the guaranteed hackathon path:

``` text
CoIE → TIE → ChIE
```

The post-hackathon optimization can become:

``` text
CoIE
 ├── tests available → TIE → ChIE
 └── no tests        → ChIE
```

The conditional path is intentionally not a hackathon blocker.

------------------------------------------------------------------------

# 48. Rust Engine Invocation vs JavaScript Tools

These are two different boundaries.

## 48.1 LLM → JavaScript tools

The LLM knows semantic tools such as:

``` text
analyze_change()
get_impact()
get_risk()
get_test_impact()
get_evidence()
```

Flow:

``` text
LLM
 ↓
Agent Runtime
 ↓
JavaScript Tool
 ↓
Tool Registry
 ↓
Validation / Authorization / Limits
 ↓
gRPC / Protobuf
 ↓
Rust
```

The LLM does not know private Rust functions such as:

``` text
coie::build_code_graph()
tie::build_test_graph()
chie::build_change_graph()
```

## 48.2 Engine → Engine

Inside Rust, the engines communicate through controlled internal typed
contracts.

Conceptually:

``` text
CoieEngine
 ├── build_graph()
 ├── query()
 └── invoke()

TieEngine
 ├── build_graph()
 ├── query()
 └── invoke()

ChieEngine
 ├── analyze()
 ├── query()
 └── invoke()
```

These are internal Rust interfaces, not model-visible tools.

------------------------------------------------------------------------

# 49. Type A and Type B Rust Functions

The Rust engine uses two broad categories of internal functions.

## Type A --- Graph construction/maintenance

Type A functions:

-   build/update graph state,
-   validate extracted entities,
-   resolve identities,
-   apply graph changes through the storage port,
-   publish graph versions,
-   report unresolved references and diagnostics.

They return a build/update report rather than the entire graph.

Conceptually:

``` text
Input
 ↓
validate
 ↓
graph storage port
 ↓
publish
 ↓
build/update report
```

## Type B / Type 2 --- Graph reading, analysis and result preparation

Type B functions:

-   retrieve symbols,
-   query callers/callees,
-   query test intelligence,
-   perform ChIE analysis,
-   prepare findings,
-   validate evidence,
-   prepare canonical results.

Conceptually:

``` text
Semantic request
 ↓
validate context
 ↓
graph storage port
 ↓
deterministic analysis
 ↓
validate evidence/completeness
 ↓
canonical structured result
```

These are not separate services.

Both operate inside the Rust service and use the same
database-independent storage boundary.

------------------------------------------------------------------------

# 50. Canonical Result Contract

Rust produces a canonical structured result.

Conceptually:

``` text
Canonical Result
├── contract version
├── operation identity
├── context
│   ├── repository
│   ├── source snapshot
│   └── graph versions
├── outcome
├── completeness
├── freshness
├── data
│   └── engine-specific payload
├── evidence references
├── pagination/truncation metadata
└── diagnostics/limitations
```

The exact field names are implementation details.

The semantics are not.

The boundary must preserve:

``` text
execution outcome
result completeness
freshness
upstream graph quality
```

Rust's canonical result remains structured nested JSON and is the semantic
source of truth. It is not destructively flattened and TOON is never the
only stored or canonical representation. The Agent Runtime parses and
validates the JSON result before producing separate UI and LLM projections.

It must also distinguish:

``` text
empty
partial
truncated
unavailable
unresolved
unsupported
invalid
failed
cancelled
```

------------------------------------------------------------------------

# 51. JavaScript Result Projection and LLM Encoding

The canonical result remains structured nested JSON. The Agent Runtime
creates separate consumer projections:

``` text
Canonical Type B JSON result
       ├── UI projection: structured JSON / UI DTO
       └── bounded LLM projection
                 ↓
          `flat` flattening
                 ↓
          `@toon-format/toon` encoding
                 ↓
          TOON text supplied to the LLM
```

The UI projection remains structured for known React components, evidence
navigation, stable IDs, expandable details and source locations. The LLM
projection is a bounded representation for the current reasoning step. The
UI and LLM must not infer canonical facts from each other's text.

## 51.1 Frozen `flat` → TOON decision

The JavaScript tool-result pipeline uses both npm packages:

- **`flat`** for flattening the bounded LLM projection of Rust Type B
  semantic results;
- **`@toon-format/toon`** for encoding that transformed projection as TOON.

This is the approved implementation decision as of 12 October 2026. It
supersedes earlier exploratory wording in the raw discussion that treated
TOON as optional or recommended passing nested data directly to TOON without
flattening. Do not reopen that decision during implementation unless a
concrete correctness blocker is demonstrated.

### 51.1.1 Benchmark rationale and implementation obligation

The official TOON benchmark does not prove that flattening an arbitrary nested
CodeMind payload will always save tokens. Its published measurements include
nested/mixed datasets where direct TOON can use more tokens than compact JSON,
while flat/tabular-friendly datasets can show a substantial TOON advantage.
The official benchmark compares the same source dataset in each format; it does
not flatten the nested dataset before encoding. Therefore CodeMind's
`flat` → TOON choice is an intentional application-level optimization, not a
claim that TOON itself requires flat input.

The implementation must validate this choice on representative CodeMind Type B
payloads rather than assuming a universal token reduction. Day 4 performance
smoke tests must compare, for the same bounded payloads:

``` text
compact JSON
   vs
native/direct TOON
   vs
flattened JSON
   vs
flattened TOON
```

The comparison must record token counts and semantic field-retrieval
correctness. Observed savings are measurements for the tested payloads, not a
universal guarantee. The benchmark does not change the frozen runtime
contract: successful LLM-facing Type B results use the `flat` →
`@toon-format/toon` pipeline.

The flattening layer must remain schema-aware and reversible. Preserve arrays
of records, evidence references, entity IDs, relationship semantics, snapshot
and graph-version context, completeness/freshness, limitations and status.
Do not flatten arrays into numeric-indexed dotted keys merely to satisfy a
mechanical definition of "flat".

The required order is:

1. Receive the Type B canonical JSON string through the typed gRPC response.
2. Parse it and validate its contract version, outcome, completeness,
   freshness, context, evidence references, pagination/truncation metadata
   and diagnostics.
3. Build a bounded LLM projection. Keep only relevant fields and preserve
   entity references, evidence references, relationship types/directions,
   source snapshot and graph-version context, outcome, completeness,
   freshness and limitations.
4. Use the `flat` package to flatten nested object paths in this projection.
   Configure flattening so arrays are preserved as arrays and their contents
   are not converted to numeric-indexed dotted keys. This keeps arrays of
   affected files, findings, graph-path steps and test candidates available
   for TOON's tabular encoding.
5. Make the path-key mapping collision-safe and reversible. Dots or the
   configured delimiter inside literal property names must not collide with
   actual nested paths. Use the package's supported configuration such as a
   reversible `transformKey` policy; do not write a custom recursive
   flattener.
6. Verify semantic preservation through `unflatten` and structural equality
   against the pre-flatten projection in tests. Supported CodeMind result
   shapes must pass this integrity check before they are exposed to the model.
   If a projection cannot be flattened without loss or ambiguity, do not
   silently bypass the approved pipeline or send the unflattened result as a
   successful model result. Return a structured formatting/contract error and
   preserve the canonical result for UI and diagnostics.
7. Encode with `@toon-format/toon` and supply the resulting TOON text as the
   LLM-visible tool result. Do not wrap the TOON text in a JSON object that
   causes the LLM to receive JSON instead of the intended TOON representation.

The default array-preservation behavior is deliberate. Blindly turning
`affectedFiles` into keys like `affectedFiles.0.path` destroys the repeated
record shape that TOON can encode efficiently. Flatten nested object-only
paths while keeping record arrays and semantic relationships intact.

The transformation is performed in one shared server-only result-encoding
module called by the JavaScript semantic tools / tool executor. Do not
reimplement the transformation independently in every tool.

## 51.2 Boundaries and non-goals

- Canonical JSON remains the internal semantic source and is not replaced by
  flattened data or TOON.
- The UI, persisted result records, gRPC payloads, progress events and
  diagnostics keep their structured contracts; they are not TOON-encoded.
- Tool-call arguments and the registry's input schemas remain their normal
  typed/JSON contracts. The TOON change applies to LLM-facing **tool result
  content** derived from Type B semantic results.
- Type A graph-build/update reports remain operational reports; do not route
  all engine progress, logs or build reports through the LLM result formatter.
- The official TOON package is the encoder/decoder. Do not author a custom
  JSON-to-TOON serializer.
- Bound the semantic projection before encoding. Never truncate the final
  TOON string at an arbitrary character boundary, because doing so can cut a
  row or invalidate declared array lengths. Reduce the projection at
  semantic boundaries and re-encode it instead.
- A formatting or flattening failure must not be presented as a successful
  canonical analysis. Preserve the original canonical result, report a
  structured formatting diagnostic, and do not silently bypass the mandatory
  `flat` → `@toon-format/toon` model-result pipeline.

------------------------------------------------------------------------

# 52. Semantic JavaScript Tool Catalog

The architectural catalog has 12 tools.

## Repository / code / test retrieval

``` text
get_repository_context()
search_code()
get_symbol()
query_code_relationships()
get_historical_changes()
get_test_intelligence()
```

## Change intelligence

``` text
analyze_change()
get_impact()
get_risk()
get_test_impact()
get_evidence()
simulate_change()
```

The initial hackathon implementation uses the first 11.

Only:

``` text
simulate_change()
```

is deferred.

The tool registry remains extensible, but internal Rust methods are not
automatically exposed as tools.

------------------------------------------------------------------------

# 53. Tool Execution Pipeline

Every semantic tool has:

-   stable identity/version,
-   description,
-   input schema,
-   authorization requirements,
-   Rust/service mapping,
-   output contract,
-   limits,
-   execution behavior,
-   error behavior.

Execution:

``` text
LLM tool call
      ↓
Registry lookup
      ↓
Validate arguments
      ↓
Authorization/context check
      ↓
Limits + duplicate-call policy
      ↓
gRPC → Rust Type B function
      ↓
Parse and validate canonical JSON result
      ↓
Create bounded LLM projection
      ↓
`flat` flattening (preserve arrays; verify reversibility)
      ↓
`@toon-format/toon` encoding
      ↓
TOON text as the LLM-visible tool result
      ↓
LLM explanation / follow-up tool selection
```

For a result whose flattening cannot be proven reversible, the tool executor
must fail that model-facing result preparation explicitly rather than silently
bypassing `flat`. The canonical structured result remains available to the UI,
operation state and diagnostics. The LLM must never receive canonical JSON text
as a substitute for the approved TOON result path.

------------------------------------------------------------------------

# 54. Tool Guardrails

The Agent Runtime enforces:

-   tool-call budgets,
-   total operation deadlines,
-   per-tool timeouts,
-   result-size limits and LLM context/token budgets,
-   semantic projection limits before serialization,
-   pagination limits,
-   traversal limits,
-   duplicate-call policy,
-   evidence sufficiency,
-   loop exhaustion,
-   authorization,
-   repository/snapshot context.

The LLM cannot bypass these limits.

------------------------------------------------------------------------

# 55. Security Boundary

The model must not manufacture:

``` text
user_id
repository_id
operation_id
permissions
evidence access
```

The runtime binds these values to authenticated application context.

The system also forbids:

-   arbitrary Cypher,
-   unrestricted graph traversal,
-   arbitrary repository command execution,
-   raw credentials,
-   unrestricted internal diagnostics,
-   unsafe source execution,
-   direct FalkorDB access from the LLM.

Repository source, comments, README files, test names, reports and other
retrieved repository artifacts are untrusted data. They may contain prompt
injection or misleading instructions. Source content must never override tool
policies, authorization, credentials or operation boundaries.

CodeMind must not execute arbitrary repository code during the initial static
analysis/test-discovery path. Running tests, builds or repository-controlled
scripts is outside the hackathon MVP.

------------------------------------------------------------------------

# 56. FalkorDB Architecture

FalkorDB is mandatory and central.

It is not just a graph visualization layer.

The graph must perform actual work in:

-   relationship traversal,
-   multi-hop investigation,
-   test association,
-   change impact,
-   evidence/path retrieval,
-   persistent memory,
-   coordination/handoffs where represented in the graph.

## 56.1 Storage boundary

``` text
CoIE / TIE / ChIE / Memory
            ↓
     Graph Storage Port
            ↓
     FalkorDB Adapter
            ↓
         FalkorDB
```

Only the Rust storage layer knows FalkorDB-specific APIs and Cypher.

------------------------------------------------------------------------

# 57. One Physical Graph, Multiple Logical Domains

The hackathon uses one physical FalkorDB graph per workspace.

Logical ownership remains:

``` text
CoIE
├── Code Graph
└── History Graph

TIE
└── Test Graph

ChIE
└── Change Graph

Agent Memory
└── Memory Graph
```

This allows cross-domain traversal without duplicating canonical
entities.

For example:

``` text
TIE Test
   ↓ TARGETS
CoIE Symbol

ChIE ImpactAnalysis
   ↓ AFFECTS
CoIE Symbol

ChIE AnalysisRun
   ↓ RECOMMENDS
TIE Test
```

------------------------------------------------------------------------

# 58. Repository and Source Storage

The graph is not the primary storage for full source contents.

The architecture keeps:

-   source snapshots,
-   source content,
-   snapshot manifests,

outside FalkorDB, while the graph stores structural facts and references
to source/provenance.

This avoids turning FalkorDB into a source-code blob store.

For the hackathon, the repository provider and controlled fixtures
provide deterministic source inputs.

------------------------------------------------------------------------

# 59. Repository Providers

Repository access is a shared capability.

Conceptually:

``` text
              RepositoryProvider
                 /    |    \
                /     |     \
             CoIE    TIE    ChIE
                \     |     /
                 \    |    /
              repository source
```

The hackathon path primarily uses:

-   the bundled demo fixture,
-   bounded public GitHub repository import where implemented.

Local repository support is a product workflow consideration, but the
guaranteed demo path remains the controlled fixture.

------------------------------------------------------------------------

# 60. Frontend Architecture

The hackathon UI is **AI-agent/chat-first**.

It is not a generic dashboard.

The chat conversation is contextual:

``` text
Repository
+
selected symbol/file
+
current snapshot
+
investigation question
```

Structured engineering components appear inside the agent experience.

The visual design itself is specified separately in `design-system.md`.

------------------------------------------------------------------------

# 61. Main UI Screens

The hackathon product contains these application-level experiences.

### Welcome / Home

Provides:

-   conversation history and a conversation-search modal,
-   New Chat,
-   repository selector,
-   suggested investigation prompts,
-   main input,
-   repositories,
-   settings with its internal settings subsections,
-   help/docs.

### Repository management

Provides:

-   available repositories,
-   indexing state,
-   graph readiness,
-   repository context,
-   open/re-analyze actions.

### Repository setup

The Add Repository flow supports:

``` text
Demo workspace
Public GitHub import
```

and displays:

``` text
accepted
indexing
completed
partial
unsupported
failed
```

### Investigation workspace

This is the primary UI.

------------------------------------------------------------------------

# 62. Investigation Chat UI

The expanded investigation workspace uses structured tabs above the
input/result area.

The hackathon UI includes:

``` text
Chat
Execution
Summary
Impact Analysis
Affected Files
Test Impact
Dependency Graph
Evidence
Recommendations
```

The conversation remains the primary interaction surface.

The tabs provide progressive access to structured investigation
information.

------------------------------------------------------------------------

# 63. Execution Tab

During engine execution the user can see:

``` text
LLM
 ↓
CoIE   ● Running
 ↓
TIE    ○ Waiting
 ↓
ChIE   ○ Waiting
```

Engine progress is represented with expandable accordions.

For example:

``` text
✓ CoIE
  Building code graph...
  1,842 symbols
  6,421 relationships
  ▼
```

When expanded:

``` text
✓ CoIE
  Repository resolved
  Source snapshot verified
  Files parsed
  Symbols extracted
  Relationships resolved
  Code Graph published
```

The same pattern is used for:

``` text
TIE
ChIE
```

The browser does not scrape logs.

Actual engine progress events populate these UI components.

------------------------------------------------------------------------

# 64. Post-Engine Reasoning State

After:

``` text
✓ CoIE
✓ TIE
✓ ChIE
```

the UI transitions to:

``` text
LLM
● Reasoning about the collected evidence...
```

The Agent Runtime then retrieves:

``` text
impact
risk
test impact
evidence
graph paths
```

and supplies bounded structured information to the LLM.

The LLM does not calculate the canonical impact or risk itself.

------------------------------------------------------------------------

# 65. Completion Signal

When ChIE finishes building and publishing the Change Graph:

``` text
ChIE
  ↓
Change Graph published
  ↓
typed completion/progress event
  ↓
Agent Runtime
  ↓
operation state updated
  ↓
semantic result tools become usable
```

The event can conceptually contain:

``` text
operation_id
engine = chie
stage = change_graph
type = stage_completed
graph_version
analysis_run_id
```

This is how the runtime knows the Change Graph is ready.

The LLM does not directly wait on an undocumented "magic signal".

------------------------------------------------------------------------

# 66. Structured Result UI

The main result sections are:

``` text
Summary
Impact
Risk
Test Impact
Evidence
Graph Paths
Memory
```

The UI never parses free-form LLM text to decide which facts to display.

The structured result determines:

-   affected entities,
-   risk,
-   tests,
-   evidence,
-   completeness,
-   freshness,
-   limitations.

------------------------------------------------------------------------

# 67. Impact UI

Impact communicates consequences, not raw graph data.

Example:

``` text
Impact
HIGH

Directly affected
4 symbols

Indirectly affected
17 files

Critical paths
3

Completeness
Complete
```

The detailed path becomes available through evidence/graph expansion.

------------------------------------------------------------------------

# 68. Risk UI

Risk must avoid false precision.

Example:

``` text
Risk
HIGH

Why
• Broad structural exposure
• Sensitive boundary affected
• Historical instability detected

Confidence
MEDIUM

Review severity
HIGH
```

Unknown evidence remains visible.

------------------------------------------------------------------------

# 69. Test Impact UI

Recommendations are separated from impact.

Example:

``` text
Recommended tests

High priority
  capability registration test
  runtime dispatch test

Standard priority
  integration test

Additional candidates
  security conformance test
```

A test is displayed once even if several affected entities lead to it.

Opening the recommendation shows the supporting path/evidence.

------------------------------------------------------------------------

# 70. Graph Path UI

The graph viewer answers:

> **"How did CodeMind reach this conclusion?"**

It displays bounded relevant paths, not the entire repository.

Example:

``` text
Changed Symbol
      ↓ CALLS
Function A
      ↓ IMPORTS
Module B
      ↓ TARGETS
Test C
```

The UI must show:

-   direction,
-   relationship type,
-   selected path,
-   truncation/limitation state.

------------------------------------------------------------------------

# 71. Evidence UI

Evidence is expandable.

A compact finding may say:

``` text
Why is this affected?

2 evidence items

[ View evidence ]
```

Expansion can show:

-   source entity,
-   target entity,
-   relationship,
-   source location,
-   snapshot,
-   graph version,
-   supporting path,
-   diagnostics,
-   uncertainty,
-   limitations.

------------------------------------------------------------------------

# 72. Memory UI

Memory is secondary to the investigation.

The user can inspect:

``` text
Previous memory
Retrieved
Validated against current graph
Applicable
Used in reasoning
```

If stale:

``` text
Previous memory
Retrieved
Current graph differs
Status: Stale
Not used as authoritative evidence
```

This is central to the Track 02 trust story.

------------------------------------------------------------------------

# 73. Handoff UI

The handoff can be shown as:

``` text
Impact Analysis
      │
      │ persisted handoff
      ▼
Test Investigation
      │
      ▼
Test Evidence
```

The UI exposes meaningful coordination state without exposing internal
implementation details.

------------------------------------------------------------------------

# 74. Loading, Partial and Failure States

Every major UI component supports:

``` text
Loading
Running
Completed
Partial
Truncated
Unavailable
Failed
Cancelled
Empty
```

Examples:

``` text
Analysis completed with limitations.

Some relationships could not be resolved.

3 findings have complete evidence.
2 findings have incomplete evidence.
```

The UI distinguishes:

``` text
No matching relationships
```

from:

``` text
Analysis incomplete
```

from:

``` text
Graph unavailable
```

from:

``` text
No supported evidence
```

------------------------------------------------------------------------

# 75. Reconnect Behavior

If the browser disconnects:

``` text
Connection interrupted.

The analysis is still running.
Reconnecting...
```

After reconnection:

``` text
Analysis resumed

Current stage:
Finding test impact
```

The runtime recovers authoritative operation state.

A reconnect must not create a second operation.

------------------------------------------------------------------------

# 76. Error Architecture

Errors are cross-cutting states rather than separate product domains.

The UI must support at least:

-   no internet,
-   API unavailable,
-   repository not found,
-   repository access denied,
-   invalid repository URL,
-   repository analysis failure,
-   chat error,
-   rate limit,
-   agent execution failure,
-   LLM reasoning failure,
-   graph error,
-   file/symbol not found,
-   test analysis error,
-   unsupported syntax,
-   partial indexing,
-   unresolved relationship,
-   FalkorDB failure,
-   Rust service failure.

Errors should preserve whether the operation itself failed or only the
browser connection failed.

------------------------------------------------------------------------

# 77. No Arbitrary LLM-Generated UI

The LLM can provide:

-   explanations,
-   summaries,
-   tool selections,
-   grounded reasoning text.

It cannot decide to create arbitrary UI components.

Instead:

``` text
Rust / Runtime
      ↓
Canonical structured result
      ↓
UI projection
      ↓
Known React component
```

This keeps the product deterministic and testable.

------------------------------------------------------------------------

# 78. Project Structure

The canonical physical layout is defined in full in the root
`implementation-plan.md`. Both documents use the same Next.js `src/`
layout; there is no parallel `apps/web/app/` or `apps/web/components/`
tree.

```text
codemind/
├── AGENTS.md
├── README.md
├── implementation-plan.md
├── package.json
├── pnpm-workspace.yaml
├── playwright.config.ts
├── .env.example
├── docker-compose.yml
├── apps/web/
│   ├── package.json                  # flat + @toon-format/toon dependencies
│   └── src/
│       ├── app/                      # pages + Route Handlers
│       ├── components/               # chat/search, execution, analysis, graph, evidence, memory, repository, settings, common
│       └── lib/
│           ├── shared/                # UI-safe types/status
│           └── server/
│               ├── agent/
│               ├── tools/             # registry, schemas, executor, guardrails, implementations
│               ├── workflows/
│               ├── memory/
│               ├── handoffs/
│               ├── operations/
│               ├── results/
│               │   ├── canonical-json.ts
│               │   ├── validate.ts
│               │   ├── llm-projection.ts
│               │   ├── llm-encoding.ts  # flat → TOON, shared by JS tools
│               │   ├── llm-encoding.test.ts
│               │   └── ui-projection.ts
│               ├── rust-client/        # gRPC client + generated bindings
│               ├── auth/
│               └── config/
├── proto/codemind/v1/                 # canonical Protobuf source
├── rust/crates/codemind-engine/
│   ├── src/                           # CoIE, TIE, ChIE, contracts, storage, lifecycle
│   └── tests/
├── fixtures/repositories/codemind-polyglot-demo/
│   ├── rust/
│   ├── typescript/
│   └── python/
├── tests/
│   ├── e2e/
│   ├── language-adapters/
│   ├── graph-conformance/
│   ├── memory/
│   └── fixtures/expected-graph/
├── docs/
│   ├── architecture.md
│   ├── testing.md
│   ├── design-system.md
│   ├── demo.md
│   ├── graph-model.md
│   ├── language-support.md
│   └── graph-queries.md
└── scripts/
    ├── setup.sh
    ├── generate-proto.sh
    ├── seed-demo.sh
    ├── reset-demo.sh
    ├── run-e2e.sh
    └── verify-clean-start.sh
```

The dependency rule remains one-way: browser → Next.js application /
server-side Agent Runtime → gRPC/Protobuf → Rust engine → graph-storage
port → FalkorDB. The packages `flat` and `@toon-format/toon` run only in
server-side JavaScript result formatting; they are not used in Rust or the
browser.

------------------------------------------------------------------------

# 79. Development Topology

The normal hackathon development environment uses three primary
processes:

``` text
Process 1
FalkorDB
Docker

Process 2
Rust CodeMind Engine
cargo run

Process 3
Next.js
pnpm dev
```

Runtime topology:

``` text
Browser
   │
   │ HTTP / SSE
   ▼
Next.js + Agent Runtime
   │
   │ gRPC
   ▼
Rust CodeMind Engine
   │
   │ graph storage port
   ▼
FalkorDB
```

Docker Compose provides the graph infrastructure.

Persistent FalkorDB data is retained for the demo.

------------------------------------------------------------------------

# 80. Fixtures and Polyglot Demo

The hackathon includes a controlled fixture repository under:

``` text
fixtures/repositories/
```

The primary proof includes:

``` text
Rust
TypeScript / JavaScript
Python
```

The fixture must contain enough real relationships and tests to
demonstrate:

-   code graph construction,
-   cross-language adapter conformance,
-   test graph construction,
-   change-impact traversal,
-   evidence,
-   recommendations,
-   memory/handoff behavior.

The demo uses the same real application pipeline as normal use.

There is no separate fake demo UI.

The final submission must also preserve these judge-facing constraints:

- The public repository is mandatory.
- Live deployment is preferred only when reliable; reliable local execution is
  the fallback.
- The bundled fixture is the guaranteed offline demo path and must not depend
  on GitHub or another external network service.
- The final submission explicitly discloses the use of AI coding assistance.
- The approximately three-minute demo is one coherent story: product first,
  Rust hero investigation, real FalkorDB graph work and evidence, Track 02
  handoff/fresh-session memory, then the TypeScript/JavaScript and Python proof.
- The architecture is explained with one concise diagram, while the graph
  model and important Cypher queries/graph algorithms are documented in the
  repository documentation.

------------------------------------------------------------------------

# 81. Repository Preparation Workflow

The user can enter through:

``` text
Use Example Repository
```

or a bounded public GitHub import.

The preparation lifecycle is visible:

``` text
Repository accepted
      ↓
Snapshot resolved
      ↓
CoIE indexing
      ↓
TIE discovery
      ↓
Graph ready
```

Possible states:

``` text
ready
partial
unsupported
failed
unavailable
```

The core demo does not depend on external GitHub availability.

------------------------------------------------------------------------

# 82. Demo Workflow

The main demo is approximately three minutes.

## 82.1 Track 01

``` text
Open CodeMind
      ↓
Open polyglot workspace
      ↓
Select Rust symbol
      ↓
Ask:
"What could this change affect?"
      ↓
CoIE
      ↓
TIE
      ↓
ChIE
      ↓
Impact
      ↓
Risk
      ↓
Tests
      ↓
Evidence
      ↓
Graph path
```

## 82.2 Track 02

Continue the same story:

``` text
Persist review
      ↓
Persist handoff
      ↓
New session
      ↓
Retrieve memory
      ↓
Validate against current graph
      ↓
Use memory in next investigation
```

## 82.3 Three-language proof

Rust is the primary hero path.

TypeScript and Python demonstrate that the canonical graph and test
model are not Rust-specific.

------------------------------------------------------------------------

# 83. Why FalkorDB Is Central

The product should fail its own design goal if FalkorDB's graph
relationships are removed.

The graph is required for:

``` text
multi-hop code traversal
+
test relationships
+
change impact
+
evidence paths
+
persistent memory/coordination
```

The demo must visibly show graph work rather than only displaying a
graph visualization.

The core explanation to a judge is:

> **The LLM decides what to investigate. FalkorDB and deterministic Rust
> intelligence determine what is actually true.**

------------------------------------------------------------------------

# 84. Testing Architecture

Testing is part of implementation, not a final Day 4 activity.

The hackathon test layers include:

``` text
Rust engine unit tests
Language adapter conformance
Canonical graph validation
FalkorDB integration
ChIE tests
TIE tests
gRPC contract tests
Agent Runtime tests
Tool registry and guardrails
Operation lifecycle
Memory
Handoffs
Security/isolation
Next.js/Route Handlers
Chat/UI components
Track 01 E2E
Track 02 E2E
Failure/partial/recovery
Performance smoke tests
Day 4 complete verification
```

The critical E2E proof includes:

### Rust

``` text
Rust fixture
 → CoIE
 → TIE
 → ChIE
 → FalkorDB
 → structured result
```

### TypeScript/JavaScript

Same semantic workflow through the adapter.

### Python

Same semantic workflow through the adapter.

### Connected-data proof

The answer must depend on actual FalkorDB relationship traversal.

### LLM result encoding proof

Test the shared JavaScript result pipeline with the actual `flat` and
`@toon-format/toon` packages. Cover flat and deeply nested objects, arrays
of primitives, arrays of uniform records, nested objects inside arrays,
nested arrays, mixed/non-uniform arrays, empty objects/arrays, nulls,
booleans, numbers, strings, keys containing the configured path delimiter
(including literal dots), CodeMind evidence objects, impact paths and large
bounded Type B results. Verify that flatten/unflatten is reversible, that
TOON encode/decode preserves the transformed JSON model, and that the full
path preserves the pre-flatten projection. Verify that a flattening collision or other non-reversible case produces an
explicit formatting/contract error rather than bypassing `flat`, and assert
that every successful LLM-visible tool result is TOON text rather than a JSON
wrapper. Record token
counts for compact JSON, direct TOON, flattened JSON and flattened TOON on
representative result shapes; do not claim a universal token-saving ratio.
Also run a small controlled LLM field-retrieval smoke test using fixed
questions and known expected values against the same canonical payload in
compact JSON, direct TOON and flattened TOON form. Record the configured
provider/model and compare correctness, not prose style. Prefer provider
usage metadata for token counts; otherwise document the tokenizer used as a
comparative proxy. These measurements validate the frozen pipeline and do
not turn the mandatory TOON tool-result contract into an optional path.


### Track 02

Memory must survive a fresh session and influence a later investigation.

------------------------------------------------------------------------

# 85. Reliability and Honesty Rules

The architecture deliberately prefers an honest partial answer over a
confident invented answer.

Examples:

### Bad

``` text
No relevant tests.
```

when TIE is unavailable.

### Correct

``` text
Test impact could not be fully determined.
TIE data is unavailable for the requested snapshot.
```

### Bad

``` text
Complete dependency analysis.
```

when traversal was truncated.

### Correct

``` text
Dependency analysis is partial.
Traversal limit reached before the full path could be established.
```

### Bad

``` text
Risk = 0
```

because evidence is missing.

### Correct

``` text
Risk cannot be fully assessed.
Confidence is low because required validation evidence is unavailable.
```

------------------------------------------------------------------------

# 86. Deferred Architecture

The following are intentionally outside the hackathon implementation:

## Advanced change simulation

The architecture supports:

``` text
simulate_change()
```

but it is deferred from the initial tool set.

## Full Git diff/PR intelligence

The MVP uses a selected symbol/file as a prospective change target.

## Runtime test execution

TIE initially discovers tests and consumes available evidence.

## Advanced history graph

The MVP retains enough source-revision provenance to identify the
analyzed state.

## Vector memory

The MVP uses structured graph retrieval.

## Agent swarms

The MVP uses one Agent Runtime and deterministic Rust engines.

## Distributed infrastructure

No:

``` text
Kafka
Redis
Temporal
RabbitMQ
Kubernetes
distributed worker fleet
```

unless an actual blocker forces a later architecture revision.

## MCP

MCP is architecturally compatible but not a core dependency.

If introduced later, it must front the same semantic tool registry
rather than creating a second abstraction.

------------------------------------------------------------------------

# 87. Final Architecture Summary

The complete hackathon architecture can be reduced to:

``` text
                         USER
                           │
                           ▼
                    CodeMind Chat UI
                           │
                           ▼
                    Next.js Application
                           │
                           ▼
                    Agent Runtime
                           │
             ┌─────────────┴─────────────┐
             │                           │
             ▼                           ▼
      Semantic Tool Registry       Operation / Memory
             │                           │
             └─────────────┬─────────────┘
                           │
                       gRPC / Proto
                           │
                           ▼
                 Rust CodeMind Engine
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
            CoIE          TIE           ChIE
             │             │             │
             └─────────────┼─────────────┘
                           │
                  Graph Storage Port
                           │
                           ▼
                       FalkorDB
```

The runtime investigation is:

``` text
User question
    ↓
LLM understands request
    ↓
Semantic tool
    ↓
CoIE builds/queries code graph
    ↓
TIE builds/queries test graph
    ↓
ChIE derives change intelligence
    ↓
Change Graph published
    ↓
Typed completion event
    ↓
Agent Runtime retrieves structured results
    ↓
LLM explains evidence
    ↓
Chat UI renders known components
    ↓
Review episode + memory + handoff persist
```

The ownership model is:

``` text
CoIE
  = canonical code/history facts

TIE
  = canonical test facts/evidence

ChIE
  = change-specific conclusions

Agent Runtime
  = orchestration + tools + memory policy

LLM
  = intent understanding + tool selection + explanation

FalkorDB
  = connected graph context and persistence

Next.js
  = application/UI boundary
```

And the central product promise remains:

> **CodeMind gives an AI codebase investigation agent a persistent, queryable
> model of how a codebase is connected, so it can investigate what a change could
> affect, explain the evidence behind its conclusions, and carry
> validated engineering context into the next session.**

------------------------------------------------------------------------

# 88. Hackathon Architecture Invariants

These rules must not be violated during implementation without an
explicit architecture review.

1.  FalkorDB remains the primary graph database.
2.  FalkorDB performs real reasoning work.
3.  The LLM never directly accesses FalkorDB.
4.  The LLM never receives arbitrary Cypher access.
5.  The LLM never directly invokes private Rust functions.
6.  CoIE owns canonical code/history facts.
7.  TIE owns canonical test facts/evidence.
8.  ChIE owns change-derived conclusions.
9.  ChIE does not duplicate CoIE/TIE canonical entities.
10. TIE does not become ChIE.
11. The Agent Runtime does not become a second Rust intelligence engine.
12. Next.js Route Handlers remain thin.
13. Rust remains a separate service.
14. CoIE/TIE/ChIE remain modules inside one Rust service.
15. FalkorDB access remains behind the Rust storage boundary.
16. Semantic tool access remains bounded and authorized.
17. Canonical results remain separate from LLM/UI projections.
18. Impact, risk, confidence and review severity remain distinct.
19. Completeness remains separate from outcome.
20. Freshness remains separate from validity.
21. Partial/truncated/unresolved/unavailable states remain explicit.
22. Browser disconnect does not automatically cancel an operation.
23. Operation state does not become memory.
24. Memory does not replace current engine evidence.
25. Persistent handoffs remain structured and idempotent.
26. The hackathon demo uses the real application pipeline.
27. The UI remains agent-first/chat-first.
28. The UI does not render arbitrary LLM-generated components.
29. Rust/TypeScript/Python remain part of the MVP proof.
30. The implementation must remain inside the frozen hackathon scope.
31. Track 01 and Track 02 are the target tracks; Track 03 is not targeted.
32. Rust Type B canonical results cross gRPC as JSON text in the typed
    Protobuf response and remain structured internally.
33. JavaScript server-side semantic tool results use `flat` followed by the
    official `@toon-format/toon` encoder before being supplied to the LLM.
34. `flat` configuration must preserve arrays and use collision-safe,
    reversible path-key handling; do not blindly flatten record arrays.
35. Successful LLM-visible tool results are TOON text produced after the
    mandatory `flat` stage; non-reversible flattening is an explicit formatting
    error, not a silent bypass.
36. UI projections, canonical results, graph persistence, tool input schemas,
    gRPC contracts and progress/events are not converted to TOON.
37. A flattening/integrity failure is explicit and diagnostic; it does not
    silently bypass the mandatory `flat` stage or send canonical JSON to the
    LLM as a successful result.
38. No custom JSON-to-TOON serializer or independent per-tool serializer is
    allowed; use one shared server-side result-encoding module.
39. The LLM-result representation never becomes a source of canonical graph
    facts and is never the only persisted result representation.

------------------------------------------------------------------------

## Final Position

The hackathon implementation is intentionally narrow but technically
deep.

It is not trying to build the complete future CodeMind product.

It is proving two things:

### Track 01 proof

> **An AI agent can reason over connected code data through FalkorDB and
> explain the path behind its conclusions.**

### Track 02 proof

> **The agent can preserve useful engineering context across sessions
> and use it again after validating it against current graph evidence.**

Everything else in this document exists to make those two proofs
reliable, explainable and reproducible.
