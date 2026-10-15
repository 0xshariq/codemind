# CodeMind Day 1 Implementation Plan

**Event:** Graph Hacks: Context for AI Agents, powered by FalkorDB  
**Implementation day:** Day 1, 15 October 2026 (IST)  
**Purpose:** Detailed execution plan for the Day 1 stages in `docs/implementation-plan.md`  
**Scope boundary:** This document expands Day 1 only. It does not replace or redefine `docs/architecture.md`.

> **Primary outcome:** Given a developer-supplied repository/source input in Rust, JavaScript/TypeScript, or Python, CodeMind creates canonical source-derived graph facts, persists them in real FalkorDB, and returns a real symbol/relationship query through Rust → gRPC/Protobuf → Next.js.

> **Day 1 status: PASS (15 October 2026).** The Compose/FalkorDB persistence proof, Rust graph integration test, source/CoIE/TIE tests, live gRPC query, Next.js API query, and relationship-removal causality proof have all passed. The `day-1` branch remains unpushed pending author review.
>
> Day 1 proves the graph is real. Day 2 adds change-intelligence reasoning and LLM tool orchestration. Day 3 proves memory and handoffs persist. Day 4 performs the complete integrated verification and submission gate.

---

## 1. Source-of-truth documents and execution rules

Use the uploaded project documents according to their defined responsibilities:

| Document | Authority for this plan |
|---|---|
| `AGENTS.md` | Operating discipline, branch protocol, security, how to handle ambiguity, and truthful test reporting |
| `docs/architecture.md` | Architecture, domain ownership, canonical graph semantics, language support, transport boundaries, and scope |
| `docs/implementation-plan.md` | Four-day order, Day 1 stages, primary file list, required tests, and exit gate |
| `docs/testing.md` | Required test semantics and final acceptance contracts; tests must be written alongside the behavior on Day 1 |
| `docs/demo.md` | Real-pipeline/offline-demo constraints and the rule that demo claims must follow actual behavior |
| `docs/design-system.md` | Shared visual language and accessibility rules if a minimal Day 1 UI is needed |

**Rules for using this document:**

1. Inspect the actual repository and current Git state before editing. Do not assume the local repository exactly matches the planned tree.
2. Implement the stages in order. A stage's gate must pass before dependent work begins.
3. Add tests alongside behavior, not at the end of the day.
4. Preserve the approved architecture. No Express service, separate Agent Runtime deployment, separate CoIE/TIE/ChIE services, direct Next.js-to-FalkorDB path, arbitrary Cypher tool, arbitrary repository-command execution, or product-level bundled fixture repository.
5. Never replace graph behavior with hardcoded results or a mock-only implementation.
6. Do not push `day-1` until every item in the full Day 1 exit gate passes. Local commits are allowed. Never discard or reset existing work without explicit approval.

---

## 2. Day 1 definition of done

Day 1 is complete only when all statements below have been demonstrated in the running system:

- [x] FalkorDB runs through Docker Compose and retains data in its named persistent volume after a restart.
- [x] The Rust `codemind-engine` service compiles and starts.
- [x] The Next.js `web` app starts.
- [x] Protobuf generation is repeatable, and both Rust and TypeScript compile against generated contracts.
- [x] A minimum typed request/response crosses the real gRPC boundary.
- [x] A developer-supplied source input can be resolved to a repository identity, an immutable source snapshot identity, and a source manifest.
- [x] Rust, JavaScript, TypeScript, and Python files are classified correctly.
- [x] The three language adapter families emit source-located canonical candidates for their supported subset.
- [x] Unsupported or unresolved syntax/relationships remain explicit diagnostics; they are not fabricated as confirmed facts.
- [x] CoIE-owned canonical code entities and relationships are persisted in FalkorDB.
- [x] TIE statically discovers supported test declarations and persists only relationships supported by evidence.
- [x] Re-ingestion is idempotent for the same repository/snapshot context.
- [x] Repository/workspace/snapshot boundaries prevent one context from reading another context's graph facts.
- [x] A bounded symbol/relationship query reads actual persisted relationships from FalkorDB through Rust.
- [x] The same query travels through gRPC and a Next.js server/API path, and the UI renders the actual structured result.
- [x] Removing or changing a relationship in an isolated test graph changes the query/path result as expected.
- [x] The mandatory Day 1 tests pass, and all executed commands and results are reported truthfully.

**Do not mark a checkbox based only on code being written.** Mark it after the corresponding command/test/manual check has actually passed.

---

## 3. Scope boundaries

### 3.1 Required on Day 1

- Runnable monorepo foundation: pnpm workspace, Next.js app, one Rust crate/service, and FalkorDB Compose service.
- Typed Protobuf contracts and code generation.
- Rust graph-storage port and FalkorDB adapter.
- Source provider, immutable snapshot context, file manifest, language classification, and safety/size limits.
- Minimum CoIE extraction for Rust, JavaScript, TypeScript, and Python.
- Code Graph persistence and bounded symbol/relationship queries.
- Basic TIE static discovery and supported test-to-code mappings.
- One real end-to-end vertical slice through FalkorDB, Rust, gRPC, Next.js, and a minimal UI.

### 3.2 Explicitly defer

