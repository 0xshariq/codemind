# CodeMind Implementation Plan

**Scope:** Graph Hacks: Context for AI Agents, powered by FalkorDB  
**Implementation window:** 15–18 October 2026 (IST)  
**Delivery model:** Three days of implementation followed by one feature-frozen verification and submission day  
**Status:** Frozen four-day execution plan reconciled with the finalized monorepo structure, repository-agnostic language support, three-level engine testing, JavaScript JSON-contract testing, and Day 4 manual/TestSprite verification

**Hackathon timing constraint:** The plan may be prepared before the event, but implementation of the actual hackathon submission starts only within the official 15–18 October window. Do not pre-build submission functionality before the event opens.

> This plan is intentionally stricter than a general roadmap. It fixes the physical project structure first, assigns responsibilities to the important files, and then orders implementation by dependency. Follow the sequence instead of jumping to whichever screen or feature looks easiest.

---

## How to use this plan

1. **Treat Part I as the canonical file layout.** Do not create a second API service, a separate Agent Runtime package, or one Rust service per engine.
2. **Treat Part II as the required implementation order.** A stage should pass its exit gate before dependent stages begin.
3. **Create files when their stage begins.** The tree is finalized, but do not create dozens of empty placeholder files on Day 1.
4. **Test alongside each implementation step.** Day 4 is for integrated verification and repair, not for writing the entire test suite.
The repository-analysis workflow must operate on the developer-supplied supported repository; test-only source trees are isolated test inputs and are not product fixtures.
6. **`architecture.md` owns the why and the boundaries.** This document owns the build order, file responsibilities, dependencies, daily gates and cut strategy. `testing.md` will own the detailed testing matrix; `demo.md` will own the final rehearsed script; `AGENTS.md` will guide coding-agent behavior.

The names below are the planned source files, not code that should be generated in advance. Generated Protobuf bindings, `Cargo.lock`, `pnpm-lock.yaml`, generated test graph output and compiled artifacts are created by the relevant build or test stages.

---

# Part I — Freeze the Project Structure

## 1. Structural decisions

The following boundaries are fixed for the hackathon:

- One monorepo.
- `apps/web` is the Next.js application, including Route Handlers and the server-side Agent Runtime.
- No Express backend.
- No separately deployed `agent-runtime` package or second Agent Runtime service.
- One Rust service in `engine`.
- CoIE, TIE and ChIE are modules inside that Rust service, not separate services or crates.
- Protobuf source files live at the repository root and are the single source of truth.
- Generated Rust/TypeScript bindings are build artifacts.
- Rust Type A/Type B results cross gRPC in the approved canonical JSON string payload inside the typed Protobuf response.
- The server-side JavaScript tool-result pipeline uses `flat` followed by `@toon-format/toon` for LLM-facing Type B results; this is mandatory, not optional.
- FalkorDB is accessed only through the Rust Graph Storage Port and its adapter.
- No distributed worker, Redis queue, Kafka, RabbitMQ, Temporal or Kubernetes in the MVP.
- One persistent FalkorDB volume supports repository graph, memory, handoffs and recoverable operation information.
- The same app pipeline must serve the real demo and ordinary investigations. No fake demo-only response pipeline.

### Canonical Next.js path rule

Earlier sketches used both `apps/web/app` and `apps/web/src/lib`. To eliminate ambiguity, this plan standardizes on the Next.js `src/` layout:

- App Router pages and Route Handlers: `apps/web/src/app/`
- UI components: `apps/web/src/components/`
- server-only application modules: `apps/web/src/lib/server/`
- shared UI-safe types: `apps/web/src/lib/shared/`

**Do not create a second `apps/web/app/` or `apps/web/components/` tree.**

## 1.1 Single AGENTS.md decision

CodeMind uses one root `AGENTS.md`. We do not create separate `AGENTS.md` files inside `apps/web/` or `engine/`.

The root rules apply to the entire monorepo and prevent contradictory local instructions between the Agent Runtime and Rust engine. Directory ownership tables in this plan and `docs/architecture.md` provide the implementation-specific guidance.

## 2. Canonical monorepo tree

