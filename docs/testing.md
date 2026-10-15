# CodeMind Testing Plan

**Scope:** Graph Hacks: Context for AI Agents, powered by FalkorDB  
**Primary purpose:** The authoritative Day 4 verification and go/no-go plan for CodeMind. Tests are written alongside implementation on Days 1–3 and executed as part of this final verification.  
**Core test principle:** Verify canonical facts, graph relationships, evidence, lifecycle, persistence and recovery. Do not use exact LLM wording as the primary correctness assertion.

> **Day 4 is not the day to write the test suite from scratch.** Tests are added alongside implementation on Days 1–3. Day 4 freezes features and executes the complete verification sequence, repairs failures, rehearses both tracks and validates the submission from a clean start.

---

# 1. Why This Document Exists

CodeMind's hackathon MVP has two outcomes to prove:

1. **Track 01 — Agents That Act on Connected Data:** A user asks what a proposed change could affect; CodeMind uses real FalkorDB relationships and deterministic Rust intelligence to derive impact, risk, test candidates and evidence-backed graph paths.
2. **Track 02 — Agent Memory and Coordination:** A review episode, useful memory and structured handoff persist; a later/fresh session retrieves relevant memory, validates it against current evidence, and uses the persisted handoff and applicable memory in a subsequent investigation.

This plan ensures the system does not merely appear to work in a happy-path screenshot. It verifies the underlying contracts and the complete application workflow.

## 1.1 Testing objectives

The test suite must establish that:

- Rust is the deterministic source of CoIE, TIE and ChIE analysis.
- Rust, TypeScript/JavaScript and Python pass the same canonical graph and investigation contract where supported.
- FalkorDB is the actual primary graph database and its relationships materially determine the investigation result.
- Graph writes are persistent, idempotent and workspace-scoped.
- Traversal is bounded, reproducible and honest about unresolved or truncated evidence.
- Tool inputs are validated and authorized before execution.
- Canonical results retain provenance, completeness, freshness, evidence and limitations through Protobuf/gRPC and the Agent Runtime.
- LLM explanations cannot manufacture engine-derived facts.
- Progress events and operation state remain correct through disconnects, duplicates, retries and reconnects.
- Memory survives a fresh session and never silently overrides current engine evidence.
- A persisted handoff is retrieved and consumed by a receiving stage; it is not merely passed in memory.
- The chat UI renders known structured components for complete, partial, truncated, empty, failed, disconnected and recovered states.
- The central demo works from a clean local repository/source-input path without requiring public GitHub or internet availability.

---

# 2. Testing Rules That Must Not Be Compromised

## 2.1 Testing is continuous

Every implementation stage adds the tests for the behavior it introduces. Day 4 runs and connects those tests, fixes integration problems and verifies the product from a clean environment.

## 2.2 Use layered tests

```text
Unit
  ↓
Component / Contract
  ↓
Integration
  ↓
Cross-system
  ↓
Full end-to-end
  ↓
Clean-start demo rehearsal
```

When an E2E test fails, use the lower-level suites to localize the root cause instead of repeatedly rerunning the entire application.

## 2.3 Test canonical facts, not AI prose

Correctness assertions should inspect:

- canonical entities,
- relationship types and direction,
- source locations and snapshot context,
- direct/indirect impact classification,
- evidence references,
- test relationship semantics,
- risk factor inputs and outcome,
- completeness and freshness,
- memory scope/applicability,
- handoff lifecycle,
- operation state and event sequence.

Do **not** require the LLM to use the exact same words in each run. It is acceptable for two grounded explanations to be worded differently. It is not acceptable for either explanation to invent a file, graph path, test, count or risk factor absent from the canonical result.

## 2.4 Deterministic tests first

Use the controlled local test inputs, a known graph seed, expected-graph assertions and controlled service failures. Network-dependent tests are optional diagnostics and must not gate the offline core workflow.

## 2.5 The graph must do real work

Mocks may be used to test error mapping and tool boundaries. They are not sufficient proof of CodeMind's central behavior. At least one Track 01 test must persist source-derived relationships in a real FalkorDB instance, traverse those relationships and show that the resulting path materially changes the impact/evidence result.

## 2.6 No hidden success

The following states are different and tests must preserve their distinction:

```text
empty
partial
truncated
unresolved
unsupported
unavailable
invalid
failed
cancelled
```

In particular:

- no matches in a successful supported query is not the same as an unavailable graph;
- an unresolved relationship is not a confirmed relationship;
- a static test target is not proof of runtime coverage;
- a truncated traversal is not a complete traversal;
- unknown validation evidence is not proof of safety;
- an SSE/browser connection failure is not automatically an operation failure.

## 2.7 Repeatability

The critical tests must be runnable more than once from the same clean state and yield the same canonical assertions. Do not create tests dependent on live repository changes or exact LLM response text.

---

# 3. Test Layers and Required Suites

The monorepo uses tests at both Rust-crate and top-level cross-system layers.

## 3.1 Rust unit/component tests

Location: inline Rust unit tests and the `engine/tests/` integration test directory.

Required focus:

- canonical identity and source normalization,
- CoIE extraction and relationship resolution,
- TIE test discovery/mapping,
- ChIE traversal, impact aggregation and risk factors,
- canonical result and error semantics,
- Type A graph-build/update versus Type B read/query/analyze contracts, including mutation boundaries and bounded outputs,
- operation lifecycle transitions,
- memory scope and lifecycle validation,
- handoff idempotency.

## 3.2 Language adapter conformance

Location:

```text
apps/web/src/tests/language-adapters/
├── rust-adapter.spec.ts
├── typescript-adapter.spec.ts
└── python-adapter.spec.ts
```

All initial language adapters must produce the same canonical semantics for equivalent supported constructs. Language-specific syntax differences are allowed; incompatible meanings in the canonical result are not.

## 3.3 Canonical graph conformance

Location:

```text
apps/web/src/tests/graph-conformance/
├── canonical-model.spec.ts
├── relationship-semantics.spec.ts
└── snapshot-isolation.spec.ts
```

This suite verifies canonical entity and connector semantics independent of the parser that produced them.

## 3.4 Memory and handoff tests

Location:

```text
apps/web/src/tests/memory/
├── persistence.spec.ts
├── handoff.spec.ts
└── scope-isolation.spec.ts
```

This suite tests persistence and coordination across sessions/restarts, not merely state retained in a page or a process-local variable.

## 3.5 Full end-to-end tests

Location:

```text
apps/web/src/tests/e2e/
├── track01.spec.ts
├── track02.spec.ts
├── polyglot-workflow.spec.ts
└── reconnect-recovery.spec.ts
```

These tests exercise the actual application boundaries and real FalkorDB behavior.

## 3.6 Suggested supporting Rust integration suite

```text
engine/tests/
├── canonical_graph.rs
├── language_adapters.rs
├── falkordb_integration.rs
├── grpc_contract.rs
├── chie_impact.rs
├── tie_discovery.rs
├── memory_persistence.rs
├── handoff_lifecycle.rs
└── operation_recovery.rs
```

The file names above follow the implementation plan. If a closely coupled test is already organized within a neighboring Rust test module, do not duplicate it just to match a filename. The required coverage and assertions are authoritative.

## 3.7 Type A / Type B engine function contract

Test the distinction established by the engine design:

- **Type A functions** build/update graph state and return typed build/update reports, including relevant snapshot/graph-version and diagnostic information. Repeat calls with the same idempotency/context must not create duplicate canonical facts.
- **Type B functions** read/query/analyze canonical state and return bounded canonical structured results. They must not mutate canonical graph state as a side effect.
- Both categories validate typed inputs, preserve authorization/workspace context, use safe errors, and retain provenance. No private Rust function is directly exposed as an LLM tool.
- The JavaScript semantic-tool adapter is responsible for transforming a validated Type B result into the bounded LLM-facing representation; the Rust canonical result and its UI projection must remain structured data. The exact LLM encoding tests are specified in Section 11.7.

---

# 4. Environment and Test Data Contract

## 4.1 Canonical test topology

The standard test topology is:

```text
Test Runner
   │
   ├── Rust unit/component tests
   ├── Next.js / Agent Runtime tests
   ├── cross-system tests
   └── browser/E2E tests
          │
          ▼
      Next.js + Agent Runtime
          │ gRPC / Protobuf
          ▼
      Rust CodeMind Engine
          │ Graph Storage Port
          ▼
        FalkorDB
```

The three main runtime processes are:

1. FalkorDB, normally through Docker Compose;
2. Rust CodeMind Engine;
3. Next.js with the Agent Runtime inside the same application.