- The full LLM agent loop and semantic tool registry.
- ChIE's complete change analysis, impact aggregation, risk assessment, and recommendation pipeline.
- Persistent review memory and handoff lifecycle implementation.
- A polished investigation workspace, elaborate graph visualization, dashboard metrics, animation, and cosmetic UI work.
- Runtime execution of repository tests or source code.
- Advanced Git history, full Git diff/PR analysis, incremental indexing optimizations, and sophisticated semantic resolution.
- A GitHub import flow if it competes with the local/offline graph path.

`flat` and `@toon-format/toon` belong in the web application's dependencies from the approved workspace stage, but Day 1 does not introduce a separate or premature LLM-result encoder. The canonical result remains structured data; the frozen `flat` → TOON path is used later by the JavaScript semantic-tool layer for successful Type B LLM-facing results.

---

## 4. Preflight: repository, branch, and specification checks

### 4.1 Inspect before editing

From the repository root:

```bash
git status --short --branch
git branch --show-current
```

Then inspect the actual files and scripts relevant to the first stage. Confirm whether the required paths already exist, and understand current contents before editing them. Preserve unrelated or uncommitted user changes.

### 4.2 Branch protocol

- Work on `day-1`.
- If the branch already exists, inspect it before switching.
- If a branch must be created, base it on the author-approved starting point and preserve existing work.
- Local commits may be made at useful checkpoints.
- Do not push the branch until the complete Day 1 exit gate passes.

### 4.3 Two source-document inconsistencies to keep visible

These are deliberately recorded instead of silently reconciled:

1. **TIE test-dataset relationship terminology:** `docs/architecture.md` uses `TestDataSet` and `USES_TEST_DATASET`; `docs/testing.md` section 7.2 still lists `USES_FIXTURE`. The Day 1 minimum static discovery path can proceed with test declarations, evidence-backed `TARGETS`, and the approved canonical code references. **Do not implement the disputed optional test-dataset relationship until the author resolves which vocabulary is intended.**
2. **Expected-graph test path:** `docs/testing.md` section 3.3 names `apps/web/src/tests/graph-conformance/`, while section 4.3 shows `tests/apps/web/src/tests/graph-conformance/expected/`. Inspect the actual repository and current test harness before creating expected-graph files. Do not create a second competing test tree merely to match the inconsistent path.

If either discrepancy turns out to affect a required Day 1 acceptance test, stop that specific change and ask the author for a decision. Do not edit a source-of-truth document simply to make the code compile.

---

## 5. Implementation sequence at a glance

```text
Preflight and branch check
        ↓
Stage 1.0: runnable workspace and persistent FalkorDB
        ↓
Stage 1.1: typed Protobuf contracts and generated bindings
        ↓
Stage 1.2: Rust graph-storage port + FalkorDB adapter
        ↓
Stage 1.3: source provider + immutable snapshot/manifest
        ↓
Stage 1.4: canonical CoIE extraction for the supported languages
        ↓
Stage 1.5: Code Graph persistence + basic TIE static discovery/queries
        ↓
Stage 1.6: real Rust → gRPC → Next.js query vertical slice
        ↓
Full Day 1 required tests and exit gate
```

**Dependency rule:** Do not build parser/engine logic on top of an unstable transport contract, do not build engine queries on top of direct adapter-specific Cypher, and do not declare a graph ready until the source snapshot and its canonical facts are tied to the same context.

---

# 6. Stage 1.0 — Establish the runnable workspace

**Goal:** Make the three primary runtime processes reproducibly start or compile before implementing graph functionality.

### Files in scope

- `AGENTS.md`
- `package.json`
- `pnpm-workspace.yaml`
- `apps/web/package.json`
- `pnpm-lock.yaml` (generated by pnpm; never hand-edit)
- `apps/web/next.config.ts`
- `apps/web/tsconfig.json`
- `engine/Cargo.toml`
- `engine/src/main.rs`
- `engine/src/lib.rs`
- `engine/src/config.rs`
- `docker-compose.yml`
- `.env.example`
- `.gitignore`
- `scripts/setup.sh`

### Implementation tasks

1. **Inspect the existing workspace.** Reuse existing files and working choices when they match the frozen architecture. Avoid dependency churn or creating empty placeholder directories in advance.
2. **Root JavaScript workspace.**
   - Configure the pnpm workspace to include the single `apps/web` application.
   - Make the root scripts/workspace configuration unambiguous.
   - Add `flat` and `@toon-format/toon` as web/server dependencies as required by the four-day plan.
   - Let pnpm generate and update the lockfile.
3. **Rust service foundation.**
   - Configure the single `codemind-engine` crate under `engine/`.
   - Keep reusable engine logic in the library and service startup in the binary.
   - Add minimum configuration loading and startup/error handling in `config.rs`, `lib.rs`, and `main.rs`.
   - Do not create separate crates/services for CoIE, TIE, or ChIE.
4. **FalkorDB Compose service.**
   - Add a minimal reproducible service definition.
   - Use a persistent named volume.
   - Keep connection information configurable rather than hardcoded in code.
   - Confirm the container/service health before later stages depend on it.
5. **Environment and secrets.**
   - Put variable names and safe local-development placeholders in `.env.example`.
   - Validate required configuration at startup and report missing/malformed settings clearly.
   - Never commit live credentials, tokens, or real secrets.
6. **Repository hygiene.**
   - Ignore appropriate build outputs, local environment files, and generated or local data that should not be committed.
   - Do not ignore source files or test evidence needed to reproduce the build.