```text
codemind/
├── AGENTS.md
├── README.md
├── package.json
├── pnpm-workspace.yaml
├── playwright.config.ts
├── .env.example
├── .gitignore
├── docker-compose.yml
│
├── apps/
│   └── web/
│       ├── package.json
│       ├── next.config.ts
│       ├── tsconfig.json
│       ├── vitest.config.ts
│       └── src/
│           ├── app/
│           │   ├── layout.tsx
│           │   ├── page.tsx
│           │   ├── globals.css
│           │   ├── repositories/
│           │   │   ├── page.tsx
│           │   │   ├── new/
│           │   │   │   └── page.tsx
│           │   │   └── [repositoryId]/
│           │   │       └── page.tsx
│           │   ├── analysis/
│           │   │   └── [analysisId]/
│           │   │       └── page.tsx
│           │   ├── settings/
│           │   │   └── page.tsx
│           │   ├── help/
│           │   │   └── page.tsx
│           │   └── api/
│           │       ├── repositories/
│           │       │   ├── route.ts
│           │       │   └── [repositoryId]/
│           │       │       └── route.ts
│           │       ├── agent/
│           │       │   └── route.ts
│           │       ├── analysis/
│           │       │   └── [analysisId]/
│           │       │       └── route.ts
│           │       ├── operations/
│           │       │   └── [operationId]/
│           │       │       ├── route.ts
│           │       │       └── events/
│           │       │           └── route.ts
│           │       └── memory/
│           │           └── route.ts
│           │
│           ├── components/
│           │   ├── shell/
│           │   │   ├── app-shell.tsx
│           │   │   └── sidebar.tsx
│           │   ├── chat/
│           │   │   ├── chat-panel.tsx
│           │   │   ├── conversation-list.tsx
│           │   │   ├── conversation-search-modal.tsx
│           │   │   ├── message-list.tsx
│           │   │   ├── message-renderer.tsx
│           │   │   └── chat-composer.tsx
│           │   ├── execution/
│           │   │   ├── execution-timeline.tsx
│           │   │   ├── engine-accordion.tsx
│           │   │   └── operation-status.tsx
│           │   ├── analysis/
│           │   │   ├── summary-panel.tsx
│           │   │   ├── impact-card.tsx
│           │   │   ├── risk-card.tsx
│           │   │   ├── affected-files-panel.tsx
│           │   │   ├── test-impact-panel.tsx
│           │   │   └── recommendations-panel.tsx
│           │   ├── graph/
│           │   │   ├── dependency-graph.tsx
│           │   │   └── graph-path-panel.tsx
│           │   ├── evidence/
│           │   │   └── evidence-panel.tsx
│           │   ├── memory/
│           │   │   ├── memory-inspector.tsx
│           │   │   └── handoff-status.tsx
│           │   ├── repository/
│           │   │   ├── repository-card.tsx
│           │   │   ├── repository-list.tsx
│           │   │   └── repository-setup-form.tsx
│           │   ├── settings/
│           │   │   └── settings-navigation.tsx
│           │   └── common/
│           │       ├── error-state.tsx
│           │       ├── empty-state.tsx
│           │       ├── loading-state.tsx
│           │       └── completeness-notice.tsx
│           │
│           └── lib/
│               ├── shared/
│               │   ├── types.ts
│               │   └── status.ts
│               └── server/
│                   ├── agent/
│                   │   ├── runtime.ts
│                   │   ├── provider.ts
│                   │   ├── prompts.ts
│                   │   └── tool-loop.ts
│                   ├── tools/
│                   │   ├── registry.ts
│                   │   ├── catalog.ts
│                   │   ├── schemas.ts
│                   │   ├── executor.ts
│                   │   ├── guardrails.ts
│                   │   └── implementations/
│                   │       ├── get-repository-context.ts
│                   │       ├── search-code.ts
│                   │       ├── get-symbol.ts
│                   │       ├── query-code-relationships.ts
│                   │       ├── get-historical-changes.ts
│                   │       ├── get-test-intelligence.ts
│                   │       ├── analyze-change.ts
│                   │       ├── get-impact.ts
│                   │       ├── get-risk.ts
│                   │       ├── get-test-impact.ts
│                   │       └── get-evidence.ts
│                   ├── workflows/
│                   │   ├── prepare-repository.ts
│                   │   ├── investigate-change.ts
│                   │   └── finalize-review.ts
│                   ├── memory/
│                   │   ├── service.ts
│                   │   ├── policy.ts
│                   │   ├── retrieval.ts
│                   │   └── freshness.ts
│                   ├── handoffs/
│                   │   ├── service.ts
│                   │   └── schema.ts
│                   ├── operations/
│                   │   ├── service.ts
│                   │   ├── state.ts
│                   │   ├── events.ts
│                   │   └── sse.ts
│                   ├── results/
│                   │   ├── canonical-json.ts
│                   │   ├── validate.ts
│                   │   ├── llm-projection.ts
│                   │   ├── llm-encoding.ts
│                   │   ├── llm-encoding.test.ts
│                   │   └── ui-projection.ts
│                   ├── rust-client/
│                   │   ├── client.ts
│                   │   ├── progress-stream.ts
│                   │   └── generated/       # Generated; never hand-edit
│                   ├── auth/
│                   │   ├── context.ts
│                   │   └── authorize.ts
│                   └── config/
│                       └── env.ts
│
├── proto/
│   └── codemind/
│       └── v1/
│           ├── common.proto
│           ├── repository.proto
│           ├── intelligence.proto
│           ├── memory.proto
│           └── operation.proto
│
├── rust/
│   ├── Cargo.toml
│   └── crates/
│       └── codemind-engine/
│           ├── Cargo.toml
│           ├── build.rs
│           ├── src/
│           │   ├── main.rs
│           │   ├── lib.rs
│           │   ├── config.rs
│           │   ├── transport/
│           │   │   ├── mod.rs
│           │   │   ├── server.rs
│           │   │   ├── progress.rs
│           │   │   └── conversions.rs
│           │   ├── contract/
│           │   │   ├── mod.rs
│           │   │   ├── context.rs
│           │   │   ├── repository.rs
│           │   │   ├── intelligence.rs
│           │   │   ├── memory.rs
│           │   │   ├── operation.rs
│           │   │   ├── result.rs
│           │   │   ├── events.rs
│           │   │   └── error.rs
│           │   ├── source/
│           │   │   ├── mod.rs
│           │   │   ├── provider.rs
│           │   │   ├── snapshot.rs
│           │   │   ├── manifest.rs
│           │   │   ├── language.rs
│           │   │   ├── limits.rs
│           │   │   └── tree_sitter.rs
│           │   ├── engines/
│           │   │   ├── mod.rs
│           │   │   ├── pipeline.rs
│           │   │   ├── coie/
│           │   │   │   ├── mod.rs
│           │   │   │   ├── engine.rs
│           │   │   │   ├── model.rs
│           │   │   │   ├── graph_builder.rs
│           │   │   │   ├── extraction/
│           │   │   │   │   ├── mod.rs
│           │   │   │   │   ├── normalize.rs
│           │   │   │   │   ├── validate.rs
│           │   │   │   │   ├── resolve.rs
│           │   │   │   │   └── evidence.rs
│           │   │   │   ├── languages/
│           │   │   │   │   ├── mod.rs
│           │   │   │   │   ├── rust.rs
│           │   │   │   │   ├── javascript.rs
│           │   │   │   │   ├── typescript.rs
│           │   │   │   │   └── python.rs
│           │   │   │   └── queries/
│           │   │   │       ├── mod.rs
│           │   │   │       ├── symbols.rs
│           │   │   │       ├── search.rs
│           │   │   │       ├── relationships.rs
│           │   │   │       ├── paths.rs
│           │   │   │       └── history.rs
│           │   │   ├── tie/
│           │   │   │   ├── mod.rs
│           │   │   │   ├── engine.rs
│           │   │   │   ├── model.rs
│           │   │   │   ├── graph_builder.rs
│           │   │   │   ├── discovery/
│           │   │   │   │   ├── mod.rs
│           │   │   │   │   ├── rust_tests.rs
│           │   │   │   │   ├── javascript_tests.rs
│           │   │   │   │   ├── typescript_tests.rs
│           │   │   │   │   └── python_tests.rs
│           │   │   │   ├── mapping/
│           │   │   │   │   ├── mod.rs
│           │   │   │   │   ├── target_mapping.rs
│           │   │   │   │   └── evidence.rs
│           │   │   │   └── queries/
│           │   │   │       ├── mod.rs
│           │   │   │       └── tests.rs
│           │   │   └── chie/
│           │   │       ├── mod.rs
│           │   │       ├── engine.rs
│           │   │       ├── model.rs
│           │   │       ├── change_set.rs
│           │   │       ├── graph_builder.rs
│           │   │       ├── impact/
│           │   │       │   ├── mod.rs
│           │   │       │   ├── traversal.rs
│           │   │       │   └── aggregation.rs
│           │   │       ├── risk/
│           │   │       │   ├── mod.rs
│           │   │       │   ├── factors.rs
│           │   │       │   └── assessment.rs
│           │   │       ├── test_impact.rs
│           │   │       ├── evidence.rs
│           │   │       └── queries.rs
│           │   ├── graph_storage/
│           │   │   ├── mod.rs
│           │   │   ├── port.rs
│           │   │   └── falkordb/
│           │   │       ├── mod.rs
│           │   │       ├── client.rs
│           │   │       ├── schema.rs
│           │   │       ├── read.rs
│           │   │       ├── write.rs
│           │   │       └── mapping.rs
│           │   ├── memory/
│           │   │   ├── mod.rs
│           │   │   ├── model.rs
│           │   │   ├── service.rs
│           │   │   ├── scope.rs
│           │   │   ├── validation.rs
│           │   │   ├── repository.rs
│           │   │   └── handoffs.rs
│           │   └── lifecycle/
│           │       ├── mod.rs
│           │       ├── operation.rs
│           │       ├── progress.rs
│           │       ├── events.rs
│           │       └── recovery.rs
│           └── tests/
│               ├── canonical_graph.rs
│               ├── language_adapters.rs
│               ├── falkordb_integration.rs
│               ├── grpc_contract.rs
│               ├── chie_impact.rs
│               ├── tie_discovery.rs
│               ├── memory_persistence.rs
│               ├── handoff_lifecycle.rs
│               └── operation_recovery.rs
│
├── tests/
│   ├── e2e/
│   │   ├── track01.spec.ts
│   │   ├── track02.spec.ts
│   │   ├── polyglot-workflow.spec.ts
│   │   └── reconnect-recovery.spec.ts
│   ├── language-adapters/
│   │   ├── rust-adapter.spec.ts
│   │   ├── typescript-adapter.spec.ts
│   │   └── python-adapter.spec.ts
│   ├── graph-conformance/
│   │   ├── canonical-model.spec.ts
│   │   ├── relationship-semantics.spec.ts
│   │   └── snapshot-isolation.spec.ts
│   ├── memory/
│   │   ├── persistence.spec.ts
│   │   ├── handoff.spec.ts
│   │   └── scope-isolation.spec.ts
│
├── docs/
│   ├── architecture.md
│   ├── testing.md
│   ├── design-system.md
│   ├── demo.md
│   ├── implementation-plan.md
│   └── design/
│
└── scripts/
    ├── setup.sh
    ├── generate-proto.sh
    ├── run-e2e.sh
    └── verify-clean-start.sh
```


## 2.1 Environment variables and repeatable commands

`.env.example` must document the variables the implementation actually needs. Keep values blank/example-only and validate them from `apps/web/src/lib/server/config/env.ts` and `engine/src/config.rs`.

| Variable | Consumer | Purpose |
|---|---|---|
| `FALKORDB_URL` | Rust | FalkorDB connection URL for the storage adapter |
| `RUST_ENGINE_GRPC_URL` | Next.js | Rust gRPC service endpoint |
| `LLM_PROVIDER` | Next.js | Select the one configured LLM provider |
| `LLM_API_KEY` | Next.js | Server-only provider credential; never expose to the browser |
| `AUTH_SECRET` | Next.js | Server-side signing/authorization configuration where needed |

Add repeatable pnpm scripts for web development, TypeScript checks, lint/tests, Protobuf generation and E2E. Keep Rust commands under Cargo and use the scripts directory for multi-step local setup/reset/rehearsal commands. The root `playwright.config.ts` configures browser/API E2E tests; `apps/web/vitest.config.ts` is for fast application/module/component tests. Do not introduce a second test-runner stack without a concrete need.

### Required LLM-result encoding dependencies

Declare these in **`apps/web/package.json` dependencies**, because the code runs in the server-side JavaScript Agent Runtime:

- `flat` (planning baseline: `^6.0.1`) for reversible nested-object flattening and unflattening.
- `@toon-format/toon` (planning baseline: `^4.4.0`) for the official TOON encoder/decoder.

Resolve and commit the actual versions in `pnpm-lock.yaml` at implementation time. These dependencies are not Rust crates, not browser dependencies, and not optional formatting experiments. Do not implement a custom JSON-to-TOON serializer. The encoder integration lives in `apps/web/src/lib/server/results/llm-encoding.ts` and is shared by the JavaScript semantic tools/tool executor.

### Frozen LLM-result encoding contract

For every successful LLM-facing Type B semantic result, the server-side
JavaScript path is:

``` text
Rust Type B canonical JSON
        ↓
parse + validate
        ↓
bounded LLM projection
        ↓
`flat`
        ↓
`@toon-format/toon`
        ↓
TOON text as tool-result content
        ↓
LLM
```

The canonical result is never mutated. The UI receives structured canonical/UI
DTO data, while the LLM receives the bounded TOON representation. Tool-call
arguments, Protobuf envelopes, progress events, diagnostics, persisted
canonical results and UI DTOs remain JSON/typed contracts.

The `flat` configuration must preserve arrays and array contents, use a
collision-safe reversible key strategy, and pass `unflatten` structural-
equality tests for all supported result shapes. Do not flatten arrays of
records into numeric-indexed keys merely to satisfy a mechanical definition of
flat, because that can destroy the repeated-record structure TOON can encode
efficiently.