There must not be a test-only direct data path from the browser to FalkorDB.

## 4.2 Use controlled polyglot test inputs

Test-input source trees are created or selected by the test harness; there is no required product-level repository fixture.

```text
temporary test repositories/source trees created by the test harness
├── rust/
├── typescript/
└── python/
```

Each language test input must include:

- files and declarations,
- supported structural relationships,
- at least one real multi-hop dependency chain,
- supported test declarations,
- a meaningful mapping from a test candidate to relevant code,
- at least one intentionally unresolved or unsupported construct for limitation testing.

The three subprojects demonstrate the same semantic investigation flow. Do not invent cross-language function calls merely to make a unified demonstration look more connected.

## 4.3 Expected graph assertions

Expected graph data under `tests/apps/web/src/tests/graph-conformance/expected/` should assert source-supported facts, including:

- canonical entity kind,
- repository-relative path,
- symbol identity and kind,
- supported connector type and direction,
- source range where available,
- test discovery and mapping semantics,
- unresolved/unsupported diagnostics.

Expected counts must be derived from the actual checked-in fixtures. Never copy illustrative counts from UI mockups into tests.

## 4.4 Isolate test data

Integration tests must use an isolated graph/workspace context or a disposable test graph. They must not clear or corrupt the persisted demo graph accidentally.

The test setup should provide:

- deterministic fixture seeding,
- an explicit test namespace/context,
- repeatable cleanup,
- a separate explicit demo reset operation,
- a way to verify that workspace A cannot query workspace B.

---

# 5. CoIE — Code Intelligence Tests

CoIE owns canonical code/repository facts and the Code Graph. The purpose of the tests is to ensure graph facts are source-derived, correctly identified, snapshot-compatible and honest about incomplete resolution.

## 5.1 Source inventory and snapshot tests

| ID | Test | Expected result |
|---|---|---|
| COIE-001 | Ingest an unchanged controlled test input twice | Same canonical entities and relationships; no duplicates |
| COIE-002 | Resolve the same repository snapshot twice | Stable repository/snapshot identity |
| COIE-003 | Ingest two distinct snapshots | Snapshot provenance is not silently mixed |
| COIE-004 | Inventory nested directories/files | Repository-relative paths and containment are correct |
| COIE-005 | Include unsupported file syntax | Unsupported diagnostic; no fabricated symbols/edges |
| COIE-006 | Fail parsing one supported file | Honest partial/degraded outcome if other valid graph data is retained |
| COIE-007 | Query source evidence for a symbol | Correct file, snapshot and source range when available |
| COIE-008 | Re-index a changed controlled test input snapshot | New snapshot context is distinguishable from the old one |
| COIE-009 | Remove or invalidate stale source facts in an updated graph | Old relationships are not presented as current snapshot truth |
| COIE-010 | Query beyond an allowed result/depth limit | Result states truncation and retains the correct context |

## 5.2 Canonical entity and identity tests

Verify that:

- language-specific declarations map into the agreed canonical `Symbol` contract;
- native kind and normalized kind are both preserved where applicable;
- overloaded symbols remain distinguishable;
- symbols with the same name in different files/scopes do not collide;
- symbol identity is deterministic in its intended repository/snapshot context;
- source ranges point into the correct source;
- aliases and imports are not misrepresented as definitions;
- file/module/package relationships preserve their agreed meanings.

The tests must not require every language to expose every possible symbol kind. Unsupported language constructs should remain explicitly unsupported instead of being silently converted into a different kind.

## 5.3 Relationship correctness tests

Required connector families include the supported subset of:

```text
CONTAINS
DEFINES
IMPORTS
EXPORTS
ALIASES
CALLS
REFERENCES
USES_TYPE
IMPLEMENTS
INHERITS_FROM
OVERRIDES
```

For each connector emitted by the controlled test input:

1. verify both endpoints exist;
2. verify source and target orientation;
3. verify the relationship has the correct semantic type;
4. verify source provenance is retained where supported;
5. verify identity and snapshot context are compatible;
6. verify the graph writer does not add duplicates on repeat ingestion.

Negative tests are important:

- same-named symbols must not be linked solely because names match;
- unresolved calls must not become confirmed `CALLS` edges;
- imported symbols must not automatically be treated as called;
- `REFERENCES` must not be silently upgraded to `CALLS`;
- dynamic or ambiguous constructs must preserve uncertainty.

## 5.4 Query and bounded traversal tests

Required cases:

- direct relationship lookup,
- callers/callees where resolvable,
- import relationships,
- bounded multi-hop path,
- cycles,
- duplicate paths to the same entity,
- maximum depth,
- maximum result count,
- timeout/limit exhaustion,
- unresolved endpoint,
- unsupported query capability,
- pagination with compatible original graph context.

Expected results:

- traversal is bounded;
- cycles do not cause unbounded traversal;
- canonical entities are deduplicated within the analysis context;
- supported paths preserve node/edge sequence and direction;
- a truncated path result is never labelled exhaustive;
- a timeout/transport failure is not reported as an empty successful query.

---

# 6. Language Adapter Conformance — Rust, TypeScript/JavaScript and Python

Each language adapter is tested independently first, then through the shared application pipeline.

## 6.1 Shared conformance contract

For each supported-language test input, test this same path:

```text
source test input
    ↓
language adapter
    ↓
canonicalized entities
    ↓
validated relationships
    ↓
FalkorDB persistence
    ↓
semantic query
    ↓
multi-hop impact evidence
    ↓
test candidate
```

## 6.2 Rust adapter

Required assertions:

- `.rs` files are identified as Rust;
- supported Rust declarations are extracted;
- supported imports and resolvable calls produce correct edges;
- `#[test]` declarations are discovered by TIE;
- source path/range and snapshot context survive graph persistence;
- unsupported macro/dynamic or unresolved constructs are reported honestly.

## 6.3 JavaScript adapter

Required assertions:

- `.js` and `.jsx` files in the fixture are classified correctly;
- supported declarations, imports/exports and resolvable calls are canonicalized;
- supported Jest/Vitest test declarations are discovered;
- test mappings preserve evidence strength;
- unsupported dynamic imports or unresolved calls do not become invented edges.

## 6.4 TypeScript adapter

Required assertions:

- `.ts` and `.tsx` files are identified correctly;
- supported interfaces, types, classes, functions and modules are represented using the canonical schema;
- type-only relationships remain distinct from runtime calls;
- supported Jest/Vitest tests are discovered;
- unsupported generic/dynamic/advanced resolution cases are reported honestly.

## 6.5 Python adapter

Required assertions:

- `.py` files are identified correctly;
- supported modules, classes, functions, imports and resolvable calls are represented canonically;
- pytest/unittest tests supported by the chosen static discovery subset are found;
- unsupported dynamic dispatch/import patterns remain unresolved where appropriate;
- no static mapping is promoted to actual execution or coverage evidence.

## 6.6 Cross-language conformance checks

The semantically equivalent test-input components need not have identical source lines or native syntax, but the contract assertions should align on:

- canonical entity semantics,
- containment/definition,
- supported imports/calls,
- test declaration,
- supported test target mapping,
- evidence and completeness handling.

The E2E should prove that CodeMind has one semantic investigation workflow across the three languages, not three unrelated hardcoded demos.

---

# 7. TIE — Test Intelligence Tests

TIE owns the Test Graph. It supplies test facts and evidence; it does not decide change risk.

## 7.1 Static discovery

Verify that TIE discovers supported test declarations in each language test input:

- Rust native test harness,
- a supported subset of Jest/Vitest for JavaScript/TypeScript,
- a supported subset of pytest/unittest for Python.

A discovered test must include enough provenance to identify its source and repository/snapshot.

## 7.2 Relationship semantic tests

The architecture distinguishes:

```text
DECLARED_BY
TARGETS
EXERCISES
COVERS
USES_FIXTURE
DEPENDS_ON
BELONGS_TO
```

Test the distinctions explicitly:

| Relationship | What the test must establish |
|---|---|
| `DECLARED_BY` | Where a test declaration is defined |
| `TARGETS` | Supported/inferred test-to-code target association |
| `EXERCISES` | Actual attributable runtime observation only |
| `COVERS` | Actual coverage evidence at the reported granularity only |
| `USES_FIXTURE` | Supported fixture association |
| `DEPENDS_ON` | Supported test dependency |
| `BELONGS_TO` | Test suite/group membership where supported |

Critical negative assertions:

- `TARGETS` is not proof that the test executed the target.
- `CALLS` in the application graph is not proof a test ran.
- a static discovery result is not a passing test result;
- do not create `EXERCISES` or `COVERS` without corresponding runtime/coverage evidence;
- missing mapping is not proof that no relevant test exists.

## 7.3 Test mapping tests

Check:

- correct target identity,
- compatible source snapshot,
- unresolved target representation,
- no mapping based only on a name coincidence when the resolution is ambiguous,
- correct provenance and source location,
- deterministic deduplication,
- cross-language behavior remains consistent.

## 7.4 Runtime execution boundary

The MVP performs static discovery. It does not run arbitrary untrusted repository tests to build the graph.

A test must verify the discovery path does not implicitly spawn test-runner commands. If external test/coverage evidence is supported, it must be labelled as supplied observation and retain provenance. Executing repository code is outside this hackathon test contract.

---

# 8. ChIE — Change Intelligence Tests

ChIE owns change-specific analysis and the Change Graph. Its findings must be derived from compatible CoIE/TIE inputs.

## 8.1 Direct and indirect impact

Given a deterministic graph built from controlled source input:

```text
Selected Symbol
   └── CALLS → Function A
                   └── IMPORTS → Module B
```

Verify that:

- one eligible relationship step is classified as direct impact;
- two or more eligible steps are classified as indirect impact;
- the selected origin entity is not mistakenly counted as a downstream affected entity;
- path order, relationship type and direction remain recoverable;
- results are deduplicated within the analysis context;
- multiple different paths to the same entity do not create duplicate entity counts;
- cycles terminate within the configured traversal rules.

## 8.2 Traversal bounds and incomplete inputs

Test:

- maximum traversal depth,
- maximum entity/result count,
- timeout,
- truncated paths,
- unresolved edges,
- missing target symbol,
- missing or degraded CoIE input,
- missing or degraded TIE input,
- incompatible graph/snapshot version,
- partial source indexing.

Expected behavior:

- retain valid results already established;
- report unresolved/truncated/incomplete information explicitly;
- do not upgrade a partial traversal to complete;
- reject or visibly report an incompatible/stale engine result;
- do not fabricate an impact result if the graph service is unavailable.

## 8.3 ChIE domain ownership tests

Verify that ChIE creates its own domain records as required:

```text
ChangeSet
ChangeRecord
AnalysisRun
ImpactAnalysis
RiskAssessment
Evidence
```

and the supported relationships include:

```text
CONTAINS_CHANGE
ANALYZES
PRODUCES
AFFECTS
HAS_RISK
RECOMMENDS
SUPPORTED_BY
```

Important assertions:

- `AFFECTS` points to the canonical CoIE entity;
- `RECOMMENDS` points to the canonical TIE test;
- ChIE does not clone canonical code/test entities into a competing identity;
- rerunning the same operation does not duplicate graph records silently;
- unpublished/invalid analysis output does not replace an authoritative completed result.

## 8.4 Risk semantics

Risk is not a calibrated 0–100 numeric score in the MVP.

Test the five factor families independently:

1. structural exposure;
2. validation evidence;
3. historical instability only when supported evidence exists;
4. explicit component criticality/sensitive-boundary policy;
5. supported change characteristics.

Verify the separate dimensions:

```text
Impact
Risk
Confidence
Completeness
Review severity
```

Required negative assertions:

- unknown evidence does not imply low risk;
- no tests discovered does not automatically mean no risk;
- missing history is not proof of historical safety;
- high impact does not automatically become high risk without considering validation/evidence;
- low confidence cannot be used to silently lower the risk category;
- no fabricated percentage is emitted by a deterministic risk implementation;
- every material risk factor is traceable to input evidence/policy.

## 8.5 Test-impact selection

Given a set of affected entities and TIE associations, verify that:

- eligible associated tests become recommendations;
- unsupported/unresolved mappings remain qualified candidates;
- tests reached through multiple affected entities appear once;
- each recommendation retains its reason/evidence references;
- priority is grounded in explicit evidence/policy;
- actual coverage/execution is not claimed from static discovery alone.

## 8.6 Evidence contract

Every material ChIE conclusion must preserve the references needed to explain it:

```text
finding
  ↓
affected entity
  ↓
relationship path
  ↓
test evidence/candidate
  ↓
risk factor
  ↓
limitation/completeness
```

The UI must be able to show why the engine derived a conclusion. If no supporting path or evidence exists, the result must say so rather than inventing one.

---

# 9. Canonical Graph and FalkorDB Integration

FalkorDB must be tested as a real dependency, not only mocked.

## 9.1 Persistence

Test that:

1. the controlled-source ingestion writes canonical entities and relationships;
2. a separate Rust query reads them back;
3. the read result matches canonical expectations;
4. a process restart does not lose data when the persistent volume is intended to retain it;
5. resetting demo data requires an explicit reset action.

## 9.2 Idempotency

Run the same ingestion operation twice.

Verify:

- entity count is stable;
- relationship count is stable;
- relationship direction/type is stable;
- the same symbol does not gain duplicate identity;
- no stale relationship is reintroduced;
- memory/handoff records are not affected by code-graph reseeding.

## 9.3 Isolation

Seed two distinct workspace/repository contexts and attempt cross-context queries.

Expected:

- Workspace A query returns only authorized A data.
- Workspace B data is not exposed to A.
- cross-workspace memory lookup is denied.
- query result caching (if any) cannot return another workspace's result.
- repository/snapshot context cannot be overridden by LLM-supplied fields.

## 9.4 Real graph traversal proof

Create/seed a known connected chain from actual test-input source. Query from the selected symbol through FalkorDB.

Verify that:

- all returned nodes/edges are present in FalkorDB;
- at least one multi-hop path is found by an actual graph query/traversal;
- the ChIE impact set is derived from that returned path;
- removing/altering the relevant relationship in a controlled test changes the expected impact/path result;
- results are not supplied by a hardcoded application-side list.

The last assertion is the strongest test that FalkorDB genuinely shapes the answer.

## 9.5 Adapter boundary

Verify that:

- Cypher resides in the Rust FalkorDB adapter/query implementation;
- domain engines call the Graph Storage Port;
- Next.js does not connect to FalkorDB;
- UI components do not import a database client;
- the LLM receives only semantic tool definitions/results, never database credentials or a direct query channel.

---

# 10. gRPC and Protobuf Contract Tests

The required cross-boundary path is:

```text
Agent Runtime
    ↓ gRPC / Protobuf
Rust service
    ↓
canonical result
    ↓ gRPC / Protobuf
Agent Runtime
```

## 10.1 Serialization preservation

For representative repository, query, ChIE, memory and operation RPCs, verify round-trip preservation of:

- operation ID,
- request/trace IDs,
- repository ID,
- source snapshot and graph version,
- engine and stage,
- typed result/outcome,
- evidence references,
- provenance,
- completeness,
- freshness,
- pagination/truncation,
- diagnostics/limitations,
- safe error code,
- sequence/event identity.

Optional fields that are unavailable should remain unavailable/unknown, not silently default to a falsely successful value.

## 10.2 Invalid request tests

Test malformed or unsupported:

- missing required repository/snapshot context,
- invalid enum value where the protocol can receive one,
- malformed entity reference,
- incompatible graph context,
- invalid pagination/cursor context,
- invalid memory/handoff payload,
- unauthorized workspace scope.

Expected behavior is a predictable typed failure, not a panic, crash or fake empty result.

## 10.3 Contract evolution

Generated bindings must be reproducible from `.proto` sources.

The contract test should fail when a change accidentally removes or changes a field the Runtime relies on. Do not hand-patch generated bindings to make a broken contract compile.

## 10.4 Service failure and timeout

Verify:

- unavailable Rust service maps to a safe application error;
- timeout is distinguishable from successful empty output;
- cancellation/retry preserves operation correlation and idempotency;
- stale result/snapshot is rejected or explicitly reported.

---

# 11. Agent Runtime, Semantic Tools and Guardrails

All 11 initial semantic tools must pass the same registry-driven path:

```text
LLM tool call
   ↓
registry lookup
   ↓
schema validation
   ↓
authorization/context
   ↓
limits + duplicate-call policy
   ↓
gRPC to Rust
   ↓
canonical result validation
   ↓
bounded LLM/UI projection
```

## 11.1 Tool input validation

The registry contract test must assert that the MVP exposes exactly these approved semantic tools: `get_repository_context`, `search_code`, `get_symbol`, `query_code_relationships`, `get_historical_changes`, `get_test_intelligence`, `analyze_change`, `get_impact`, `get_risk`, `get_test_impact`, and `get_evidence`. Assert that `simulate_change` and private Rust engine functions are not exposed as LLM-callable tools. Tool schemas and runtime validators must derive from the same registry definition.