7. **Setup script.**
   - `scripts/setup.sh` should perform only documented setup work that is actually supported by the repository.
   - Keep it repeatable; do not put hidden data deletion/reset behavior into an ordinary setup command.

### Stage 1.0 checks

- [x] Docker Compose configuration parses.
- [x] FalkorDB starts.
- [x] The Rust crate compiles.
- [x] `pnpm --filter web dev` starts the Next.js application.
- [x] A controlled test write survives a FalkorDB service/container restart when the persistent volume is retained.
- [x] No secrets or unrelated local files are staged.

### Gate

Proceed only when the application skeleton, Rust skeleton, and graph database are independently runnable. This stage does not pass merely because the files exist.

---

# 7. Stage 1.1 — Define and generate transport contracts

**Goal:** Agree on the typed boundary before engine behavior depends on it.

### Files in scope

- `proto/v1/common.proto`
- `proto/v1/repository.proto`
- `proto/v1/intelligence.proto`
- `proto/v1/memory.proto`
- `proto/v1/operation.proto`
- `engine/build.rs`
- `engine/src/contract/mod.rs`
- `engine/src/contract/context.rs`
- `engine/src/contract/repository.rs`
- `engine/src/contract/intelligence.rs`
- `engine/src/contract/result.rs`
- `engine/src/contract/error.rs`
- `scripts/generate-proto.sh`
- `apps/web/src/lib/server/rust-client/generated/` (generated output)

### Contract design tasks

1. **Shared context first.** Define the typed repository/snapshot and operation context needed to bind requests and results to one trusted work context.
2. **Canonical result envelope.** Preserve, as applicable:
   - outcome;
   - completeness;
   - freshness;
   - repository and source-snapshot context;
   - graph version/provenance;
   - typed result data;
   - evidence/source references;
   - diagnostics and limitations;
   - safe error information.
3. **Use JSON only for canonical data inside the Protobuf response.**
   - Rust serializes the canonical structured data as JSON text in the agreed typed response field.
   - Protobuf remains the typed transport envelope.
   - Do not serialize TOON from Rust or use TOON as the transport/canonical format.
   - JavaScript parses and validates the returned JSON at the server boundary.
4. **Repository/query contracts.** Add the minimum typed requests and responses needed to resolve/describe a source snapshot and query persisted symbols/relationships. Let the existing architecture define the semantics; do not invent a large RPC surface just to anticipate future work.
5. **Correlation and event metadata.** The operation/progress contract needs the required identity and correlation fields, including `operation_id`, `request_id`, `trace_id`, `repository_id`, engine/stage information, stable event identity or sequence, and timestamp where applicable.
6. **Memory/handoff definitions.** Include their agreed contract messages now so code generation can be validated early. Do not implement the Day 3 memory/handoff behavior in this stage.
7. **Generated artifacts.** Generate Rust and TypeScript bindings from `proto/v1/`. Never hand-edit generated files.
8. **Typed error behavior.** Make invalid context, invalid inputs, unsupported cases, timeouts, and service unavailability distinguishable from a successful empty result.

### Stage 1.1 tests

- [x] `scripts/generate-proto.sh` can be run repeatedly with consistent output.
- [x] Generated Rust types compile.
- [x] Generated TypeScript types compile.
- [x] A basic typed request/response round-trip test passes.
- [x] Repository/snapshot context, correlation IDs, result status, evidence references, completeness, freshness, and diagnostics survive serialization.
- [x] A missing or invalid required context becomes an explicit typed failure, not a fake empty result.
- [x] Generated files are not hand-patched to hide a schema/code mismatch.

### Gate

No graph-engine code should depend on an unverified or manually duplicated transport contract.

---

# 8. Stage 1.2 — Implement the graph-storage boundary and FalkorDB adapter

**Goal:** Give Rust engines a small domain-independent storage boundary and prove real FalkorDB reads/writes through it.

### Files in scope

- `engine/src/graph_storage/mod.rs`
- `engine/src/graph_storage/port.rs`
- `engine/src/graph_storage/falkordb/mod.rs`
- `engine/src/graph_storage/falkordb/client.rs`
- `engine/src/graph_storage/falkordb/schema.rs`
- `engine/src/graph_storage/falkordb/read.rs`
- `engine/src/graph_storage/falkordb/write.rs`
- `engine/src/graph_storage/falkordb/mapping.rs`
- `engine/src/contract/error.rs`

### Implementation tasks

1. **Design the smallest storage port needed for Day 1.** It must support the approved engine operations without exposing FalkorDB-specific types or Cypher to CoIE/TIE callers.
2. **Keep the database adapter as the only Cypher boundary.**
   - Cypher and FalkorDB-specific API knowledge stay inside `graph_storage/falkordb/`.
   - Do not put database queries in Next.js, route handlers, React components, or LLM-facing code.
3. **Configuration and connections.**
   - Read connection settings from validated configuration.
   - Map connection and query failures to safe typed errors.
   - Avoid logging credentials or raw secrets.