The LLM must never receive JSON text as a fallback for a successful Type B
result. If flattening or integrity validation fails, the tool must surface a
structured formatting/contract error and preserve the canonical result for the
UI and diagnostics rather than silently bypassing the approved pipeline.

This decision is based on the current TOON benchmark and our CodeMind-specific
need, but the benchmark does not establish a universal token reduction. The
implementation must measure compact JSON, direct TOON, flattened JSON and
flattened TOON on representative CodeMind payloads and record observed token
counts and field-retrieval correctness.

### Structure notes

- The directory list is the intended module layout, not a requirement to create all files on Day 1.
- If an agreed file can share a few tightly coupled private helpers without weakening the contracts, avoid splitting it into trivial one-function files. Do not, however, merge CoIE, TIE and ChIE into one undifferentiated module.
- `chie/simulation/` and `simulate_change()` are deliberately absent from the active tree because hypothetical simulation is deferred.
- Rich commit/PR/issue/developer-history ingestion is not a required module. `queries/history.rs` provides only the bounded history/snapshot capability that can be supported with actual data; broad history stays cuttable.
- `get_historical_changes` remains in the semantic catalog, but must return real available history or an explicit unavailable/unsupported outcome. It must not fabricate historical data.
- If a specific framework/parser proves incompatible, preserve its adapter interface and report the unsupported subset honestly rather than pretending to support it.
- Use the actual approved visual direction and `design-system.md` tokens. Do not implement the UI screenshot's illustrative counts or numeric risk percentages as hardcoded data.

---

## 3. File ownership and implementation contract

### 3.1 Root files

| File | Responsibility | First touched |
|---|---|---|
| `AGENTS.md` | Hard project constraints, directory ownership, test-before-completion rules, untrusted-repository policy, no scope creep | Day 1, Stage 0 |
| `README.md` | Initially minimal setup notes; completed for submission on Day 4 | Day 1, then Day 4 |
| `docs/implementation-plan.md` | This sequenced build plan | Planning artifact |
| `package.json` | Root pnpm scripts and workspace-level commands only | Day 1 |
| `pnpm-workspace.yaml` | Registers `apps/web` | Day 1 |
| `.env.example` | Documents required configuration names with safe placeholders | Day 1 |
| `.gitignore` | Excludes secrets, build artifacts, local source copies and generated state as appropriate | Day 1 |
| `docker-compose.yml` | FalkorDB and persistent named volume; no extra services | Day 1 |

`AGENTS.md` must reinforce these constraints: one Next.js app, one Rust service, no direct LLM/Next.js database access, no arbitrary Cypher or shell execution, no fake graph facts, and tests run before a stage is called complete.

### 3.2 Next.js and Agent Runtime files

| File/module | Responsibility |
|---|---|
| `src/app/page.tsx` | Chat-first welcome experience and entry into the demo workspace |
| `src/app/repositories/page.tsx` | Repository list and indexing status |
| `src/app/repositories/new/page.tsx` | Add repository / choose example / bounded public GitHub intake |
| `src/app/repositories/[repositoryId]/page.tsx` | Repository context and readiness |
| `src/app/analysis/[analysisId]/page.tsx` | Expandable investigation workspace and structured tabs |
| `src/app/settings/page.tsx` | A compact settings page; no provider secrets or advanced engine internals required for the MVP |
| `src/app/help/page.tsx` | Basic help and how the engines work; cut before core workflows if needed |
| API Route Handlers | Parse/authenticate/authorize/validate requests, call server modules, return JSON or SSE; no graph algorithms |
| `agent/runtime.ts` | Coordinates the LLM/tool loop and investigation workflow |
| `agent/provider.ts` | One configured LLM provider behind a small interface |
| `agent/prompts.ts` | System/developer instructions requiring tool-grounded conclusions and honest limitations |
| `agent/tool-loop.ts` | Bounded tool-call loop, deadlines and exhaustion behavior |
| `tools/catalog.ts` | Single catalog of the semantic tools visible to the LLM |
| `tools/schemas.ts` | Runtime input validation and provider tool schema derived from the same typed definitions |
| `tools/registry.ts` | Maps stable tool identifiers to implementations |
| `tools/executor.ts` | Validates, authorizes, limits, calls Rust and validates canonical results |
| `tools/guardrails.ts` | Tool budget, deadline, per-tool timeout, result limits, duplicate-call behavior and loop exhaustion |
| `workflows/prepare-repository.ts` | Snapshot preparation → CoIE → TIE → graph readiness |
| `workflows/investigate-change.ts` | CoIE context → TIE → ChIE → semantic result retrieval |
| `workflows/finalize-review.ts` | Store episode/finding, propose bounded memory and coordinate handoff |
| `memory/*.ts` | Runtime retrieval/proposal/policy/freshness checks only; Rust owns durable persistence |
| `handoffs/*.ts` | Create, retrieve, claim and consume persistent handoffs through typed Rust APIs |
| `operations/*.ts` | Application coordination, event ordering, reconnect cursor and SSE projection |
| `results/canonical-json.ts`, `validate.ts` | Parse the gRPC JSON-string payload and validate canonical result semantics before any projection |
| `results/llm-projection.ts` | Build a bounded model-specific projection without dropping identity, evidence, outcome, completeness, freshness or limitations |
| `results/llm-encoding.ts` | Use `flat` with collision-safe reversible key handling and array preservation, then encode with `@toon-format/toon`; provide TOON text to the LLM; reject non-reversible projections rather than silently bypassing `flat` |
| `results/ui-projection.ts` | Produce structured UI-safe results from the canonical parsed JSON, never from flattened JSON or TOON text |
| `rust-client/*.ts` | Only application-side gRPC boundary; never expose generated transport types directly to React components |
| `auth/*.ts` | Bind trusted caller/workspace/repository scope to tool calls |
| `config/env.ts` | Load and validate environment once, server-side |

### 3.3 Rust files and engine ownership

| Area | Files own |
|---|---|
| `main.rs`, `lib.rs`, `config.rs` | Service bootstrap, reusable engine library, validated runtime configuration |
| `transport/*` | gRPC service endpoints, typed progress streaming and conversion to/from generated contracts |
| `contract/*` | Domain-owned request/result/context/event/error types; keep these independent from UI types |
| `source/*` | Repository-provider boundary, immutable snapshot, manifest, file inventory, language detection, limits and Tree-sitter integration |
| `engines/pipeline.rs` | Controlled `CoIE → TIE → ChIE` pipeline for the guaranteed hackathon path |
| `engines/coie/*` | Canonical code facts, language extraction, normalization, relationship validation/resolution, code graph writes and bounded code queries |
| `engines/tie/*` | Static test discovery, test/test-suite model, supported test-to-code associations, test graph writes and test queries |
| `engines/chie/*` | ChangeSet/AnalysisRun, direct/indirect impact, qualitative risk factors, test recommendations, evidence and change graph |
| `graph_storage/port.rs` | Database-independent storage operations required by the engines |
| `graph_storage/falkordb/*` | FalkorDB client, schema/bootstrap, parameterized read/write queries and mapping of graph rows to domain types |
| `memory/*` | Durable memory entities, provenance/scope validation, memory writes/retrieval and persisted handoff operations |
| `lifecycle/*` | Recoverable operation state, typed stage events, stable event IDs/sequence and reconnect recovery |

Do not let engine code build ad hoc Cypher strings. Graph query text belongs within the FalkorDB adapter. `Type A` functions perform graph construction/maintenance and return build/update reports; `Type B` / `Type 2` functions perform graph reading, bounded analysis and result preparation. Both use the same storage boundary.

### 3.4 Protobuf source contracts

| File | Contract responsibility |
|---|---|
| `common.proto` | Shared identifiers/context, typed outcomes, completeness/freshness and common metadata |
| `repository.proto` | Repository registration/resolution, snapshot context, indexing request/report |
| `intelligence.proto` | Typed CoIE/TIE/ChIE invocations, semantic query inputs and canonical result payloads |
| `memory.proto` | Episode/finding/memory/handoff persistence and retrieval operations |
| `operation.proto` | Operation state, engine stage status, progress event envelope, event sequence and recovery snapshot |

The exact field names can be settled when the first service contract is compiled, but the semantics above cannot be weakened. Do not duplicate these contracts in manually maintained TypeScript interfaces. Generated bindings must be regenerated from the `.proto` source.

### 3.5 Tool catalog for the hackathon

Implement the initial **11 semantic tools**:

1. `get_repository_context`
2. `search_code`
3. `get_symbol`
4. `query_code_relationships`
5. `get_historical_changes`
6. `get_test_intelligence`
7. `analyze_change`
8. `get_impact`
9. `get_risk`
10. `get_test_impact`
11. `get_evidence`

`simulate_change` remains in the architectural catalog only and is deferred. Graph-path retrieval is served by bounded `query_code_relationships` and evidence results; do not add a parallel tool solely for a UI tab. The catalog should expose one schema/definition per tool and keep the UI independent from private Rust methods.