For each registered tool, test:

- valid minimum input;
- missing required field;
- wrong field type;
- invalid entity identifier;
- oversized input;
- unsupported enum/mode;
- mismatched repository or snapshot;
- unknown/unregistered tool name.

Invalid input must be rejected before a Rust invocation where appropriate.

## 11.2 Authorization and context

Test that the LLM cannot manufacture or override:

```text
user_id
workspace/tenant scope
repository_id
operation_id
permissions
evidence access
```

The runtime binds these to trusted application state. A tool request for a repository not authorized for that operation must be rejected.

## 11.3 Guardrails

Test all centralized controls:

- total tool-call budget,
- total operation deadline,
- per-tool timeout,
- result-size limit,
- pagination limit,
- traversal limit,
- duplicate-call handling,
- loop exhaustion,
- evidence-sufficiency enforcement,
- safe error projection.

When a budget is exhausted:

```text
stop new calls
    ↓
preserve valid findings/results
    ↓
mark result partial/incomplete when appropriate
    ↓
explain the limitation
```

Do not fabricate completion to make the agent loop terminate neatly.

## 11.4 No arbitrary Cypher or source execution

Negative tests must confirm:

- unknown/raw Cypher is not exposed as a tool;
- a model cannot select arbitrary graph labels/relationship types or bypass approved query bounds;
- no arbitrary shell/repository command tool exists;
- static discovery does not execute tests from the repository;
- repository source, comments, README/docs, test names/descriptions, history/issues, retrieved memory, graph properties, tool outputs and diagnostic logs are treated as untrusted content, not higher-priority instructions;
- prompt-injection text embedded in any such content cannot override tool allowlists, authorization, scope, query bounds, memory promotion rules, secret handling or execution restrictions;
- an instruction embedded in a source comment, README, test name, memory record or log cannot cause the runtime to bypass tool validation or reveal credentials.

## 11.5 Result-projection honesty

Feed a canonical result with specific nodes, paths, risk factors and limits into the projection pipeline.

Verify:

- the UI projection includes only the supplied authoritative data;
- the LLM projection is bounded and preserves the important evidence/limitations;
- no invented file, symbol, count, relationship, test, risk factor or graph path is added by result-transforming code;
- UI data is not parsed back out of LLM prose;
- TOON is used only as the LLM-context encoding and never becomes the authoritative result or UI contract.

## 11.6 LLM provider failure

Simulate an unavailable provider, timeout and malformed tool-call response.

Verify:

- safe error/status is shown;
- existing canonical graph data remains valid;
- an LLM failure is not reported as a FalkorDB or Rust failure unless that component actually failed;
- the UI does not render fabricated final findings;
- the selected local data path remains available for the local fallback demo.

## 11.7 Type B LLM-result encoding: `flat` → TOON

This is a required contract test because successful Type B results are encoded for the LLM in the JavaScript semantic-tool layer. The canonical Rust result must remain structured; the UI gets its own structured projection. Use the `flat` npm package for the reversible flattening transform and the official `@toon-format/toon` package for TOON encoding. Do not add a custom JSON-to-TOON serializer.

Required path:

```text
Rust Type B canonical structured result
  ↓ parse + validate
bounded LLM projection
  ↓ flat (reversible transform; preserve record arrays)
flattened LLM projection
  ↓ verify unflatten structural equality
@toon-format/toon encode
  ↓
TOON text as the LLM-facing tool result
```

Test the following cases against representative CodeMind payloads:

- shallow objects and nested objects at several depths;
- primitive arrays, arrays of records, nested arrays, empty arrays, mixed arrays, and arrays containing nested objects;
- empty objects, nulls, booleans, numbers, and strings;
- literal keys containing dots, delimiter characters, numeric-looking segments, and keys that would collide with a generated flattened path;
- impact paths, findings, affected files, nested test evidence, source/snapshot references, diagnostics, and bounded partial/truncated results;
- large but permitted Type B tool results at the actual configured size limit.

Assertions:

- flatten followed by unflatten is structurally equal to the exact LLM projection, including key identity, value types, array boundaries/order, empty collections and nested record fields;
- arrays of records remain arrays where practical so TOON can encode uniform records compactly; do not flatten records into `array.0.*` paths merely for convenience;
- the encoded TOON decodes back to the intended flattened JSON-model projection according to the package's supported API;
- the original canonical result is not mutated, and the UI projection does not use the flattened/TOON value;
- a successful Type B LLM-facing result always traverses `flat` then TOON; no Type A graph-build report, raw Rust function, operation event, diagnostic log or UI DTO is sent through that model-facing result encoder as if it were a Type B result;
- when safe reversible flattening cannot be proven, the tool returns an explicit formatting/contract error. It must not silently bypass `flat`, fall back to direct TOON, or send JSON to the LLM as a fallback; the validated canonical result remains available to the UI and safe diagnostics.

### Token-efficiency benchmark

Use equivalent input fields and values to compare four representations:

1. compact JSON;
2. direct/native TOON;
3. flattened JSON;
4. flattened TOON.

Measure actual token counts using one documented tokenizer consistently, successful retrieval of critical fields (path steps, relationship types/directions, test candidates, evidence references, snapshot/version and limitations), and structural/data preservation. Include impact paths, findings, affected files, nested test evidence, and mixed/partial/truncated results. Store measurements with the candidate commit/environment. Do not claim a universal token saving; flattened TOON is retained as the chosen encoding path, while the benchmark measures its actual trade-offs for CodeMind.

---

# 12. Operation Lifecycle and Progress Tests

Long-running repository preparation and analysis are operations, not just one synchronous HTTP response.

## 12.1 Lifecycle transitions

Test valid transitions including:

```text
created → running → waiting → completed
created → running → failed
running → cancellation_requested → cancelled
running → completed_with_limitations
```

Reject invalid regressions such as a terminal completed operation becoming running again because an old event arrived late.

Keep these dimensions separate:

- lifecycle: running/waiting/completed/failed/cancelled;
- outcome: success/degraded/failure/cancelled;
- completeness: complete/partial/truncated/unknown.

## 12.2 Event identity and ordering

Verify that progress events have stable identity and operation correlation, with monotonic sequence values for an operation.

Test:

- duplicate event delivery;
- stale/out-of-order event delivery;
- event with wrong operation ID;
- event from an incompatible/stale graph version;
- sequence gap;
- terminal event followed by an older running event.

Expected:

- duplicate/stale events do not regress UI state;
- unrelated-operation events are ignored/rejected;
- the current authoritative operation state wins over outdated event replay;
- unknown sequence gaps are handled through recovery rather than guessed progress.

## 12.3 Two streaming boundaries

The required path is:

```text
Rust typed progress
    ↓
gRPC progress stream
    ↓
Agent Runtime
    ↓
Next.js SSE
    ↓
browser
```

Test that:

- browser receives authorized, safe progress;
- Rust credentials and unrestricted diagnostics are never sent to the browser;
- UI progress is derived from typed progress events rather than log scraping;
- graph-publication events are distinguishable from diagnostic logs and user-facing messages.

## 12.4 Disconnect and reconnect

Test:

1. Start a long-running operation.
2. Capture its operation ID and latest event sequence.
3. Disconnect the browser/SSE client.
4. Allow the operation to proceed.
5. Reconnect using the same operation identity and last-known sequence.
6. Recover authoritative current state and relevant retained events.

Expected:

- the operation did not fail solely because the client disconnected;
- reconnect does not start a second operation;
- current stage/status is restored;
- duplicates do not rewind progress;
- final result matches the original operation.

## 12.5 Cancellation, retries and operation duplication

Test:

- cancellation request remains pending until the service confirms cancellation;
- terminal cancellation is represented explicitly;
- retry retains the required correlation/idempotency data;
- a repeated submit does not silently create duplicate side effects;
- a stale engine result cannot replace a newer graph/analysis result.

---

# 13. Memory Tests — Track 02

Memory is useful only if it is persistent, scoped, evidence-linked and checked against the current graph.

## 13.1 Persistent entities

Required persisted types:

- `ConversationSession`
- `ConversationTurn`
- `ReviewEpisode`
- `ReviewFinding`
- `SemanticMemory`
- `ProcedureMemory`
- `Handoff`
- `EngineReference`

`ReviewFeedback` is optional under time pressure.

## 13.2 Write-pipeline validation

Test the controlled path:

```text
review
  ↓
episode
  ↓
finding
  ↓
memory candidate
  ↓
schema validation
  ↓
authorization
  ↓
scope validation
  ↓
provenance/evidence validation
  ↓
promotion policy
  ↓
Rust persistence
  ↓
FalkorDB
```

Expected:

- malformed records are rejected;
- a model proposal is not automatically trusted memory;
- evidence references remain attached;
- rejected candidates are not retrieved as validated memory;
- secrets, credentials, unrestricted source files and unbounded tool outputs do not get persisted as memory.

## 13.3 Fresh-session persistence

Core sequence:

```text
Session A
  ↓
complete investigation
  ↓
persist ReviewEpisode + ReviewFinding
  ↓
persist SemanticMemory + ProcedureMemory
  ↓
end session / restart application
  ↓
Session B
  ↓
retrieve authorized memory
```

Expected:

- records still exist after the new session/restart;
- retrieval is performed through the storage/runtime path;
- results are not merely reinserted into a new prompt from frontend memory;
- retrieved memory retains its scope, provenance and evidence/source references.

## 13.4 Lifecycle versus freshness

Lifecycle values:

```text
candidate
validated
rejected
superseded
```

Freshness values:

```text
fresh
stale
unknown
```

Test combinations including `validated + stale`.

Expected:

- a validated old record may still be stale;
- a candidate is not treated as validated;
- unknown freshness is not treated as fresh;
- a stale memory is marked and cannot override current CoIE/TIE/ChIE facts.

## 13.5 Current evidence validation

Run memory retrieval against:

- compatible unchanged snapshot;
- a newer source snapshot;
- a symbol whose identity no longer resolves;
- a changed graph relationship;
- missing TIE evidence;
- incomplete current graph.

Expected:

- applicability is evaluated;
- memory is used only when authorized and suitably applicable;
- stale or unverified memory is either omitted from authoritative reasoning or shown as historical context with a clear qualification;
- old counts or paths are never presented as current facts without revalidation.

## 13.6 Memory scope isolation

Test all three approved scopes:

```text
user
repository/project
team/tenant
```

Cases:

- same user, authorized memory;
- different user, private memory;
- same repository, authorized project memory;
- unrelated repository;
- authorized team memory;
- non-member tenant/team;
- attempted implicit scope promotion.

Expected:

- no implicit scope inheritance or promotion;
- access is checked in both Agent Runtime and Rust persistence;
- unauthorized memory is not returned in a “stale” or “empty” fallback that leaks existence or content.

## 13.7 Memory affects a later investigation

A valid positive test must show:

1. memory was persisted in Session A;
2. Session B retrieved the record from persistence;
3. its applicability was checked against current evidence;
4. the current investigation had access to the applicable memory;
5. at least one explainable aspect of the later investigation used that memory;
6. current graph facts remained authoritative.

It is not enough to assert that the database contains a memory record.

---

# 14. Handoff Tests — Track 02 Coordination

A handoff is a persistent structured work packet, not a string message or in-memory function argument.

## 14.1 Schema and provenance

Verify required fields are present and validated where applicable:

```text
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

A receiving stage must reject or explicitly qualify handoffs with invalid schema, incompatible snapshot, invalid authorization or unresolved required references.

## 14.2 Lifecycle

Test:

```text
pending → claimed → consumed
                  ↘ expired
```

The exact implementation may support additional failure/recovery metadata, but it must not silently treat an expired, invalid or wrong-scope handoff as consumed.

Operation lifecycle is separate:

```text
running / completed / failed / cancelled / degraded
```

A handoff being consumed does not by itself mean the operation succeeded.

## 14.3 Idempotency and concurrency

Test:

- same handoff write repeated;
- same handoff claim repeated by the same consumer;
- concurrent claim by two consumers;
- consume repeated after success;
- retry after an interrupted claim;
- an old payload attempting to overwrite a newer version;
- duplicated progress/transport delivery.

Expected:

- stable handoff identity and idempotency rules apply;
- only the allowed consumer can claim/consume;
- no silent overwrite;
- repeated delivery does not create duplicate investigation work;
- interrupted processing can be recovered according to the persisted state.

## 14.4 Receiving-stage proof

The end-to-end test must demonstrate:

```text
Impact analysis stage
    ↓
persist handoff through Rust/FalkorDB
    ↓
receiving test-investigation stage starts
    ↓
receiving stage loads handoff from persistence
    ↓
validates snapshot/scope/provenance
    ↓
uses handoff to continue investigation
```

The test must fail if the receiving stage only uses a TypeScript object that was passed directly from the sending stage without persistence and retrieval.

---

# 15. Next.js, Route Handler and Chat UI Tests

## 15.1 Route Handlers remain thin

For repository, agent, analysis, operation-event and memory endpoints, test:

- request schema validation,
- authentication/authorization context,
- delegation to the correct server module,
- safe structured response,
- safe status/error mapping,
- streaming headers and cancellation cleanup where applicable.

A Route Handler test should not require it to duplicate the Rust engine's graph reasoning.

## 15.2 UI components use structured fixtures

Test components against fixed result fixtures for:

- summary,
- impact,
- risk,
- affected files,
- test recommendations,
- evidence,
- graph paths,
- memory inspector,
- handoff status,
- engine accordion/progress,
- completeness/limitation notice;
- compact affected-file/finding/test-candidate tables or result lists.

Verify that tables/lists render from structured canonical/UI projections, retain row-level relationship/evidence status, remain usable at narrow widths, and support keyboard navigation/focus. Never make core component tests depend on a live LLM response.

## 15.3 Required UI states

Every major interaction/state component should be tested for applicable versions of:

```text
Empty
Loading
Running
Waiting
Complete
Partial
Truncated
Unavailable
Failed
Retrying
Disconnected
Reconnected
Cancelled
```

Verify that:

- a partial analysis is not styled as a fully successful one;
- a truncated traversal displays its limitation;
- “no results” is different from “service unavailable”;
- risk labels and counts come from structured data;
- evidence can be expanded and matches the result;
- engine accordions show actual typed progress;
- a completed operation does not keep showing an engine as running;
- stale/out-of-order progress cannot regress the UI;
- the final result does not display fake percentage progress;
- there is no placeholder/hardcoded demo result masquerading as a Rust result.

## 15.4 Chat tabs and structured result continuity

Verify that the main investigation workspace can switch among:

```text
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

All views must point at the same analysis/operation context. Switching tabs must not create a new operation or silently query a different snapshot.

---

# 16. Critical End-to-End Scenarios

The following are the core hackathon E2E tests. They must run through the application, not only through direct Rust functions.

## 16.1 E2E-01 — Rust Track 01 investigation

**Purpose:** Prove the complete connected-data analysis path.

**Setup:** Clean controlled-input workspace, Rust source test input ingested into FalkorDB, known symbol and test relationships available.

**Steps:**

1. Open CodeMind.
2. Open the selected local polyglot workspace.
3. Select the known Rust symbol.
4. Ask: **“What could this change affect?”**
5. Observe LLM planning/tool selection.
6. Observe CoIE stage/progress.
7. Observe TIE stage/progress.
8. Observe ChIE stage/progress.
9. Wait for ChIE's typed graph-publication/completion event.
10. Observe the LLM reasoning state.
11. Inspect summary, impact, risk and test recommendation results.
12. Open at least one evidence-backed graph path.
13. Open the supporting evidence.

**Expected:**

- request is bound to the correct repository/snapshot;
- semantic tools are invoked through the registry;
- persisted FalkorDB relationships drive at least one multi-hop result;
- direct versus indirect impact is correct for the fixture;
- risk factors correspond to actual supplied data/policy;
- test recommendations trace to TIE's supported associations;
- the selected path and result evidence are inspectable;
- outcome/completeness/limitations are preserved;
- LLM wording may vary, but it may not invent facts absent from the canonical result.

## 16.2 E2E-02 — TypeScript/JavaScript Track 01 investigation

Repeat E2E-01 against the selected TypeScript/JavaScript source input.

**Expected:**

- the correct adapter and parser run;
- the canonical entity/relationship contract is equivalent where the source supports equivalent constructs;
- supported Jest/Vitest-style test candidates are discovered;
- all result/evidence/context assertions pass;
- no relationships are copied from the Rust source input or hardcoded to make the result look identical.

## 16.3 E2E-03 — Python Track 01 investigation

Repeat E2E-01 against the selected Python source input.

**Expected:**

- the correct Python adapter runs;
- canonical entities, supported imports/calls and test candidates are present;
- unresolved dynamic relationships remain unresolved;
- impact and test recommendations follow actual persisted graph relationships;
- the same result contract and UI states work as for Rust.