4. **Schema/bootstrap.** Implement the minimum graph bootstrap/schema behavior that the Day 1 canonical model requires. Avoid speculative indexes or large future-domain schemas without an immediate requirement.
5. **Canonical mapping.** Map canonical entity and relationship values to/from FalkorDB without losing kind, direction, properties, source/snapshot provenance, or workspace context.
6. **Parameterized operations.** Use safe parameterized query patterns wherever supported. Do not concatenate untrusted repository or symbol content into Cypher.
7. **Idempotent writes.** Use stable canonical identity and merge/upsert behavior so re-ingesting the same context does not duplicate nodes or edges.
8. **Workspace isolation.** Preserve the one-physical-graph-per-workspace architecture and require trusted workspace/repository context on operations. Do not trust LLM or browser-supplied identity fields as authorization.
9. **Source storage boundary.** Keep source snapshots, source contents, and manifests outside FalkorDB. Store graph facts plus provenance/source references in FalkorDB.
10. **Integration-test isolation.** Use a disposable graph or explicit isolated test workspace/context. Test cleanup must not erase the demo graph implicitly.

### Stage 1.2 tests

- [x] A Rust integration test writes a small known graph through the storage port.
- [x] A separate Rust read queries that graph back through the same port.
- [x] Entity kind, relationship type/direction, and key provenance fields match the expected values.
- [x] Repeating the same write does not increase canonical entity/relationship counts.
- [x] Workspace A cannot read Workspace B's test data.
- [x] Retained graph data survives the intended restart test.
- [x] A successful empty query is distinguishable from an unavailable database or a failed query.
- [x] A failure in the FalkorDB adapter is surfaced as an explicit error; it does not return invented graph facts.

### Gate

Do not begin CoIE persistence until the real adapter can write/read an isolated graph, preserve scope, and demonstrate repeat-write idempotency.

---

# 9. Stage 1.3 — Source provider, immutable snapshots, and file inventory

**Goal:** Resolve developer-supplied source into a stable, traceable input for every engine.

### Files in scope

- `engine/src/source/mod.rs`
- `engine/src/source/provider.rs`
- `engine/src/source/snapshot.rs`
- `engine/src/source/manifest.rs`
- `engine/src/source/language.rs`
- `engine/src/source/limits.rs`
- `engine/src/source/tree_sitter.rs`

### Implementation tasks

1. **Repository provider.**
   - Accept a developer-supplied repository/source input through the approved provider path.
   - Make local source input the dependable Day 1 path.
   - Do not make ingestion depend on public GitHub availability or a predefined CodeMind demo repository.
   - If repository acquisition/input semantics are not present in the actual repository, inspect existing design and ask before introducing a new public arbitrary-host-path API.
2. **Repository identity and snapshot identity.**
   - Resolve a stable repository identity.
   - Resolve an immutable `sourceSnapshotId` representing the analyzed source state.
   - Keep repository identity, source snapshot identity, graph version, and entity identity distinct.
   - Do not hardcode a particular identity formula beyond the requirements for determinism, snapshot awareness, and collision avoidance.
3. **Snapshot manifest.** Record enough information to associate every indexed file and later query with the correct repository and snapshot. Include repository-relative path, detected language, provenance, and supported file metadata.
4. **Source storage.** Keep source bytes, source snapshots, and manifests outside FalkorDB; the graph will store structural facts and references to source evidence.
5. **File inventory.**
   - Walk the supplied repository recursively.
   - Normalize paths to repository-relative paths.
   - Detect `.rs`, `.js`, `.jsx`, `.ts`, `.tsx`, and `.py`.
   - Apply repository ignore rules and agreed source-size/processing limits.
   - Do not let a path escape the supplied repository root.
6. **Tree-sitter foundation.** Prepare the Tree-sitter parsing layer and language detection needed by the CoIE adapters.
7. **Partial inputs.** Distinguish unsupported language/syntax, parse failures, and successfully inspected files. Do not mark all graph preparation successful when required processing failed.
8. **Untrusted source discipline.** Treat source, comments, READMEs, test names, and configuration content as untrusted data. Do not execute source code or commands from the supplied repository.

### Stage 1.3 tests

- [x] Resolving the same unchanged source input yields stable repository/snapshot context.
- [x] Changing the source input yields a distinguishable snapshot.
- [x] Nested directories and repository-relative paths are handled correctly.
- [x] Rust, JavaScript, TypeScript, and Python extensions are identified correctly.
- [x] Ignore rules and source-size limits are applied.
- [x] Unsupported files or parse failures produce diagnostics rather than fabricated entities.
- [x] Two repository/workspace contexts do not mix snapshot/source records.
- [x] The test harness can construct temporary source trees; no product-level `fixtures/repositories/` directory is added.

### Gate

A developer-supplied source input must be resolvable and snapshotted without needing a bundled product repository or external GitHub connectivity.

---

# 10. Stage 1.4 — Minimum CoIE extraction for Rust, JavaScript, TypeScript, and Python

**Goal:** Convert supported syntax into source-located canonical candidates using the same language-agnostic graph model.

### Files in scope

- `engine/src/engines/mod.rs`
- `engine/src/engines/coie/mod.rs`
- `engine/src/engines/coie/engine.rs`
- `engine/src/engines/coie/model.rs`
- `engine/src/engines/coie/graph_builder.rs`
- `engine/src/engines/coie/extraction/mod.rs`
- `engine/src/engines/coie/extraction/normalize.rs`
- `engine/src/engines/coie/extraction/validate.rs`
- `engine/src/engines/coie/extraction/resolve.rs`
- `engine/src/engines/coie/extraction/evidence.rs`
- `engine/src/engines/coie/languages/mod.rs`
- `engine/src/engines/coie/languages/rust.rs`
- `engine/src/engines/coie/languages/javascript.rs`
- `engine/src/engines/coie/languages/typescript.rs`
- `engine/src/engines/coie/languages/python.rs`