### 3.6 Persisted domain types required for Track 02

The MVP persistence contract must cover:

- `ConversationSession`
- `ConversationTurn`
- `ReviewEpisode`
- `ReviewFinding`
- `SemanticMemory`
- `ProcedureMemory`
- `Handoff`
- `EngineReference`

`ReviewFeedback` is optional if the core persistence path is at risk. Each persistent memory object needs scope, provenance, source review/evidence references and lifecycle/freshness metadata. Operation state is separate from memory. A handoff is structured and persisted; the receiving stage must read it back rather than receiving only an in-memory object.

---

# Part II — Dependency-Driven Four-Day Plan

## 4. Implementation order at a glance

```text
Day 1 — Foundation
Environment + contracts
    ↓
FalkorDB storage port/adapter
    ↓
Source snapshot + three minimal language adapters
    ↓
Canonical Code Graph + basic Test Graph
    ↓
gRPC + Next.js query vertical slice
    ↓
Day 1 gate

Day 2 — Track 01
Bounded graph queries
    ↓
TIE mappings + ChIE impact/risk/evidence
    ↓
Semantic tool registry + LLM loop
    ↓
Real multi-hop investigation
    ↓
Structured chat result + progress
    ↓
Track 01 gate

Day 3 — Track 02 + UI
Persist episodes/findings
    ↓
Semantic/procedure memory
    ↓
Persistent handoff consumed by next stage
    ↓
Fresh-session retrieval + scope/freshness checks
    ↓
Complete functional chat workspace
    ↓
Track 02 gate

Day 4 — Freeze and deliver
Fresh start
    ↓
Full three-language E2E
    ↓
Track 01 + Track 02 rehearsals
    ↓
Failure/reconnect/isolation checks
    ↓
README/docs/demo video/public submission
```

Never invert this dependency order because a UI feature appears easier to implement.

---

# 5. Day 1 — Foundation and first vertical slice

**Goal:** Given source from a developer-supplied supported repository, CodeMind builds canonical graph facts, persists them in FalkorDB, and retrieves real facts through the actual Rust → gRPC → Next.js path.

Do not spend Day 1 polishing the agent UI. Do not start memory before the graph slice works.

## Stage 1.0 — Establish the runnable workspace

**Files**

- `AGENTS.md`
- `package.json`
- `pnpm-workspace.yaml`
- `apps/web/package.json` (including `flat` and `@toon-format/toon` server dependencies)
- `pnpm-lock.yaml` (generated by pnpm; never hand-edit)
- `apps/web/next.config.ts`
- `apps/web/tsconfig.json`
- `engine/Cargo.toml`
- `engine/Cargo.toml`
- `engine/src/main.rs`
- `engine/src/lib.rs`
- `engine/src/config.rs`
- `docker-compose.yml`
- `.env.example`
- `.gitignore`
- `scripts/setup.sh`

**Order**

1. Add the root pnpm workspace and the single `apps/web` app.
2. Add the Rust Cargo workspace with one `codemind-engine` crate.
3. Add the minimum Compose definition for FalkorDB, including a persistent named volume.
4. Add validated environment loading and placeholders. Never commit credentials.
5. Add `AGENTS.md` constraints before using coding agents to implement more files.
6. Prove each of the three primary processes can start or at least compile: FalkorDB, Rust, Next.js.

**Gate:** `docker compose up` brings up FalkorDB; Rust compiles; `pnpm --filter web dev` starts Next.js; restart does not erase the FalkorDB volume.

## Stage 1.1 — Define and generate the transport contracts

**Files**

- `proto/codemind/v1/common.proto`
- `proto/codemind/v1/repository.proto`
- `proto/codemind/v1/intelligence.proto`
- `proto/codemind/v1/memory.proto`
- `proto/codemind/v1/operation.proto`
- `engine/build.rs`
- `engine/src/contract/mod.rs`
- `engine/src/contract/context.rs`
- `engine/src/contract/repository.rs`
- `engine/src/contract/intelligence.rs`
- `engine/src/contract/result.rs`
- `engine/src/contract/error.rs`
- `scripts/generate-proto.sh`
- `apps/web/src/lib/server/rust-client/generated/` (generated output)

**Order**

1. Define shared repository/snapshot context first.
2. Define a canonical response envelope carrying outcome, completeness, freshness, context, data, evidence references and diagnostics/limitations.
3. Carry the canonical data result as JSON text in the agreed string field of the typed Protobuf response. Protobuf stays the transport envelope; TOON is not a gRPC or Rust canonical-result format.
4. Define repository/snapshot requests and the minimum typed engine/tool request-response contract.
5. Define operation/progress events with `operation_id`, `request_id`, `trace_id`, `repository_id`, `engine`, `stage`, stable event identity/sequence and timestamp.
6. Add memory/handoff message types before Day 3, but test code generation early.
7. Generate Rust and TypeScript bindings. Do not hand-edit generated files.
8. Compile a minimal typed gRPC request/response path before building graph logic on top.

**Gate:** Protobuf generation is repeatable, Rust and TypeScript compile against generated contracts, and a small contract test passes.

## Stage 1.2 — Implement the graph-storage boundary before engine queries

**Files**

- `engine/src/graph_storage/mod.rs`
- `engine/src/graph_storage/port.rs`
- `engine/src/graph_storage/falkordb/mod.rs`
- `engine/src/graph_storage/falkordb/client.rs`
- `engine/src/graph_storage/falkordb/schema.rs`
- `engine/src/graph_storage/falkordb/read.rs`
- `engine/src/graph_storage/falkordb/write.rs`
- `engine/src/graph_storage/falkordb/mapping.rs`
- `engine/src/contract/error.rs`

**Order**

1. Define the smallest domain-independent storage operations the engines need.
2. Implement FalkorDB connection/configuration and error conversion.
3. Add parameterized read/write helpers and a schema/bootstrap path.
4. Keep Cypher inside the adapter. No FalkorDB query strings in Next.js, the LLM tool code or React components.
5. Use idempotent upsert/merge behavior for canonical entities and relationships.
6. Add a disposable test graph or namespaced workspace test context so integration tests cannot corrupt demo data.

**Gate:** A Rust integration test writes a small known graph, queries it back and verifies workspace scoping and repeat-write idempotency.

## Stage 1.3 — Establish repository snapshots and supported-language source analysis

**Files**

- `engine/src/source/mod.rs`
- `engine/src/source/provider.rs`
- `engine/src/source/snapshot.rs`
- `engine/src/source/manifest.rs`
- `engine/src/source/language.rs`
- `engine/src/source/limits.rs`
- `engine/src/source/tree_sitter.rs`

**Order**

1. Resolve a stable repository identity and immutable `sourceSnapshotId`.
2. Inventory repository files using repository-relative paths, language and provenance metadata.
3. Detect Rust, JavaScript, TypeScript and Python files.
4. Apply repository ignore rules and source-size limits.
5. Keep source snapshots isolated by repository/workspace.
6. Make the source provider work against an arbitrary developer-supplied repository.
7. Use temporary test repositories/source trees for deterministic adapter tests; do not create a product-level fixture repository.

**Gate:** A supported repository can be resolved and snapshotted without requiring a predefined CodeMind demo repository.

## Stage 1.4 — Implement the minimum CoIE extraction path for all three languages

**Files**

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

**Initial extraction subset**

- files and directories,
- modules/packages where supportable,
- functions/methods/classes/structs/traits/interfaces/enums or other agreed declarations,
- source ranges,
- imports/exports,
- direct calls only where the target is resolvable,
- diagnostics for syntax failures or unsupported constructs.

**Rules**

- All adapters emit one canonical candidate contract.
- Shared validation/identity assignment runs after extraction.
- Adapters never write directly to FalkorDB.
- Do not make a graph edge because two names look similar.
- Preserve unresolved status and source evidence.

**Gate:** All three language test inputs produce source-located canonical files/symbols and supported relationships; unsupported constructs are reported rather than invented.

## Stage 1.5 — Persist the Code Graph and build the basic Test Graph

**Files**

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

**Order**

1. Persist the CoIE Code Graph through the storage port.
2. Add bounded `search_code`, `get_symbol` and relationship queries.
3. Add TIE static discovery for Rust `#[test]`, a useful subset of Jest/Vitest, and pytest/unittest.
4. Preserve `DECLARED_BY` and evidence-backed `TARGETS`; do not invent `EXERCISES` or `COVERS` without real runtime/coverage evidence.
5. Deduplicate test entities/recommendations by canonical identity.
6. Preserve snapshot compatibility between a test mapping and its CoIE target.
7. Report `supported_no_results`, `partial`, `unsupported` and `failed` distinctly.

**Gate:** FalkorDB contains real nodes and relationships from all supported languages; a query can start at a selected symbol and retrieve linked symbols and relevant test candidates.

## Stage 1.6 — Publish a real query vertically through gRPC and Next.js

**Files**

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

**Vertical slice**

```text
Open supported repository
    ↓
resolve repository snapshot
    ↓
CoIE extraction / TIE discovery
    ↓
write Code/Test Graph to FalkorDB
    ↓
select a symbol
    ↓
query bounded relationships through Rust
    ↓
typed gRPC response
    ↓
Next.js Route Handler / server client
    ↓
render actual entities in UI
```