## 16.4 E2E-04 — FalkorDB is materially responsible for the answer

**Purpose:** Explicitly prove that CodeMind is not an LLM answer with a graph illustration added afterward.

**Steps:**

1. Run a known controlled-source investigation and record its canonical path/impact.
2. In an isolated test graph, remove or disable one relationship required for that path, or use a second controlled fixture variant without that edge.
3. Repeat the exact semantic query.
4. Compare the canonical result and evidence path.

**Expected:**

- the affected path/result changes in accordance with the graph;
- the application does not return the earlier hardcoded path;
- the result reflects the graph version used;
- if the result becomes incomplete/unresolved, the UI says so.

This test must use real FalkorDB behavior. A mock-only equivalent does not satisfy it.

## 16.5 E2E-05 — Track 02 persisted handoff

**Purpose:** Prove that one stage writes a real handoff and another stage reads and uses it.

**Steps:**

1. Start a symbol-first change investigation.
2. Run the real CoIE → TIE → ChIE path.
3. Persist the review episode and finding.
4. Persist the structured handoff through the Rust persistence boundary.
5. Finish the session or restart the application process.
6. Start/open the receiving test-investigation stage.
7. Retrieve the handoff from storage.
8. Validate its scope, snapshot, graph references and schema.
9. Claim/consume it through the approved lifecycle.
10. Continue the receiving stage using the handoff plus current graph/test evidence.
11. Inspect the UI handoff status.

**Expected:**

- the handoff exists in FalkorDB-backed persistence;
- the receiving stage reads it back from the persistent store;
- the receiving stage does not rely only on a same-process in-memory object;
- stable handoff identity/idempotency are preserved;
- the consumed status is genuine;
- the resulting test evidence remains current and correctly qualified.

## 16.6 E2E-06 — Track 02 memory survives a fresh session

**Steps:**

1. In Session A, complete a meaningful investigation.
2. Persist `ReviewEpisode` and `ReviewFinding`.
3. Persist one qualified `SemanticMemory`.
4. Persist one reusable `ProcedureMemory`.
5. End Session A; restart the application or begin a genuinely fresh session.
6. In Session B, request a related investigation.
7. Retrieve authorized memory from persistent storage.
8. Validate its provenance and applicability against the current graph/snapshot.
9. Observe whether it is applicable, stale, or unknown.
10. Inspect which memory was used and how it informed the investigation.
11. In a separate controlled snapshot/graph variant, retrieve a previously validated memory whose referenced relationship or snapshot has changed.
12. Confirm the memory is explicitly marked stale/less applicable and that current graph evidence wins.

**Expected:**

- memory survives the session boundary and, for the restart scenario, the process boundary;
- retrieval is not hidden prompt replay or browser local state;
- current evidence is consulted;
- applicable memory influences the later investigation in an explainable way;
- stale/unknown memory does not override current graph facts;
- the UI makes memory validation/applicability visible;
- the controlled stale-memory scenario is deterministic, reproducible, and does not destructively mutate the positive demo fixture;
- a memory's `validated` lifecycle state does not imply it is currently fresh/applicable.

## 16.7 E2E-07 — Repeatability

Run the same investigation more than once.

Verify:

- no duplicate canonical code entities/relationships;
- no duplicate tests;
- no duplicate ChIE records that are supposed to be idempotent under the same operation/idempotency key;
- no duplicate handoff side effects;
- memory remains structurally valid;
- evidence references remain valid;
- the same source snapshot produces stable canonical assertions.

## 16.8 E2E-08 — Offline fallback

**Steps:**

1. Disable/deny external network access or use a controlled environment where the external provider is unavailable.
2. Start FalkorDB, Rust and Next.js locally.
3. Open the selected local polyglot workspace.
4. Run the preloaded graph preparation/investigation route that is supported by the implementation.
5. Confirm graph-backed impact and evidence can be shown without fetching a new repository.

**Expected:**

- no public GitHub access is required for the core demo;
- no mock impact response is substituted;
- if LLM reasoning is unavailable, the UI distinguishes that failure from successful graph work instead of fabricating a full final answer;
- use a preloaded controlled test input as actual input data, not prewritten final results.

---

# 17. Failure, Partial, Security and Recovery Scenarios

Day 4 must deliberately exercise failure paths. Do not merely infer them from passing unit tests.

| ID | Scenario | Test action | Required behavior |
|---|---|---|---|
| FAIL-01 | No internet | Disable external connectivity | Local controlled test input path remains available; external import fails honestly |
| FAIL-02 | Public repository not found | Submit a known invalid/nonexistent public URL in a controlled test | Clear repository-not-found/access error; no graph marked ready |
| FAIL-03 | Repository access denied | Use an inaccessible repository/authorization fixture | Access denied; do not expose protected content |
| FAIL-04 | Invalid URL | Submit malformed URL and disallowed scheme/host form | Reject before source acquisition; no arbitrary fetch |
| FAIL-05 | Rust service unavailable | Stop service or point test client to an unavailable endpoint | Safe service-unavailable state; no invented result |
| FAIL-06 | FalkorDB unavailable | Stop or isolate the database in a controlled test | Graph operations fail safely; no false success |
| FAIL-07 | LLM provider failure | Use invalid test credentials or a controlled provider failure | Explanation failure is distinct from successful/failed graph processing |
| FAIL-08 | Partial indexing | Inject a file parsing/extraction failure | Partial/degraded status and diagnostic; surviving facts remain valid |
| FAIL-09 | Unresolved relationship | Use the unresolved controlled test-input edge | Preserve unresolved status and limitation; do not fabricate an edge |
| FAIL-10 | Unsupported syntax | Use a construct outside the declared adapter subset | Explicit unsupported diagnostic; no fabricated graph fact |
| FAIL-11 | Traversal limit | Lower test-only bound to force truncation | Result marked truncated/partial; no complete claim |
| FAIL-12 | Tool timeout | Force a timeout in a test tool/RPC | Safe timeout result; valid prior findings retained |
| FAIL-13 | Tool budget exhausted | Use a deliberately bounded test budget | Stop new calls; preserve valid results; mark incomplete as needed |
| FAIL-14 | Invalid tool arguments | Pass malformed tool input | Rejected before unsafe execution |
| FAIL-15 | Unauthorized repository/tool call | Bind operation to workspace A and request B | Denied; no B data returned |
| FAIL-16 | Arbitrary Cypher attempt | Attempt to call a non-existent raw-query tool or inject a graph query into input | No database bypass; only registered semantic tools run |
| FAIL-17 | Arbitrary shell execution attempt | Put command-like instructions in a user prompt or source file | No arbitrary repository command is executed |
| FAIL-18 | Secret in source/prompt | Add controlled secret test input to content | Secret is not copied into persistent memory or unsafe browser diagnostics |
| FAIL-19 | Browser/SSE disconnect | Disconnect during a running operation | Operation continues independently; UI can reconnect |
| FAIL-20 | Duplicate progress event | Replay the same event ID/sequence | UI state does not regress or duplicate the visible stage |
| FAIL-21 | Out-of-order progress event | Deliver stale lower-sequence event after a newer one | Latest authoritative state remains visible |
| FAIL-22 | Duplicate operation request | Repeat submission with same idempotency context | No silent duplicate side effects |
| FAIL-23 | Stale engine result | Return a result bound to an incompatible snapshot | Reject or explicitly report stale/incompatible result |
| FAIL-24 | Stale memory | Change snapshot/relationship after creating memory | Memory marked stale/limited; current graph wins |
| FAIL-25 | Cross-scope memory read | Attempt user/project/team scope violation | Deny at Runtime and Rust persistence boundaries |
| FAIL-26 | Concurrent handoff claim | Two consumers claim the same handoff | Only valid claim proceeds; no silent double consumption |
| FAIL-27 | Invalid handoff schema | Remove required metadata or corrupt reference | Reject; handoff is not treated as consumed |
| FAIL-28 | Process restart | Restart application/service at a tested operation boundary | Persistent repository/memory/handoff state remains correct; operation status recovered according to persistence design |
| FAIL-29 | Empty successful query | Query a supported symbol with no matching related entities | Accurate empty result, not an infrastructure error |
| FAIL-30 | No TIE mapping | Use a supported target with no established test mapping | “No mapping established”/limitation; not “no tests exist” unless proven |

### Failure rehearsal guidance