### Extraction pipeline

```text
Immutable source snapshot
        ↓
File inventory + language detection
        ↓
Language-specific Tree-sitter adapter
        ↓
Entity/relationship candidates + source ranges
        ↓
Canonical normalization and validation
        ↓
Identity and reference resolution
        ↓
Relationship validation + evidence/diagnostics
        ↓
CoIE graph builder
```

### Minimum Day 1 extraction subset

Support the approved initial subset only:

- files and directories;
- modules/packages where supportable;
- declaration entities such as functions, methods, classes, structs, traits, interfaces, enums, constants, macros, and supported types;
- source ranges and source evidence;
- imports/exports where supported;
- direct calls only when the target is actually resolvable;
- diagnostics for syntax errors, unresolved relationships, and unsupported constructs.

This is a minimum supported subset, not a promise to fully semantically resolve each language.

### Implementation tasks

1. **Canonical candidate contract.** All language adapters must emit the same candidate shape. Preserve native language/kind information while normalizing entities into the canonical `Repository`, `RepositorySnapshot`, `Directory`, `File`, `Module`, `Package`, and `Symbol` model where applicable.
2. **Identity and snapshot association.** Use deterministic identities appropriate to the canonical model. Ensure symbols in different files/scopes or overloaded declarations do not collide, and facts from distinct source snapshots are not mixed.
3. **Source provenance.** Preserve repository-relative path, source range when available, language, parser/adapter provenance, and resolution status.
4. **Structural relationships.** Build supported `CONTAINS`, `DEFINES`, `IMPORTS`, and `EXPORTS` semantics where justified by the source.
5. **Behavioral/dependency relationships.** Emit `CALLS`, `REFERENCES`, `USES_TYPE`, `IMPLEMENTS`, `INHERITS_FROM`, or `OVERRIDES` only when the chosen parser/resolution subset can support that relationship. Do not add an edge because two names are similar.
6. **Shared validation.** Run shared normalization, validation, identity assignment, evidence attachment, and relationship validation after language-specific extraction.
7. **Keep adapters storage-agnostic.** Language adapters produce candidates; they do not write to FalkorDB directly. The graph builder and storage port own publication.
8. **Honest partial results.** When parsing one supported file fails, retain only internally consistent facts and report the partial/degraded status. Do not knowingly publish stale edges as current facts.

### Per-language minimum verification

| Adapter | Verify |
|---|---|
| Rust | `.rs` detection; supported declarations; supported imports/resolvable calls; source ranges; static `#[test]` declaration can be found by TIE; unresolved macro/dynamic cases remain diagnostic |
| JavaScript | `.js` / `.jsx` detection; supported declarations/imports/exports/calls; supported Jest/Vitest test syntax can be found by TIE; unresolved dynamic imports/calls are not invented |
| TypeScript | `.ts` / `.tsx` detection; functions/classes/interfaces/types/modules in the canonical schema; type-only relationships are not falsely represented as runtime calls; unsupported advanced resolution is diagnostic |
| Python | `.py` detection; supported modules/classes/functions/imports/resolvable calls; supported pytest/unittest declarations can be found by TIE; unresolved dynamic dispatch/import remains unresolved |

### Stage 1.4 tests

- [x] Each adapter has controlled test inputs with source-located canonical candidates.
- [x] Equivalent supported constructs yield compatible canonical semantics across languages.
- [x] Native kind and normalized kind are preserved where applicable.
- [x] Source ranges point to the correct file and source snapshot.
- [x] Imports/aliases are not misrepresented as definitions.
- [x] Calls are linked only when resolution supports the target.
- [x] Same-named symbols in different files/scopes remain distinct.
- [x] Unsupported or ambiguous syntax is explicitly reported.
- [x] Static test declarations are not confused with execution or coverage evidence.

### Gate

All three language adapter families must produce real source-derived canonical files/symbols and supported relationships from their own language test inputs. No fabricated cross-language dependency chains.

---

# 11. Stage 1.5 — Persist the Code Graph and build basic TIE static discovery

**Goal:** Prove that code relationships and test candidates can be retrieved from real FalkorDB through bounded semantic query functions.

### Files in scope

- `engine/src/engines/coie/queries/mod.rs`
- `engine/src/engines/coie/queries/symbols.rs`
- `engine/src/engines/coie/queries/search.rs`
- `engine/src/engines/coie/queries/relationships.rs`
- `engine/src/engines/coie/queries/paths.rs`
- `engine/src/engines/tie/mod.rs`
- `engine/src/engines/tie/engine.rs`
- `engine/src/engines/tie/model.rs`
- `engine/src/engines/tie/graph_builder.rs`
- `engine/src/engines/tie/discovery/mod.rs`
- `engine/src/engines/tie/discovery/rust_tests.rs`
- `engine/src/engines/tie/discovery/javascript_tests.rs`
- `engine/src/engines/tie/discovery/typescript_tests.rs`
- `engine/src/engines/tie/discovery/python_tests.rs`
- `engine/src/engines/tie/mapping/mod.rs`
- `engine/src/engines/tie/mapping/target_mapping.rs`
- `engine/src/engines/tie/mapping/evidence.rs`
- `engine/src/engines/tie/queries/mod.rs`
- `engine/src/engines/tie/queries/tests.rs`