**Gate:** This works with the real FalkorDB adapter; it is not a JSON mock or hardcoded response. The app can be restarted and still query the persistent graph.

## Day 1 required tests

- Rust unit tests for canonical identity/normalization.
- Adapter conformance tests for Rust, TypeScript/JavaScript and Python.
- Canonical graph shape/relationship tests.
- FalkorDB integration tests.
- Ingestion idempotency.
- Snapshot/workspace isolation.
- Basic gRPC contract test.
- Basic Route Handler → Runtime/client test.
- At least one graph query that demonstrably depends on a persisted relationship.

## Day 1 exit gate

Do not begin advanced UI or memory until every required item passes:

- [ ] FalkorDB starts and persists data.
- [ ] Rust service starts.
- [ ] Next.js starts.
- [ ] Protobuf/gRPC request and response work.
- [ ] Source snapshot/provenance is recorded.
- [ ] Rust adapter works.
- [ ] TypeScript/JavaScript adapter works.
- [ ] Python adapter works.
- [ ] Canonical Code Graph is persisted.
- [ ] Basic Test Graph and supported test mappings are persisted.
- [ ] A symbol/relationship query returns real results.
- [ ] Repeated ingestion is safe.
- [ ] Workspace isolation is tested.
- [ ] Day 1 tests pass.

If this gate fails, cut public GitHub import and optional parsing depth first. Do not proceed by replacing the graph with mock data.

---

# 6. Day 2 — Track 01: Connected-data investigation

**Goal:** A developer can ask “What could this affect?”, the LLM uses the approved semantic tools, Rust retrieves/traverses actual FalkorDB relationships, and CodeMind returns impact, qualitative risk, test candidates and evidence-backed paths.

## Stage 2.1 — Finish bounded Type B query contracts

**Files**

- `engine/src/engines/coie/queries/relationships.rs`
- `engine/src/engines/coie/queries/paths.rs`
- `engine/src/engines/tie/queries/tests.rs`
- `engine/src/contract/result.rs`
- `apps/web/src/lib/server/results/validate.ts`

**Implement**

- query context bound to repository/snapshot/graph versions,
- maximum result count and traversal depth,
- pagination where needed,
- explicit truncation/unsupported/unresolved outcomes,
- evidence references for graph paths,
- canonical result validation at the TypeScript/Rust boundary.

A timeout or truncation must never look like an exhaustive result.

## Stage 2.2 — Implement real ChIE change analysis

**Files**

- `engine/src/engines/chie/mod.rs`
- `engine/src/engines/chie/engine.rs`
- `engine/src/engines/chie/model.rs`
- `engine/src/engines/chie/change_set.rs`
- `engine/src/engines/chie/graph_builder.rs`
- `engine/src/engines/chie/impact/mod.rs`
- `engine/src/engines/chie/impact/traversal.rs`
- `engine/src/engines/chie/impact/aggregation.rs`
- `engine/src/engines/chie/risk/mod.rs`
- `engine/src/engines/chie/risk/factors.rs`
- `engine/src/engines/chie/risk/assessment.rs`
- `engine/src/engines/chie/test_impact.rs`
- `engine/src/engines/chie/evidence.rs`
- `engine/src/engines/chie/queries.rs`
- `engine/src/engines/pipeline.rs`
- `engine/tests/chie_impact.rs`
- `engine/tests/tie_discovery.rs`

**Implement in this order**

1. Resolve the user-selected symbol; fall back to a file only if symbol resolution is unavailable.
2. Pin the source snapshot and compatible CoIE/TIE graph context.
3. Traverse the supported relationships, prioritizing `CALLS` and `IMPORTS`.
4. Classify one-step propagation as direct impact and two-or-more steps as indirect impact.
5. Preserve distinct relationship types, directions and supporting paths.
6. Query TIE mappings to produce evidence-backed test candidates.
7. Deduplicate canonical impacted entities and test recommendations.
8. Create `ChangeSet`, `ChangeRecord`, `AnalysisRun`, `ImpactAnalysis`, `RiskAssessment` and `Evidence` records in the ChIE domain.
9. Link `AFFECTS` to canonical CoIE entities and `RECOMMENDS` to canonical TIE test entities. Do not duplicate their nodes.
10. Publish the Change Graph/AnalysisRun only after required validation succeeds.

### Risk implementation rules

Risk is a deterministic, versioned qualitative assessment—not a made-up calibrated percentage. Implement the five approved factor families:

1. structural exposure,
2. validation evidence,
3. historical instability when evidence exists,
4. explicit criticality/sensitive-boundary policy,
5. supported change characteristics.

Keep impact, risk, confidence, review severity and completeness separate. Missing evidence must not become “safe”, zero or low risk.

### Explicitly not part of Stage 2

- `simulate_change()`
- full Git diff/PR analysis
- arbitrary test execution
- broad commit/issue/PR/developer-history graph
- claims of perfect static resolution for dynamic language constructs

**Gate:** a selected symbol produces direct and indirect impact from a genuine multi-hop FalkorDB traversal, test candidates from TIE, an explainable qualitative risk assessment and supporting evidence.

## Stage 2.3 — Implement the semantic tool registry and guardrails

**Files**

- `apps/web/src/lib/server/tools/catalog.ts`
- `apps/web/src/lib/server/tools/schemas.ts`
- `apps/web/src/lib/server/tools/registry.ts`
- `apps/web/src/lib/server/tools/executor.ts`
- `apps/web/src/lib/server/tools/guardrails.ts`
- all 11 files under `apps/web/src/lib/server/tools/implementations/`
- `apps/web/src/lib/server/rust-client/client.ts`
- `apps/web/src/lib/server/results/llm-projection.ts`
- `apps/web/src/lib/server/results/llm-encoding.ts`
- `apps/web/src/lib/server/results/ui-projection.ts`
- `apps/web/src/lib/server/results/llm-encoding.test.ts`
- `apps/web/package.json` (`flat`, `@toon-format/toon`)

**Implement**

1. Define each tool once with a stable identifier, description, schema, authorization requirements, output contract, time/size limits and Rust mapping.
2. Derive the LLM-facing tool schema and runtime input validation from the same typed definition.
3. Validate input before making any gRPC call.
4. Bind repository, snapshot and authorization context from trusted runtime state, not model-provided identity fields.
5. Centralize call budgets, total deadline, per-tool timeout, duplicate-call behavior, result-size limits and loop exhaustion.
6. Validate the canonical Rust JSON result before projecting it. Preserve outcome, completeness, freshness, snapshot/graph versions, evidence, truncation and diagnostics.
7. Build a bounded LLM projection that keeps relevant entity/evidence references, relationship semantics and context. Canonical JSON remains untouched and feeds the UI projection separately.
8. Use the `flat` npm package on the bounded LLM projection. Configure it to preserve arrays and array contents so findings, affected-file rows, path steps and test candidates remain records for TOON tabular encoding. Use collision-safe reversible path-key transforms; test literal delimiter/dot keys and nested-path collisions.
9. Verify flattening with `unflatten` and structural equality. If a particular projection cannot be flattened without loss or ambiguous keys, fail model-result preparation with a structured formatting/contract error. Do not silently bypass the mandatory `flat` stage.
10. Encode the flattened projection using `@toon-format/toon`. Give the LLM the encoded TOON text as the tool-result content, not a JSON wrapper around the TOON string. Do not hand-write a JSON-to-TOON serializer.
11. Keep this logic in the shared server-only `results/llm-encoding.ts` helper. All JavaScript semantic tools/tool executor must call the shared path rather than implementing their own format conversion.
12. Bound the projection before encoding and never cut the final TOON string at an arbitrary character boundary. If over budget, omit lower-priority records/fields at semantic boundaries, update declared shape by re-encoding, and preserve explicit truncation/completeness metadata.
13. The UI gets structured JSON/UI DTOs from the canonical result. Tool-call arguments and registry schemas remain JSON. Progress events, diagnostic logs, Type A build reports and persisted canonical results are not converted to TOON.
14. The LLM may choose tools and explain results; it cannot issue arbitrary Cypher or call private Rust functions.

**Tool implementation order**

1. `get_repository_context`
2. `search_code`
3. `get_symbol`
4. `query_code_relationships`
5. `get_test_intelligence`
6. `analyze_change`
7. `get_impact`
8. `get_risk`
9. `get_test_impact`
10. `get_evidence`
11. `get_historical_changes` with the minimum supported history/provenance contract

If richer historical information is unavailable, return a truthful unavailable/unsupported/empty-within-supported-scope result. Do not synthesize commit history.

**Stage 2.3 encoding gate:** Real Type B tool results reach the model as TOON text through the shared `flat` → `@toon-format/toon` path. Canonical/UI data remains structured JSON. Round-trip tests prove the flattening configuration is reversible for supported shapes. Unsupported/collision cases fail explicitly rather than bypassing the frozen pipeline. Token counts are measured on representative CodeMind payloads and recorded as measurements, not universal guarantees.