Use controlled injection in local/staging tests. Do not damage the final persisted demo state to simulate failures. Prefer an isolated graph, environment override, test adapter or controlled service stop, then restore the known-good environment and rerun the main smoke test.

---

# 18. Day 4 — Detailed Verification Schedule

Day 4 is reserved for full verification, reliability fixes, documentation and submission. Do not add a new major feature.

The order below is deliberate: establish a clean environment first, prove the three-language graph pipeline, prove both tracks, then test failures and record the demo.

## Phase 0 — Freeze and record the candidate

Before running tests:

- [ ] Declare feature freeze.
- [ ] Record the candidate commit SHA.
- [ ] Confirm `AGENTS.md`, `architecture.md`, `implementation-plan.md`, `testing.md`, `design-system.md` and `demo.md` refer to the same architecture/scope.
- [ ] Confirm there are no known uncommitted changes that could invalidate the test record.
- [ ] Confirm `.env.example` contains names/placeholders only, not real secrets.
- [ ] Confirm the controlled demo/test inputs and expected graph data are version controlled.
- [ ] Record any already-known limitation; do not conceal it during the final run.

**Output:** A clear candidate version and a known limitation list.

## Phase 1 — Clean-start environment check

1. Start FalkorDB using Docker Compose.
2. Verify FalkorDB is healthy and its persistent volume is mounted.
3. Start the Rust CodeMind Engine.
4. Verify its startup/configuration output contains no credentials.
5. Start Next.js.
6. Verify the browser can open the app and the Route Handler can reach Agent Runtime/Rust.
7. Run Protobuf generation check; confirm generated bindings are reproducible.
8. Run the relevant static checks/builds.

Checklist:

- [ ] FalkorDB reachable.
- [ ] Rust service reachable through gRPC.
- [ ] Next.js starts.
- [ ] Environment validation rejects missing required config clearly.
- [ ] No secret is printed or committed.
- [ ] The graph volume persists across a controlled restart.
- [ ] UI is not directly connected to FalkorDB or Rust internals.

**Stop condition:** If the environment is not stable, do not proceed to the demo rehearsal. Fix the base service/contract issue first.

## Phase 2 — Run fast tests

Run the project's configured commands for:

1. Rust unit/component tests.
2. TypeScript type checking/linting and server-module tests.
3. Protobuf generation/contract check.
4. UI component tests.
5. Adapter conformance and canonical graph tests.
6. Memory/handoff unit tests.

The final root scripts should provide repeatable wrappers. If a wrapper is missing, run the corresponding direct workspace command and add/fix the wrapper after the test itself works. Do not invent a passing result just because a script is absent.

- [ ] Rust test suite passes.
- [ ] Web typecheck/lint/test suites pass.
- [ ] Generated Protobuf bindings are consistent.
- [ ] Adapter conformance passes.
- [ ] Canonical graph tests pass.
- [ ] Memory/handoff unit tests pass.

## Phase 3 — Reset and seed a known controlled graph

1. Use the explicit test/demo reset path only where required.
2. Seed the selected local polyglot test repository.
3. Run CoIE extraction and graph persistence.
4. Run TIE static test discovery and supported test mappings.
5. Verify graph data in FalkorDB using the supported Rust semantic query path.
6. Save/log the test-input snapshot ID and graph versions used for subsequent tests.

Checklist:

- [ ] Rust source input entities and relationships match expected source assertions.
- [ ] TypeScript/JavaScript source input entities and relationships match expected source assertions.
- [ ] Python source input entities and relationships match expected source assertions.
- [ ] Supported tests are discovered.
- [ ] `TARGETS` is not treated as `COVERS` or `EXERCISES`.
- [ ] Unresolved/unsupported fixture cases stay qualified.
- [ ] Repeating ingestion does not create duplicate canonical graph facts.
- [ ] No test-input result is hardcoded into the UI.

## Phase 4 — Complete three-language end-to-end investigation

Run the same semantic user journey once for each language.

### Rust

```text
Open workspace
 → select known Rust symbol
 → ask “What could this change affect?”
 → LLM selects registered semantic tools
 → CoIE facts queried from FalkorDB
 → TIE test intelligence queried
 → ChIE computes impact/risk/test impact/evidence
 → Change Graph published
 → typed completion event received
 → Runtime retrieves result
 → LLM explains result
 → UI renders structured components
```

### TypeScript/JavaScript

Run the equivalent journey on the TypeScript fixture.

### Python

Run the equivalent journey on the Python source input.

For each run:

- [ ] Repository and snapshot context are correct.
- [ ] The correct language adapter ran.
- [ ] At least one genuine multi-hop graph path is returned where the fixture defines one.
- [ ] Direct/indirect impact classification matches the expected graph path.
- [ ] Test recommendations reference actual TIE test nodes.
- [ ] Evidence/path references resolve.
- [ ] Risk dimensions remain independent and evidence-backed.
- [ ] Limits and unresolved edges are represented honestly.
- [ ] LLM explanation has no unsupported file/symbol/count/path/test.
- [ ] All results shown in the UI belong to the same analysis context.

## Phase 5 — Explicit FalkorDB dependency proof

1. Run the known investigation against the seeded fixture.
2. Save the structured result/path.
3. Change or remove a relevant relationship in an isolated test graph/fixture variant.
4. rerun the same query against that graph.
5. compare canonical affected sets and evidence paths.

- [ ] The result changes in a way explained by the relationship change.
- [ ] No application-side hardcoded list reproduces the old path.
- [ ] Graph version/snapshot remains visible and correct.
- [ ] Any resulting incomplete/no-path condition is reported honestly.

This phase is essential: it demonstrates that FalkorDB is the reasoning substrate, not a decorative graph widget.

## Phase 6 — Track 01 complete rehearsal

Use the exact planned story:

```text
Open CodeMind
  ↓
open selected local polyglot workspace
  ↓
select Rust symbol
  ↓
ask “What could this change affect?”
  ↓
watch LLM planning
  ↓
watch CoIE → TIE → ChIE
  ↓
inspect impact
  ↓
inspect risk
  ↓
inspect test recommendations
  ↓
open evidence
  ↓
trace graph path
  ↓
read final explanation
```

- [ ] The flow works from the actual welcome/home entry.
- [ ] The execution timeline reflects actual typed progress.
- [ ] Engine accordions expand and show their actual progress records.
- [ ] ChIE completion is tied to publication/completion, not a timer.
- [ ] LLM reasoning begins after the result is available.
- [ ] Structured result tabs show consistent data.
- [ ] No demo-only hardcoded metrics are visible.
- [ ] The complete Track 01 flow is repeatable.

## Phase 7 — Track 02 persistence, handoff and fresh-session rehearsal

1. In Session A, complete the Track 01 investigation.
2. Verify review episode/finding persistence.
3. Verify one semantic memory and one reusable procedure are persisted.
4. Verify a structured handoff is persisted.
5. End the session and restart a process or otherwise prove state is not page-local.
6. Begin Session B.
7. Retrieve the handoff from storage in the receiving stage.
8. Verify it is claimed/consumed with the approved lifecycle.
9. Retrieve memory from storage.
10. Check scope/provenance/current graph compatibility.
11. Demonstrate applicable memory influencing the later investigation.
12. Repeat the request to verify idempotency.

Checklist:

- [ ] Episode persists.
- [ ] Finding persists with evidence references.
- [ ] Semantic memory persists.
- [ ] Procedure memory persists.
- [ ] Handoff persists.
- [ ] Receiving stage loads the handoff from storage.
- [ ] Handoff consumption is reflected in persisted state.
- [ ] New session retrieves authorized memory.
- [ ] Memory applicability is checked against current evidence.
- [ ] Memory influences the investigation only when applicable.
- [ ] Stale memory is marked/not treated as truth.
- [ ] Unauthorized cross-scope retrieval is denied.
- [ ] Repeated delivery does not duplicate handoffs or corrupt memory.

## Phase 8 — Operation reconnect and lifecycle rehearsal

1. Start a long-running operation and record its operation ID/current sequence.
2. Disconnect the browser/SSE subscriber.
3. Let the operation continue to the next meaningful stage.
4. Reconnect using the existing operation ID/last-known sequence.
5. Verify current state and events are recovered.
6. Deliver a duplicate/stale event in a test scenario.
7. Verify the UI does not rewind or restart.

- [ ] Disconnect does not mark the operation failed.
- [ ] Reconnect does not create a new operation.
- [ ] Current authoritative state is restored.
- [ ] Event sequence remains monotonic.
- [ ] Duplicates/out-of-order events do not regress status.
- [ ] Terminal outcome, completeness and freshness remain separate.
- [ ] Cancellation is represented only after confirmation.
- [ ] Safe error/limitation metadata survives the boundary.