### CoIE graph publication and query tasks

1. Publish source-derived canonical entities and relationships through the Rust Graph Storage Port.
2. Associate published facts with repository, source snapshot, and workspace context.
3. Add bounded queries for:
   - symbol lookup;
   - code search;
   - direct relationship retrieval;
   - supported bounded paths between symbols/entities.
4. Apply result limits and context/snapshot checks at the semantic query boundary.
5. Preserve relationship type and direction, path order, source evidence, and unresolved/partial/truncated status.
6. Do not introduce a primitive `DEPENDS_ON` edge merely to simplify a query. Use the existing canonical relationship semantics and derive query views where supported.
7. Verify a read/query operation does not mutate canonical graph state as a side effect.

### TIE static discovery and mapping tasks

1. Discover only the approved static test subsets:
   - Rust native test harness, including supported `#[test]` patterns;
   - a useful supported subset of Jest/Vitest for JavaScript and TypeScript;
   - a supported subset of pytest/unittest for Python.
2. Persist TIE-owned test entities while referencing the existing CoIE entities; do not create duplicate code identities.
3. Record the declaring source and repository/snapshot context.
4. Create `DECLARED_BY` for supported declaration ownership and `TARGETS` only when the selected static mapping/resolution approach supports the association with evidence.
5. Deduplicate test records and mappings by canonical identity.
6. Require the TIE mapping and referenced CoIE target to belong to a compatible snapshot context.
7. Do not execute test files, arbitrary shell commands, or test runners as part of discovery.
8. Do not emit `EXERCISES` or `COVERS` without actual attributable runtime/coverage evidence.
9. Preserve different statuses distinctly, including `supported_no_results`, `partial`, `unsupported`, and `failed`.

### Stage 1.5 tests

- [x] CoIE facts are persisted and read back through the storage port.
- [x] Symbol search and lookup return actual graph entities, not a prewritten list.
- [x] Relationship queries preserve edge type, direction, endpoints, path, source evidence, and snapshot context.
- [x] TIE discovers supported static tests for each supported language/framework subset.
- [x] `DECLARED_BY` and supported `TARGETS` semantics are correct.
- [x] No runtime execution or coverage claim is inferred from static discovery alone.
- [x] Unknown, unresolved, or ambiguous test mappings remain qualified.
- [x] Repeated ingestion does not duplicate test nodes or mappings.
- [x] Query results respect repository/workspace/snapshot context and configured bounds.
- [x] Removing a necessary relationship in the isolated graph changes the relevant query/path result.

### Gate

FalkorDB contains real source-derived code and test facts for the supported inputs. A bounded query beginning from a selected symbol can retrieve its connected symbols and supported relevant test candidates.

---

# 12. Stage 1.6 — Complete a real Rust → gRPC → Next.js vertical slice

**Goal:** Connect the previously tested stages into the smallest working application flow without an LLM, fake result, or polished investigation UI being required.

### Files in scope

- `engine/src/transport/mod.rs`
- `engine/src/transport/server.rs`
- `engine/src/transport/conversions.rs`
- `engine/src/main.rs`
- `apps/web/src/lib/server/rust-client/client.ts`
- `apps/web/src/lib/server/results/canonical-json.ts`
- `apps/web/src/lib/server/results/validate.ts`
- `apps/web/src/lib/server/config/env.ts`
- `apps/web/src/app/api/repositories/route.ts`
- `apps/web/src/app/api/repositories/[repositoryId]/route.ts`
- `apps/web/src/app/repositories/page.tsx`
- `apps/web/src/app/repositories/[repositoryId]/page.tsx`
- `apps/web/src/lib/shared/types.ts`
- `engine/tests/grpc_contract.rs`
- `engine/tests/falkordb_integration.rs`

If a small repository-preparation workflow function is necessary to keep Route Handlers thin, place it under the already-approved `apps/web/src/lib/server/workflows/` module. Do not create another backend/service, and do not move graph traversal or business logic into Route Handlers.

### Vertical flow

```text
Developer-supplied local repository/source input
        ↓
Resolve repository + immutable snapshot
        ↓
CoIE extraction and canonicalization
        ↓
TIE static test discovery/mapping
        ↓
Publish canonical Code/Test Graph to FalkorDB
        ↓
Select or resolve a canonical symbol
        ↓
Run a bounded relationship query in Rust
        ↓
Rust typed response over gRPC/Protobuf
        ↓
Next.js server-side Rust client
        ↓
Parse and validate canonical JSON result
        ↓
Thin Route Handler/application workflow
        ↓
UI renders actual structured entities and relationships
```

### Implementation tasks

1. **Rust service entry point.** Start the gRPC service with validated configuration and explicit startup errors. Keep CoIE/TIE/ChIE modules inside the single service.
2. **Transport-to-domain conversion.** Convert typed transport requests into engine inputs and engine results into typed response envelopes without losing repository/snapshot context, outcome, completeness, freshness, evidence, diagnostics, or limitations.
3. **Server-side client.** Implement the Next.js Rust client as the only application-side communication boundary to Rust. Apply safe timeout/error mapping and validate response context.
4. **Canonical JSON validation.**
   - Parse the JSON text field from the Protobuf response.
   - Validate required canonical fields and context before using the result.
   - Preserve JSON as the structured canonical result.
   - Do not flatten or TOON-encode data for the UI.
   - Do not hide malformed data behind a default successful empty response.