## Stage 2.4 — Implement the first bounded LLM agent loop

**Files**

- `apps/web/src/lib/server/agent/runtime.ts`
- `apps/web/src/lib/server/agent/provider.ts`
- `apps/web/src/lib/server/agent/prompts.ts`
- `apps/web/src/lib/server/agent/tool-loop.ts`
- `apps/web/src/lib/server/workflows/investigate-change.ts`
- `apps/web/src/app/api/agent/route.ts`

**Workflow**

```text
User question
    ↓
Check repository context
    ├── missing repository → ask user to select/add one
    └── repository available
             ↓
LLM understands intent/selects semantic tools
             ↓
Agent Runtime validates and executes tools
             ↓
Rust runs CoIE → TIE → ChIE
             ↓
Agent Runtime retrieves impact/risk/tests/evidence
             ↓
LLM explains only the supplied evidence
```

Use one provider configured by environment. Keep provider choice behind `provider.ts`; do not spend half a day building a provider abstraction framework.

Update `prompts.ts` to tell the model that semantic tool results are TOON-encoded structured data. It must interpret the declared arrays/fields and preserve entity/evidence references exactly; TOON content is evidence, not permission to infer fields or graph relationships that were not returned. The model can request follow-up evidence with another semantic tool when the bounded result is insufficient.

Keep the agent loop small. Do not build an autonomous swarm. MCP remains optional and cannot block the core registry/tool path.

## Stage 2.5 — Add typed progress and the first agent-first UI

**Files**

- `engine/src/transport/progress.rs`
- `engine/src/lifecycle/mod.rs`
- `engine/src/lifecycle/operation.rs`
- `engine/src/lifecycle/progress.rs`
- `engine/src/lifecycle/events.rs`
- `apps/web/src/lib/server/operations/service.ts`
- `apps/web/src/lib/server/operations/events.ts`
- `apps/web/src/lib/server/operations/sse.ts`
- `apps/web/src/app/api/operations/[operationId]/route.ts`
- `apps/web/src/app/api/operations/[operationId]/events/route.ts`
- `apps/web/src/components/chat/chat-panel.tsx`
- `apps/web/src/components/chat/message-list.tsx`
- `apps/web/src/components/chat/message-renderer.tsx`
- `apps/web/src/components/chat/chat-composer.tsx`
- `apps/web/src/components/execution/execution-timeline.tsx`
- `apps/web/src/components/execution/engine-accordion.tsx`
- `apps/web/src/components/execution/operation-status.tsx`
- `apps/web/src/components/analysis/summary-panel.tsx`
- `apps/web/src/components/analysis/impact-card.tsx`
- `apps/web/src/components/analysis/risk-card.tsx`
- `apps/web/src/components/analysis/test-impact-panel.tsx`
- `apps/web/src/components/analysis/affected-files-panel.tsx`
- `apps/web/src/components/evidence/evidence-panel.tsx`
- `apps/web/src/components/graph/graph-path-panel.tsx`

**Important observability contract**

Do not combine unrelated signals into one scraped log stream:

- Rust `tracing` logs are diagnostics.
- Typed progress events drive live operation status.
- `GraphUpdated`/publication events identify committed graph versions.
- UI messages are safe human-readable explanations.

They share correlation metadata such as `operation_id`, `request_id`, `trace_id`, `repository_id`, `engine`, `stage`, sequence/event identity and timestamp.

For the guaranteed route show:

```text
LLM planning → CoIE → TIE → ChIE → LLM reasoning/response
```

Each engine accordion consumes typed progress events. When ChIE publishes the Change Graph, emit a typed completion event; Agent Runtime correlates it and only then retrieves final impact/risk/test/evidence results. The LLM does not wait for an undocumented signal and the browser does not parse logs.

**Gate:** the agent completes one actual investigation from the UI; all findings are backed by the returned canonical result; progress stages reflect real operation events.

## Stage 2.6 — Track 01 integration tests and gate

**Tests**

- semantic tool schema/authorization/limit tests,
- bounded traversal and multi-hop path tests,
- ChIE direct/indirect impact tests,
- risk factor and missing-evidence tests,
- TIE recommendation and dedup tests,
- gRPC integration tests,
- Next.js Runtime/tool integration tests,
- polyglot investigation E2E.

**Day 2 exit gate**

- [ ] User can choose a repository and a symbol.
- [ ] User can ask “What could this affect?”
- [ ] LLM tool calls go through the semantic registry.
- [ ] Rust queries FalkorDB for canonical facts.
- [ ] At least one multi-hop path materially changes the result.
- [ ] CoIE, TIE and ChIE run through controlled typed engine interfaces.
- [ ] ChIE returns impact, qualitative risk, test candidates, evidence and limitations.
- [ ] Result paths are traceable to actual graph relationships.
- [ ] Rust, TypeScript/JavaScript and Python all use the same investigation workflow.
- [ ] The first Track 01 E2E passes.

At this point there must be a credible Track 01 product even before advanced memory is added.

---

# 7. Day 3 — Track 02, persistence and complete functional UI

**Goal:** Prior investigation experience survives across sessions, a receiving stage consumes a persisted handoff, memory is validated against current evidence, and the functional chat-first interface shows the full workflow.

Implement the minimum meaningful memory model. Do not introduce vectors or advanced memory ranking.

## Stage 3.1 — Persist sessions, turns, episodes and findings

**Files**

- `engine/src/contract/memory.rs`
- `engine/src/memory/mod.rs`
- `engine/src/memory/model.rs`
- `engine/src/memory/service.rs`
- `engine/src/memory/scope.rs`
- `engine/src/memory/validation.rs`
- `engine/src/memory/repository.rs`
- `apps/web/src/lib/server/memory/service.ts`
- `apps/web/src/lib/server/workflows/finalize-review.ts`
- `apps/web/src/app/api/memory/route.ts`
- `engine/tests/memory_persistence.rs`

**Persist at least**

- `ConversationSession`
- `ConversationTurn`
- `ReviewEpisode`
- `ReviewFinding`

Each persistent object must include the appropriate repository/project or user/team scope, provenance, source snapshot/graph references, timestamps and lifecycle/freshness metadata.

Do not store unrestricted raw prompts, unbounded tool outputs, secrets or credentials as memory. Keep persisted summaries/findings bounded and evidence-referenced.

## Stage 3.2 — Implement one qualified semantic memory and one procedure

**Files**

- extend `engine/src/memory/model.rs`
- extend `engine/src/memory/service.rs`
- extend `engine/src/memory/validation.rs`
- `apps/web/src/lib/server/memory/policy.ts`
- `apps/web/src/lib/server/memory/retrieval.ts`
- `apps/web/src/lib/server/memory/freshness.ts`

**Requirements**

1. Persist at least one `SemanticMemory`.
2. Persist at least one `ProcedureMemory`.
3. Use structured graph retrieval, not embeddings.
4. Check scope, repository/project, target symbol/test, provenance, validation state and snapshot compatibility.
5. Keep lifecycle separate from freshness:
   - lifecycle: candidate, validated, rejected, superseded;
   - freshness: fresh, stale, unknown.
6. Validate the memory against current engine evidence before supplying it to the LLM.
7. Current evidence always outranks old memory.
8. If the memory is stale, irrelevant or scope-denied, show that state and do not use it as authoritative evidence.

The LLM can propose a memory candidate but cannot directly persist trusted knowledge.

## Stage 3.3 — Implement a persistent, idempotent handoff

**Files**

- `engine/src/memory/handoffs.rs`
- extend `engine/src/contract/memory.rs`
- `apps/web/src/lib/server/handoffs/schema.ts`
- `apps/web/src/lib/server/handoffs/service.ts`
- `apps/web/src/components/memory/handoff-status.tsx`
- `engine/tests/handoff_lifecycle.rs`

**Required flow**

```text
Impact investigation
    ↓
construct typed handoff
    ↓
validate schema, source/target scope, provenance and references
    ↓
persist through Rust → FalkorDB
    ↓
receiving test-investigation stage retrieves it
    ↓
claim/consume with stable idempotency
    ↓
continue with current TIE/graph evidence
```

The receiving stage must read the persisted handoff from storage. Passing an in-memory TypeScript object is not sufficient.

Minimum fields include handoff ID/schema version, operation/review episode IDs, source/target stage, task/findings, evidence references, source snapshot and graph versions, open questions, status, idempotency key, sequence/version and timestamps.

Handoff lifecycle and operation lifecycle remain separate. Duplicate delivery must not silently overwrite an existing handoff.

## Stage 3.4 — Prove fresh-session retrieval and scope isolation

**Files**

- extend `apps/web/src/lib/server/memory/retrieval.ts`
- extend `apps/web/src/lib/server/auth/context.ts`
- extend `apps/web/src/lib/server/auth/authorize.ts`
- `apps/web/src/components/memory/memory-inspector.tsx`
- `engine/tests/memory_persistence.rs`
- `engine/tests/handoff_lifecycle.rs`
- `engine/tests/operation_recovery.rs`

**Acceptance path**