## Phase 9 — Failure and recovery rehearsal

Execute the relevant `FAIL-*` scenarios from Section 17. At minimum, rehearse:

- offline/network failure,
- repository import/access error,
- LLM provider failure,
- Rust service failure,
- FalkorDB failure,
- partial indexing,
- unresolved relationship,
- unsupported syntax,
- forced traversal truncation,
- browser disconnect/reconnect,
- duplicate operation,
- stale memory,
- cross-scope denial.

After every destructive failure injection:

1. restore the known-good environment;
2. re-seed or verify the persistent demo graph as needed;
3. run a short known graph query;
4. confirm the system has returned to a demo-ready state.

## Phase 10 — Performance smoke test

Record actual measured results for:

- fixture indexing,
- one representative FalkorDB graph query,
- one complete investigation,
- one tool execution,
- progress propagation into the browser,
- memory retrieval,
- handoff persistence and subsequent retrieval.

Do not invent target values after seeing results. First confirm that:

- no call hangs indefinitely;
- traversal/result bounds hold on a larger controlled source input;
- the chat UI receives progress promptly enough to understand the stage;
- the final investigation finishes reliably enough for the planned demo;
- repeat runs do not degrade due to duplicate state.

If performance is not adequate, fix a measured bottleneck or cut an optional feature instead of introducing major new infrastructure on Day 4.

## Phase 11 — Demo and submission verification

- [ ] Reset to the known presentation state without deleting required source input.
- [ ] Confirm the local/offline demo path starts from a clean checkout.
- [ ] Rehearse the approximately three-minute Track 01 + Track 02 storyline.
- [ ] Confirm graph evidence and the query/path used in the demo are documented.
- [ ] Verify `README.md` setup/run instructions on a clean environment.
- [ ] Verify architecture, graph model, queries, testing and demo documentation agree.
- [ ] Add the required AI-coding-assistant disclosure.
- [ ] Confirm public source repository access.
- [ ] Record the final demo video.
- [ ] Verify video playback and that it shows the actual product.
- [ ] Confirm the demo's counts/findings are sourced from source-input/engine results.
- [ ] Freeze the submission candidate after the last successful run.

---

# 19. Day 4 Go / No-Go Gate

Do not declare CodeMind hackathon-ready until all protected outcomes pass.

## Track 01 must pass

- [ ] Rust end-to-end investigation.
- [ ] TypeScript/JavaScript end-to-end investigation.
- [ ] Python end-to-end investigation.
- [ ] All three use real adapters and the same canonical contract.
- [ ] FalkorDB determines connected evidence in a demonstrated multi-hop query.
- [ ] ChIE derives impact and risk from graph/evidence inputs.
- [ ] TIE returns explainable test candidates.
- [ ] Results include source/provenance and graph paths where available.
- [ ] Tool execution is validated, authorized and bounded.
- [ ] UI renders the structured result and honest limitations.
- [ ] Offline fixture demonstration works.

## Track 02 must pass

- [ ] Review episode and findings persist.
- [ ] Semantic memory and procedure persist.
- [ ] Persisted handoff is read and consumed by the receiving stage.
- [ ] Fresh session retrieves memory.
- [ ] Current evidence validates applicability/freshness.
- [ ] Memory informs a later investigation when applicable.
- [ ] Stale memory is not treated as canonical truth.
- [ ] Scope isolation is enforced and tested.
- [ ] Duplicate/concurrent writes do not silently corrupt state.

## Reliability must pass

- [ ] Fresh-start environment works.
- [ ] Protobuf/gRPC contract path works.
- [ ] FalkorDB data persists as intended.
- [ ] Operation progress/reconnect works.
- [ ] Partial/unsupported/unresolved/failure states remain honest.
- [ ] Key E2E flows are deterministic and repeatable.
- [ ] No blocking security/isolation failure remains.
- [ ] README/setup/demo/submission items are complete.

If a protected item fails, it is a **no-go for claiming the full MVP is complete**. Use the cut strategy from `implementation-plan.md` to reduce optional scope; do not replace failed behavior with fake output.

---

# 20. Defect Severity and Repair Policy

Use a simple triage policy so Day 4 does not get consumed by cosmetic work.

| Severity | Definition | Day 4 handling |
|---|---|---|
| **P0 — Demo/security blocker** | Data leakage, arbitrary graph/shell execution, corrupt canonical data, failed core Track 01/02 scenario, fake graph result | Stop demo work; fix or remove unsafe/unsupported feature |
| **P1 — Core behavior broken** | Rust/TS/Python path fails, memory/handoff is not persistent, gRPC contract loses important result semantics, reconnect creates duplicate work | Fix before final rehearsal |
| **P2 — Reliability/visible limitation** | Incorrect partial state, stale progress, bad recovery message, intermittent non-core error | Fix if it threatens the demo or user trust |
| **P3 — Cosmetic** | Spacing, icon, animation, small visual inconsistency without behavioral impact | Defer if any P0/P1/P2 remains |

After a fix:

1. add or repair a regression test;
2. run the narrow suite;
3. run impacted integration tests;
4. rerun the corresponding E2E;
5. rerun the final smoke test.

Do not merge a Day 4 fix without checking the affected dependency path.

---

# 21. Suggested Command Contract

The root workspace should provide stable, documented command wrappers. Their exact invocation should match the scripts actually implemented in `package.json`, `apps/web/package.json`, Cargo and the E2E runner. Do not document a command as passing until it has actually been run.

The minimum command categories are:

```text
Environment
  docker compose up -d
  scripts/setup.sh

Rust
  cargo fmt --all -- --check
  cargo test --workspace
  cargo clippy --workspace --all-targets

Web
  pnpm --filter web typecheck
  pnpm --filter web lint
  pnpm --filter web test
  pnpm --filter web build

Cross-system
  scripts/generate-proto.sh
  scripts/seed-demo.sh
  scripts/run-e2e.sh
  scripts/verify-clean-start.sh
```

If the chosen package scripts or tooling differ, update the actual wrapper/documentation to a single consistent command set. The key requirement is a reproducible command contract—not adherence to an arbitrary command name.

Network-dependent import checks should be separate from the mandatory offline graph/test/demo checks.

---

# 22. Final Test Record

Keep a concise execution record for the final candidate. It should contain:

```text
Candidate commit:
Date/time and timezone:
Environment:
Rust version:
Node/package manager version:
FalkorDB image/version:
Fixture revision/snapshot:
Protobuf generation status:

Rust unit tests:
Adapter conformance:
Canonical graph tests:
FalkorDB integration:
gRPC contracts:
Agent/tool guardrails:
Operation/progress tests:
Memory persistence:
Handoff lifecycle:
Scope/security:
UI/component tests:
Track 01 E2E:
Track 02 E2E:
Polyglot E2E:
Offline fallback:
Failure/recovery:
Performance smoke:

Known limitations:
Accepted non-blocking defects:
Demo video path/link:
Final go/no-go:
```

Record actual pass/fail outcomes. Do not state that a test passes merely because it appears in the plan.

---

# 23. Final Acceptance Contract

CodeMind is ready for submission only when all these statements are true:

1. **Rust works end to end.**
2. **TypeScript/JavaScript works end to end.**
3. **Python works end to end.**
4. The three adapters provide real source-derived canonical facts.
5. FalkorDB performs real connected-data reasoning.
6. ChIE produces evidence-backed impact/risk conclusions.
7. TIE provides test candidates with honest evidence semantics.
8. Tool calls are validated, authorized and bounded.
9. The UI presents structured results without inventing engine facts.
10. Partial, unresolved, unsupported, truncated and unavailable conditions are distinguished.
11. A persisted handoff is actually consumed by a receiving stage.
12. A fresh session retrieves persistent memory.
13. Current evidence validates that memory's applicability.
14. Applicable memory can inform a later investigation.
15. Scope isolation works.
16. Duplicate operations/updates do not silently corrupt state.
17. Progress and reconnect recovery work.
18. The clean-start Day 4 run is repeatable.
19. The demo works without requiring external network access.
20. Setup, graph model, useful queries, tests, demo and AI-assistance disclosure are ready.

---

## Final rule

**Day 1 proves the graph is real. Day 2 proves the agent reasons over it. Day 3 proves memory and handoffs persist across sessions. Day 4 proves that the complete product is repeatable, honest under failure and ready to submit.**

The testing plan is complete only when these are observed in the running system—not just described by the architecture or shown in a UI mockup.