5. **Thin API handlers.** Handle HTTP parsing, request validation, authorization/context binding, and response construction. They must call application/server code and must not contain Cypher, traversal, or engine algorithms.
6. **Minimal repository UI.**
   - Show actual repository/snapshot state.
   - Allow a user to open/select the input and a real returned symbol.
   - Show actual query output from the service.
   - Keep the page functional and readable; defer elaborate chat layouts, dashboards, graph controls, and cosmetic polish.
7. **UI contract.** `apps/web/src/lib/shared/types.ts` should define typed UI-safe representations without importing database clients or relying on LLM prose. UI facts must originate in validated structured results.
8. **Actual status semantics.** Empty successful results, unsupported input, partial extraction, unresolved relationships, graph unavailability, timeout, and failure must not be conflated.
9. **Restart persistence.** Restart the app/service as part of the check and verify that a new query can read the persisted graph. Do not re-seed a fake result into frontend state.

### Stage 1.6 tests

- [x] A basic gRPC request/response works between the actual Next.js server client and Rust service.
- [x] Round-trip preserves repository/snapshot context, correlation metadata, structured result, evidence references, completeness/freshness, and diagnostics.
- [x] Invalid request context is rejected explicitly.
- [x] Rust service unavailable/timeout is distinguishable from successful empty output.
- [x] Route Handler uses the application/server boundary and has no FalkorDB/Cypher dependency.
- [x] The browser does not connect directly to Rust or FalkorDB.
- [x] UI renders the real structured query result.
- [x] A restart does not require hardcoded frontend results or a fresh fake graph.
- [x] The same bounded query is repeatable against the same graph version and context.

### Gate

The same developer-supplied input can be indexed, persisted in real FalkorDB, queried in Rust, returned over typed gRPC, validated by Next.js, and rendered in the UI. This is the Day 1 vertical slice.

---

# 13. Day 1 required test matrix

Run narrow tests as soon as their implementation stage exists; do not wait until the end.

| Area | Required assertion | Evidence of completion |
|---|---|---|
| Workspace/startup | FalkorDB, Rust, and Next.js start/compile | Actual command output |
| Protobuf | Regeneration is repeatable; Rust and TypeScript compile; round-trip preserves fields | Generation run + contract-test result |
| Canonical identity | Stable identities; overload/scope distinctions; source ranges and snapshot context preserved | Unit/conformance tests |
| Language adapters | Rust, JS/TS, Python produce supported source-located entities and relationships | Adapter conformance results |
| Source snapshots | Stable unchanged snapshot identity; changed input is distinguishable; paths remain within root | Source tests |
| Canonical graph | Correct node kind and relationship type/direction | Graph conformance assertions |
| FalkorDB | Real write/read through Rust; restart persistence; workspace isolation | `engine/tests/falkordb_integration.rs` or equivalent |
| Idempotency | Repeat ingestion without duplicate canonical entities/relationships/tests | Integration test |
| TIE | Static tests discovered; supported `TARGETS` evidence retained; no false execution/coverage claims | TIE unit/integration tests |
| Bounded query | Symbol/relationship/path queries use actual database relationships and respect limits | Query integration test |
| Graph causality | Altering a required edge changes the canonical path/result | Isolated real-FalkorDB test |
| gRPC | Typed request/response and safe errors work | `engine/tests/grpc_contract.rs` or equivalent |
| Next.js boundary | Server client validates canonical JSON; route is thin; no browser DB access | Route/client test |
| Minimal UI | Real response rendered, honest empty/partial/error state | App test or manual verified workflow |
| Security | No source execution, arbitrary Cypher, leaked credentials, path traversal, or cross-workspace reads | Focused negative tests/review |

The exact filenames may be organized into an adjacent test module if that is already the repository convention. Do not duplicate tests just to match a filename. Required coverage is the acceptance contract.

---

# 14. Suggested verification commands

Use only commands/scripts that actually exist in the current repository. Where this plan asks for a script to be created, verify it works before relying on it.

### Environment and Protobuf

```bash
docker compose up -d
scripts/generate-proto.sh
```

Check the repository before assuming these commands are already implemented. Verify FalkorDB health and generated output after running them.

### Rust

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

Run narrow tests first where useful, then the broader workspace suite. If the Cargo manifest/workspace uses a different valid command contract, follow the actual manifest and keep the documented command consistent.

### Next.js / TypeScript

At minimum, verify:

```bash
pnpm --filter web dev
```

Run the web typecheck, lint, unit tests, and build commands only if their scripts are actually defined. Add those scripts when the approved workspace stage requires them; do not report a nonexistent script as a passed check.

### Integration and vertical slice

- Run the FalkorDB integration tests against the real service, not a mock.
- Run the Protobuf/gRPC contract tests.
- Exercise the actual Next.js server client → Rust service → FalkorDB query path.
- Repeat ingestion, query the same graph, and verify idempotency.
- Perform the isolated graph-edge mutation proof.
- Restart the relevant process/service and query the persisted graph again.