```text
Session A
  → investigate
  → persist episode/finding
  → persist semantic memory/procedure
  → persist handoff
  → end session / restart app

Session B
  → retrieve authorized memory
  → check current graph/snapshot
  → mark applicable or stale
  → use only applicable context
  → conduct a new investigation
```

Test user, repository/project and team/tenant scope decisions explicitly. Scope inheritance is never implicit. A private memory must not be exposed in another scope.

## Stage 3.5 — Complete the functional UI

**Files**

- `apps/web/src/app/layout.tsx`
- `apps/web/src/app/globals.css`
- `apps/web/src/app/page.tsx`
- `apps/web/src/app/repositories/page.tsx`
- `apps/web/src/app/repositories/new/page.tsx`
- `apps/web/src/app/repositories/[repositoryId]/page.tsx`
- `apps/web/src/app/analysis/[analysisId]/page.tsx`
- `apps/web/src/components/shell/app-shell.tsx`
- `apps/web/src/components/shell/sidebar.tsx`
- `apps/web/src/components/repository/repository-card.tsx`
- `apps/web/src/components/repository/repository-list.tsx`
- `apps/web/src/components/repository/repository-setup-form.tsx`
- `apps/web/src/components/analysis/recommendations-panel.tsx`
- `apps/web/src/components/graph/dependency-graph.tsx`
- `apps/web/src/components/common/error-state.tsx`
- `apps/web/src/components/common/empty-state.tsx`
- `apps/web/src/components/common/loading-state.tsx`
- `apps/web/src/components/common/completeness-notice.tsx`
- `apps/web/src/lib/shared/status.ts`

**UI implementation order**

1. Chat-first shell and message rendering.
2. Repository selection and the guaranteed supported-repository path.
3. Execution timeline and expandable CoIE/TIE/ChIE accordions.
4. Summary/impact/risk/test/evidence/path structured components.
5. Partial/truncated/unavailable/failed/reconnecting states.
6. Persistent handoff status and Memory Inspector.
7. Tabs/secondary views for affected files, graph and recommendations.
8. Apply the established CodeMind tokens from `design-system.md`.
9. Add settings/help only after the core walkthrough is functional.

Use actual structured results. The UI must not parse the LLM prose to infer risk or affected counts. Screenshots are visual references, not application data. In particular, **do not implement a fabricated numeric risk percentage**; show a supported qualitative tier and its evidence/limitations.

## Stage 3.6 — Test memory, operations, progress and the full app

**Tests**

- review episode and finding persistence,
- semantic/procedure persistence,
- restart/fresh-session retrieval,
- current-snapshot validation and stale-memory handling,
- handoff persistence/claim/consumption,
- duplicate writes and concurrent writes,
- cross-scope denial,
- at-least-once progress event handling,
- monotonic sequence/duplicate event handling,
- browser disconnect and reconnect,
- full chat/result component tests,
- Track 02 E2E.

**Day 3 exit gate**

- [ ] Review episode persists.
- [ ] Finding persists with evidence references.
- [ ] Semantic memory and procedure persist.
- [ ] At least one handoff persists.
- [ ] Receiving stage retrieves and consumes the persisted handoff.
- [ ] New session retrieves authorized memory.
- [ ] Memory freshness/applicability is checked against current evidence.
- [ ] Stale memory never overrides fresh graph facts.
- [ ] Scope isolation works.
- [ ] Operation state/progress survives browser disconnect and supports reconnect.
- [ ] The chat UI displays structured analysis and explicit limitations.
- [ ] Track 02 E2E passes.

If this gate fails, cut advanced memory policy and secondary UI polish; do not replace persistent memory or handoff with a mock.

---

# 8. Day 4 — Feature freeze, reliability and submission

**Hard rule:** No new major features on Day 4. Work only on bugs, integration failures, security, reliability, performance smoke tests, docs, demo and submission.

## Stage 4.1 — Start from a clean environment

**Files**

- `scripts/verify-clean-start.sh`
- `scripts/setup.sh`
- `README.md`
- `.env.example`
- `docker-compose.yml`

Run the project from a fresh checkout or clean environment, with no undocumented local state. Verify:

```text
FalkorDB
  → Rust engine
  → Next.js
  → test repository/source input
  → ingestion
  → investigation
  → memory/handoff
  → new session
```

The persistent graph volume should be used deliberately. Also test a reset path that removes demo data only when explicitly invoked.

## Stage 4.2 — Three-level engine verification

Run all three engine levels:

1. **Level 1 — Unit/component:** deterministic extraction, normalization, graph semantics, TIE discovery, ChIE impact/risk and contract logic.
2. **Level 2 — Integration:** real FalkorDB, source repository, storage idempotency, gRPC and persistence boundaries.
3. **Level 3 — Pipeline/system:** real repository → CoIE → TIE → ChIE → canonical result → gRPC path.

A green Level 1 suite is not sufficient evidence that the engines work. Level 2 and Level 3 must also pass.

## Stage 4.3 — JavaScript semantic-tool JSON contract verification

For each critical Type B tool, verify that the JavaScript layer receives and validates the canonical JSON result before any LLM encoding. Assert parse success, schema validation, structured object shape, UI projection integrity and explicit failure on malformed input. Then separately assert that the LLM-facing branch produces TOON through `flat → @toon-format/toon`.

## Stage 4.4 — Complete the E2E matrix

**Files**

- `apps/web/src/tests/e2e/track01.spec.ts`
- `apps/web/src/tests/e2e/track02.spec.ts`
- `apps/web/src/tests/e2e/polyglot-workflow.spec.ts`
- `apps/web/src/tests/e2e/reconnect-recovery.spec.ts`
- all adapter, conformance and memory tests
- `apps/web/src/lib/server/results/llm-encoding.test.ts` (flat/unflatten integrity, delimiter/key collisions, preserved arrays, TOON round-trip and representative token measurements)
- `engine/tests/*`

Run the same semantic workflow against supported repositories containing:

1. Rust.
2. TypeScript/JavaScript.
3. Python.

Then rehearse the complete Track 01 and Track 02 stories exactly as documented in `demo.md`.

## Stage 4.5 — Manual E2E and TestSprite verification

**Purpose:** Day 4 must prove the complete product from the user's perspective after the deterministic suites are green.

### Manual E2E

Perform a clean-start journey through repository selection, indexing, CoIE → TIE → ChIE investigation, graph-backed impact/evidence, structured result rendering, memory persistence, handoff persistence/consumption, fresh-session retrieval, current-evidence validation, browser disconnect/reconnect and restart recovery.

### TestSprite

Run TestSprite against the actual implemented web application after unit, integration, engine Level 3 and Playwright tests pass. TestSprite is an additional end-to-end/application verification layer, not a replacement for deterministic Rust or JavaScript contract tests.

TestSprite must cover the critical user journeys, navigation, repository setup, investigation, progress UI, structured result rendering, memory/handoff flow, fresh-session behavior and failure/recovery states. Investigate every failure and classify it as a real defect, environment/test issue, expected limitation or unsupported behavior. Re-run the lower-level tests after fixing real defects.

## Stage 4.6 — Deliberately rehearse failures

Verify at least the following:

| Failure | Expected behavior |
|---|---|
| No network / GitHub unavailable | Local supported-repository analysis still works; public import fails honestly |
| Repository not found or inaccessible | Safe error with recovery action |
| Invalid repository URL | Validation error before fetch |
| Rust service unavailable | Operation reports failure/unavailability; no fake result |
| FalkorDB unavailable | No successful graph claim; safe actionable error |
| LLM provider failure | Graph work may remain available; response-generation failure is distinct |
| Partial indexing/parser error | Graph/result is marked partial with diagnostics |
| Unsupported syntax/framework | Report unsupported subset; do not invent relationships |
| Unresolved symbol/call | Keep unresolved status |
| Traversal limit/time-out | Mark truncated/partial, never complete |
| Browser disconnect | Operation continues; reconnect restores current state |
| Duplicate tool/operation request | Idempotency/duplicate policy prevents duplicate side effects |
| Stale memory | Surface stale state; current evidence remains authoritative |
| Cross-scope memory access | Deny and test the denial |
| Duplicate/concurrent handoff write | No silent overwrite; stable identity and recovery |
| Restart application | Repository graph, memory and handoff remain available in the persistent store |

## Stage 4.7 — Performance smoke tests

Measure, do not guess:

- repository indexing time,
- query/graph traversal time,
- tool end-to-end latency,
- progress update latency,
- memory retrieval latency,
- memory/handoff persistence and recovery;
- compact JSON vs direct TOON vs flattened JSON vs flattened TOON token counts for the same bounded CodeMind result shapes;
- deterministic flatten/unflatten and TOON encode/decode overhead;
- a small controlled LLM field-retrieval smoke test using fixed questions and known expected values against compact JSON, direct TOON and flattened TOON representations of the same payload.

Use the same configured provider/model and equivalent fixed prompt framing for the field-retrieval comparison. Prefer provider-reported input-token usage; if unavailable, use a documented tokenizer as a comparative proxy. Record the model/tokenizer and measured environment. Token savings vary by data shape; report observed results rather than claiming a universal reduction. Compare correctness, not prose style. The comparison validates and tunes the frozen `flat` → TOON pipeline; it does not make flattening optional or change the requirement that successful LLM-facing Type B tool results use `flat` followed by TOON.