---

# 15. Failure handling and stop conditions

Stop the dependent stage, isolate the cause, and fix or ask for a decision when any of these occurs:

- Protobuf bindings drift from the `.proto` source or cannot be regenerated.
- The storage port leaks FalkorDB-specific Cypher/types into domain engines or Next.js.
- A graph write/read test only works against a mock.
- Re-ingestion duplicates entities/edges or serves facts from the wrong snapshot.
- An adapter creates a confirmed relationship from name similarity rather than supported resolution.
- Static test discovery runs arbitrary repository code or implies that a test executed.
- A query returns the same hardcoded path after the persisted relationship required for that path is removed.
- A cross-workspace query returns another workspace's graph facts.
- A timeout, unavailable service, parse failure, unsupported construct, and successful empty query are collapsed into one success state.
- The Next.js route accesses FalkorDB directly or contains graph traversal/engine logic.
- The UI renders invented counts, symbols, tests, or graph paths.
- A required source-document conflict changes behavior or a canonical contract.

Do not fix a failing test by deleting/weaken­ing the assertion. Use lower-level tests to localize a failure before repeatedly running the full stack.

---

# 16. If Day 1 is falling behind

Keep the required proof intact. Cut in this order:

1. Bounded public GitHub import.
2. Optional parsing depth beyond the minimum subset.
3. Nonessential UI polish and secondary display elements.
4. Nonessential helper abstractions that are not required by the frozen contracts.

Do **not** cut:

- real FalkorDB persistence and traversal;
- the Rust storage boundary;
- the typed Rust ↔ gRPC/Protobuf ↔ Next.js path;
- minimum supported-language extraction for Rust, JavaScript/TypeScript, and Python;
- the basic TIE static discovery path and its honest evidence semantics;
- snapshot/workspace isolation;
- idempotency;
- the relationship-dependent graph query proof;
- required tests or the Day 1 gate.

If the protected subset cannot be completed, report the exact failure and ask for an author decision. Do not claim Day 1 passed or push the branch knowingly failing the gate.

---

# 17. Day 1 final exit gate

Complete this checklist from the actual working tree and environment:

## Processes and workspace

- [x] `day-1` branch verified and unrelated changes preserved.
- [x] FalkorDB starts.
- [x] FalkorDB data survives the intended restart with its persistent volume.
- [x] Rust service compiles and starts.
- [x] Next.js starts.
- [x] Environment examples are present and no secrets are committed.

## Contracts and storage

- [x] Protobuf generation is repeatable.
- [x] Rust and TypeScript compile against the generated contracts.
- [x] Basic typed gRPC request/response test passes.
- [x] FalkorDB adapter writes and reads a known graph through the storage port.
- [x] Workspace scoping and repeat-write idempotency tests pass.

## Source, CoIE, and TIE

- [x] Source snapshot/provenance is recorded.
- [x] Rust adapter works.
- [x] JavaScript adapter works.
- [x] TypeScript adapter works.
- [x] Python adapter works.
- [x] Canonical Code Graph is persisted.
- [x] Basic Test Graph and supported static test mappings are persisted.
- [x] Unsupported/unresolved cases remain explicit.
- [x] No tests or source commands are executed by static discovery.
- [x] Snapshot/workspace isolation is tested.

## Actual query path

- [x] Bounded symbol/relationship query returns real FalkorDB facts.
- [x] Query passes through Rust and gRPC/Protobuf.
- [x] Next.js validates canonical JSON and receives the real result.
- [x] The UI renders actual structured entities/relationships.
- [x] A relevant persisted relationship change changes the isolated test query/path result.
- [x] Repeated ingestion is safe.
- [x] A service restart does not require hardcoded data to recover the query result.

## Tests and reporting

- [x] All Day 1 unit, conformance, contract, and integration tests pass.
- [x] Commands run and outcomes are recorded accurately.
- [x] Known limitations and unresolved specification decisions are listed.
- [x] Working-tree diff is reviewed for unrelated changes and secrets.
- [x] No branch push occurs before every required gate above passes.

**Day 1 exit decision:** `PASS` only when every required item is verified; otherwise `NOT PASS` with the failed checks and next action explicitly listed.

---

## 18. Required end-of-day report

Report the following to the author:

1. **Implemented:** the actual source-to-graph-to-query behavior now available.
2. **Files changed:** grouped by workspace/contracts/storage/source/CoIE/TIE/transport/web/tests.
3. **Checks run:** exact commands and observed pass/fail outcomes.
4. **Gate:** whether each Stage 1.0–1.6 gate and the full Day 1 exit gate passed.
5. **Evidence:** which real FalkorDB integration test and graph-edge dependency test prove the graph is doing actual work.
6. **Limitations:** unsupported syntax, unresolved relationships, and any cut optional scope.
7. **Specification issues:** author decisions still needed, including the two inconsistencies recorded in Section 4.3.
8. **Git:** current branch, working-tree status, commit information if committed, and whether the branch remains unpushed pending the full gate.

Do not report success merely because an implementation file exists or the UI appears correct.

---

## Final rule

**Day 1 must end with a real, persistent, source-derived graph and a query that crosses the real Rust → gRPC/Protobuf → Next.js boundary. No fake graph, invented relationships, direct database shortcut, or skipped gate can substitute for that proof.**