## Stage 4.8 — Finish docs and submission

**Files**

- `README.md`
- `docs/architecture.md`
- `docs/implementation-plan.md`
- `docs/testing.md`
- `docs/design-system.md`
- `docs/demo.md`
- 
- 
- 

Ensure the public repository explains:

- the submission targets Track 01 and Track 02, not Track 03, and why FalkorDB is central,
- setup and environment,
- the FalkorDB graph data model,
- node and relationship semantics,
- actual Cypher queries/graph algorithms used by the product,
- Rust engine responsibilities,
- Agent Runtime/tool boundary,
- Track 01 graph-grounded reasoning,
- Track 02 persistent memory/handoff,
- supported language/framework subset and known limitations,
- run/test commands,
- local supported-repository/offline analysis path,
- AI coding-assistant disclosure.

Record the approximately three-minute final demo as one coherent story. Start with the product and the Rust investigation, not an architecture lecture; demonstrate real graph paths and evidence, then continue into the persisted handoff, fresh-session memory validation and the TypeScript/Python proofs. Prepare a concise architecture diagram and judge-question answers in `docs/demo.md`. Verify public repository access, clean setup instructions and working video. Prefer a live deployment only if it is reliable; the documented local setup plus video is the fallback. Do not claim a live deployment if the deliverable is local execution.

**Day 4 exit gate**

- [ ] Fresh-start setup succeeds.
- [ ] Unit/integration/conformance tests pass, including the `flat` → TOON result-encoding suite.
- [ ] Type B tool results sent to the LLM are TOON text; canonical/UI results remain structured JSON.
- [ ] Rust, TypeScript/JavaScript and Python E2E pass.
- [ ] Track 01 rehearsal passes.
- [ ] Track 02 rehearsal passes.
- [ ] Failure/reconnect/isolation/idempotency checks pass.
- [ ] Demo is repeatable without public network access.
- [ ] README and required docs are complete.
- [ ] Actual graph queries/algorithms are documented.
- [ ] AI-assistance disclosure is included.
- [ ] Public repository and demo video are ready for submission.
- [ ] Author can explain and defend the implementation and graph model.

---

# 9. Cut strategy if time is running out

Cut in this order, without renegotiating the architecture:

1. **Public GitHub import.** Retain the real repository-analysis path and its source-derived graph; do not replace it with a bundled demo repository.
2. **MCP integration.** Keep the semantic registry and the same internal tool contracts.
3. **Advanced history.** Retain snapshot/source provenance; drop broad PR/issue/developer/history ingestion.
4. **Advanced memory behavior.** Drop retrieval ranking, automatic promotion, consolidation and analytics; retain episode/finding, one semantic memory, one procedure, a persisted handoff and cross-session retrieval.
5. **UI polish and secondary screens.** Drop animations, elaborate graph interactions, extensive settings/help/search and decorative states; retain the functional chat, progress, structured results and Memory Inspector.
6. **Secondary investigation features.** Keep one excellent symbol-first Track 01 investigation rather than several incomplete investigation modes.

### Protected core — never cut

```text
FalkorDB as the real primary graph database
+ real multi-hop graph reasoning
+ Rust / TypeScript-JavaScript / Python workflow
+ evidence-backed Track 01 investigation
+ persisted handoff actually consumed by a receiving stage
+ cross-session memory validated against fresh evidence
+ usable chat-first agent UI
+ truthful partial/unsupported/error states
+ mandatory `flat` → `@toon-format/toon` Type B result path with canonical/UI JSON preserved
```

If a stage is behind, stop adding new work, report which exit criteria failed, and cut from the list above. Do not hide a failure with mocked numbers, a fake graph or fabricated LLM results.

---

# 10. Implementation invariants

These rules apply on all four days.

1. **One-way dependency:** UI → Route Handler → Agent Runtime → gRPC/Protobuf → Rust → Graph Storage Port → FalkorDB.
2. **No database bypass:** React, Route Handlers, Agent Runtime and LLM code never query FalkorDB directly.
3. **No arbitrary Cypher or shell execution from the LLM.**
4. **CoIE owns canonical code/history facts; TIE owns test facts; ChIE owns change-derived conclusions.**
5. **TIE does not decide change risk; ChIE does not duplicate canonical code/test nodes.**
6. **Type A builds/maintains graphs and returns reports; Type B/Type 2 queries/analyses and returns bounded, evidence-backed semantic results.**
7. **Every engine uses the common operation/progress correlation conventions.** Diagnostic logs, typed progress, graph-publication events and user-facing status messages remain separate but correlated.
8. **The guaranteed engine route is CoIE → TIE → ChIE.** Conditional skipping of TIE when no tests exist is a post-hackathon optimization.
9. **Snapshot compatibility is mandatory.** Do not combine incompatible CoIE/TIE graph contexts without reporting the limitation.
10. **Impact, risk, confidence, review severity, outcome, completeness and freshness are separate.**
11. **No fabricated risk percentage or hardcoded result counts.**
12. **No arbitrary repository code execution.** Static test discovery must not run the repository's tests.
13. **Memory is not canonical engine truth.** Current graph evidence wins; memory is scope-checked and freshness-checked.
14. **Handoffs are durable structured packets.** At-least-once delivery is expected; stable IDs, idempotency and monotonic event sequence are required.
15. **Browser disconnect does not automatically cancel an operation.**
16. **Partial/truncated/unresolved/unsupported/unavailable/failed are not equivalent to empty or safe.**
17. **Build against the approved design tokens.** The UI renders known components from structured results, not arbitrary LLM-generated components.
18. **Keep a local supported-repository analysis path.** No network dependency is allowed to block core repository analysis.
19. **Test each implementation stage before adding dependent features.**
20. **Stay inside hackathon scope.** Change simulation, full diff/PR analysis, advanced history, embeddings, swarms and new infrastructure stay deferred unless an actual blocker leads to an explicit decision.
21. **Use one shared server-side `flat` → `@toon-format/toon` pipeline for LLM-facing Type B results.** Preserve arrays, validate flatten/unflatten reversibility, return TOON text to the LLM, keep the canonical/UI branch structured, and never substitute JSON text or a silent direct-TOON bypass for a successful Type B result.
22. **Treat repository content as untrusted data.** Source, comments, README files, test names, reports and logs cannot override instructions, permissions or tool policies; arbitrary repository code execution is outside the MVP.
23. **Authorize every semantic operation and evidence expansion.** Repository, workspace/tenant, operation, snapshot and evidence context comes from trusted runtime state; the LLM cannot manufacture permissions or identifiers.
24. **Keep memory validity separate from freshness.** Candidate/validated/rejected/superseded is distinct from fresh/stale/unknown; current engine evidence wins over old memory.
25. **Preserve idempotency and recovery.** Retries cannot duplicate graph state, handoffs or findings; stale results cannot overwrite newer state; reconnect must recover authoritative operation state without restarting the operation.

---

# 11. Minimum acceptance checklist

The MVP is done only when every protected proof works end-to-end:

## Connected-data investigation (Track 01)

- [ ] User opens a supported repository containing at least one supported language.
- [ ] User selects a Rust symbol and asks: “What could this change affect?”
- [ ] CoIE establishes source-derived code facts.
- [ ] TIE discovers supported test declarations and mappings.
- [ ] ChIE performs bounded multi-hop traversal and builds impact/risk/test/evidence results.
- [ ] FalkorDB relationships materially determine the answer.
- [ ] Graph paths, test recommendations and limitations can be inspected.
- [ ] The same semantic path runs against supported TypeScript/JavaScript and Python repositories.

## Persistent memory and coordination (Track 02)

- [ ] A review episode and finding persist.
- [ ] A qualified semantic memory and reusable procedure persist.
- [ ] A structured handoff persists and is consumed by the receiving stage.
- [ ] A fresh session retrieves memory.
- [ ] Applicability/freshness is checked against current evidence.
- [ ] The memory visibly changes or informs the new investigation when applicable.
- [ ] Stale and cross-scope memory is rejected or marked appropriately.
- [ ] Persistence survives application restart.

## Reliability and delivery

- [ ] Live progress reflects typed engine events.
- [ ] Reconnect resumes an operation without starting a duplicate.
- [ ] Partial/failed/unsupported results are honest.
- [ ] A local supported-repository path works without a public-network dependency.
- [ ] The LLM-facing Type B result formatter uses `flat` and `@toon-format/toon`; flattening integrity tests and representative token measurements pass.
- [ ] The repository, setup instructions, graph schema/queries, testing guide, demo and AI disclosure are ready.

---

# 12. Final execution rule

**Day 1 proves that repository analysis and the graph are real. Day 2 proves that the agent can reason over the graph. Day 3 proves that the agent remembers and coordinates across sessions. Day 4 proves that the whole product is reliable, testable and submission-ready.**

If those four proofs remain intact, CodeMind meets the intended hackathon scope. Everything else is secondary.
