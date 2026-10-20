# CodeMind Complete Fixing Audit and Implementation Plan

**Purpose:** A source-grounded plan for correcting CodeMind's web product, component system, runtime boundaries, and test coverage.
**Scope:** Full CodeMind codebase review, with UI fidelity as a hard acceptance requirement.
**Working branch reviewed:** `web-audit`
**Reviewed revision:** `0c58ef0c908970e37b5b3c1cdd34d2f0b01b9766`
**Document status:** Plan only. No application logic is changed by this document.

> This audit is based on the repository tree and source files inspected on the web-audit branch, plus the architecture, design-system, implementation, and testing documents. Before implementation, verify that the currently checked-out workspace matches the uploaded 20 October ZIP. If the ZIP contains newer changes, use the ZIP as the source of truth and reconcile the path inventory first.

---

## 1. Objectives and Non-Negotiable Rules

This is not a rewrite plan. CodeMind already has a meaningful Rust engine, a gRPC boundary, a server-side Agent Runtime, semantic tools, operation/progress handling, and several product components. The purpose is to correct what is incomplete, inconsistent, duplicated, or visually different from the product specifications while preserving the working foundations.

### 1.1 Visual fidelity is a release requirement

The supplied design images are authoritative visual references. They are not suggestions.

For every approved reference screen, the final implementation must match the reference at its canonical viewport in:

- Overall composition, content alignment, widths, heights, spacing, and scroll behavior.
- Typography, font family, font weight, line height, wrapping, and hierarchy.
- Exact colors, surfaces, borders, radii, shadows, and separators.
- Sidebar, active navigation, header, cards, tables, tabs, status indicators, forms, icons, and actions.
- Relative placement and size of every visible section, including whitespace.
- Interaction states, open/closed panels, loading, progress, completion, error, empty, partial, and unavailable states.
- Responsive behavior specified by the product and design system.

“100% visual fidelity” means the team must compare actual browser screenshots with the exact selected reference image at a controlled viewport, inspect image overlays/diffs, fix every genuine mismatch, and rerun the comparison. A passing build, a successful API call, or a page that merely looks similar does not satisfy this requirement.

Browser rasterization can create minor antialiasing noise. That does not excuse meaningful differences in layout, color, typography, spacing, component dimensions, or content. Define the environment once and use it consistently.

Do not update screenshot baselines merely to accept an incorrect UI. Do not mask a mismatched section, remove a hard reference element, replace a designed layout with a generic dashboard, or substitute a static mock for working UI.

### 1.2 Architecture boundaries must remain intact

The intended dependency direction is:

> Browser UI → Next.js Route Handler → Agent Runtime → typed gRPC/Protobuf → Rust engine → graph storage port → FalkorDB.

Responsibilities:

- **Next.js UI:** presentation and user interaction.
- **Route Handlers:** request parsing, authentication, authorization, validation, invocation of application code, and response/stream creation.
- **Agent Runtime:** LLM orchestration, semantic tools, workflows, memory policy, handoffs, and application operation coordination.
- **Rust engine:** canonical deterministic code, test, and change intelligence through CoIE, TIE, and ChIE.
- **Protobuf/gRPC:** typed application-to-engine transport contract.
- **FalkorDB adapter/storage port:** graph persistence and query boundary.

The UI must not implement graph reasoning, connect directly to FalkorDB, call Rust directly from the browser, manufacture canonical facts, or duplicate server-side agent policy.

### 1.3 File-change discipline

For every target file:

1. Read the whole file and directly related dependencies.
2. State its current responsibility and observed behavior.
3. Identify a concrete architecture, behavior, or design discrepancy.
4. Make the smallest change that resolves that discrepancy.
5. Update tests at the same time.
6. Run the narrow tests and then affected integration/regression tests.
7. Capture and compare the resulting screen with the exact reference.
8. Record what passed, failed, or could not be run.

Do not modify unrelated files simply because they are nearby. Do not change engine production code to compensate for a web styling or composition issue. Do not weaken or delete tests to obtain a green run.

---

## 2. Reference Set and Design-Source Resolution

### 2.1 Core architecture and implementation documentation

Read and follow these source files:

- **docs/architecture.md**: canonical system responsibilities, dependency direction, scope, semantics, runtime, and engine boundaries.
- **docs/design-system.md**: global tokens, typography, color, spacing, radii, primitive behavior, states, graph/timeline conventions, responsive behavior, and accessibility rules.
- **docs/implementation-plan.md**: original intended implementation sequence and file responsibility.
- **docs/testing.md**: existing verification requirements and engine/runtime contract tests.
- **AGENTS.md**: agent operating rules. It explicitly says the agent implements the author's decisions, does not make product or architecture decisions, and must inspect design-system and screen-specific designs before UI work.

Treat architecture, design-system, screen-specific designs, and test requirements as complementary documents. The global design system does not replace a screen design.

### 2.2 Design assets to inspect and map

All relevant files in docs/design must be inspected at full resolution and entered into a design-to-route matrix. Do not infer authority or discard variant images from filenames alone.

| Reference asset | Required audit mapping |
|---|---|
| CodeMind AI Code Analysis Dashboard.png | Main homepage / initial investigation screen |
| CodeMind Agent Execution Dashboard.png | Live investigation execution |
| CodeMind Pipeline Analysis Dashboard.png | Detailed pipeline workspace |
| CodeMind Pipeline Analysis Dashboard (1).png | Compare visually with the other pipeline image; determine whether it is an alternate, revised, or separate state |
| CodeMind Zero-Risk Analysis Dashboard.png | Determine the exact completed-result screen/state it specifies |
| CodeMind Control Plane Impact Dashboard.png | Map to the relevant impact/results view if it is an approved product target |
| CodeMind Repository Dashboard.png | Repository dashboard |
| CodeMind Repository Setup Dashboard.png | Repository source selection, configuration, indexing, and completion/failure states |
| CodeMind Settings Dashboard.png | Settings workspace |
| CodeMind AI Settings Dashboard.png | Compare with Settings Dashboard and establish which details belong to AI/model settings versus a separate screen variant |
| CodeMind Help & Docs Dashboard.png | Help and documentation |
| CodeMind Conversation Search Interface.png | Conversation search UI and its interaction states |
| CodeMind Error States Dashboard.png | Shared error/recovery states and their route-specific use |
| CodeMind Dark Design System Board.png | Visual token and component-reference support; do not automatically treat it as a product page |
| Developer AI Dashboard UI Mockup.png | Determine whether this is an approved CodeMind design, an earlier exploration, or reference material only |

The current design tree contains more than the six dashboard references previously called out, including a second pipeline image, two settings-related images, a conversation search image, an error-state dashboard, an impact dashboard, and a design-system board. Every asset must be classified before the corresponding work begins.

The inspected revision's docs/design/README.md returned no substantive content. Therefore the design-to-route map needs to be established explicitly and checked by the project owner where a variant or conflict cannot be resolved from the source image and documents. Never guess silently which of two conflicting reference images is authoritative.

### 2.3 Required design-to-route matrix

For every selected image, record:

- exact image filename and dimensions;
- authoritative route and component composition;
- reference viewport and device scale factor;
- expected data fixture and content;
- required interactive states;
- associated visual test;
- baseline mismatch list;
- current status: not inspected, mapped, implementation underway, diff reviewed, accepted.

No screen should enter implementation until its reference and intended state are unambiguous.

---

## 3. What Is Aligned with the Architecture

### 3.1 Rust engine responsibilities are correctly separated in the architecture

The engine source is organized around CoIE, TIE, ChIE, graph storage, source snapshots, operation lifecycle, memory, and transport. The architecture defines Rust as the owner of deterministic code/test/change intelligence. That is the correct boundary for this product.

**Decision:** preserve engine implementations unless a reproducible defect or contract mismatch is demonstrated. UI work must not silently change canonical reasoning behavior.

### 3.2 The gRPC/Protobuf service boundary exists

The repository contains proto/v1 contracts and generated TypeScript service/message bindings, including OperationService and progress messages. The browser does not need a separate direct connection to Rust.

**Decision:** keep the typed boundary. Change the source .proto and regenerate bindings only when a verified contract defect or an intentionally approved contract extension requires it. Do not manually edit generated bindings.

### 3.3 The Agent Runtime already has meaningful modules

The source tree contains dedicated agent, auth, config, conversations, handoffs, memory, operations, repositories, results, Rust client, tools, and workflow modules. This is substantially aligned with the architecture in docs/architecture.md.

**Decision:** refine boundaries where current code concentrates too many responsibilities. Do not create a competing second runtime or duplicate service layer.

### 3.4 Progress and operations have an implementation path

The tree contains Rust progress streaming, operation state/events/services, SSE, and UI-side execution components. The transport path is established in code, even though presentation and consistency require work.

**Decision:** audit and reuse the existing events first. Do not invent display-only progress percentages or duplicate event state to make a screen appear more active.

### 3.5 Product components already exist

The current source includes summary, impact, risk, affected-files, test-impact, recommendations, graph, graph-path, evidence, memory, handoff-status, repository-card, repository-list, repository-setup-form, timeline, accordion, and status components.

**Decision:** every existing component must be audited. Reuse if it matches the required responsibility/design; refactor if the responsibility is sound but the structure/state/style is not; recompose if a correct component is not wired into its target screen; replace only if its abstraction conflicts with the product.

---

## 4. Major Problems to Correct

### 4.1 The visual system and token rules are inconsistent

Verified examples on the inspected revision:

- docs/design-system.md specifies a default button height of 40px, horizontal padding of 16px, and radius-md of 8px. The shared button uses rounded-2xl and its default size configuration is not the same as the documented 40px standard.
- docs/design-system.md specifies a default card with surface background, a 1px border, radius-lg of 12px, and 16px padding. The shared Card implementation uses a radius capped at 24px, shadow-sm, and ring styling.
- docs/design-system.md specifies an input with surface background, border color, and radius-md. The shared Input implementation uses h-8, rounded-2xl, a transparent border, and an input/50 background utility.

This is a design-system compliance issue, not a preference. Inspect every UI primitive against the documented tokens and actual design images, then update shared defaults and variants. Confirm the correct border/focus/shadow treatment from the image rather than automatically removing every style.

### 4.2 Homepage does not fully reproduce the specified screen

The current homepage is in apps/web/src/app/page.tsx. It fetches repositories, supports a question/composer flow, has four suggestion prompts (Analyze a file, Understand a feature, Trace an error, Plan a change), and includes a capabilities area. That is functional, but its entire composition and repository actions must be compared against the actual homepage image.

The desired screen includes the designed application shell, conversation/history navigation, repository selection, centered hero, composer controls, suggestions, repository setup choices, and the exploration/capabilities area as specified by the reference.

**Fix:** reproduce the full image rather than polishing the current reduced layout. Every shown action must connect to an actual supported flow.

### 4.3 Homepage and conversation entry flow must be unified

The home page has its own submission/redirect behavior. The conversation page in apps/web/src/app/conversations/[conversationId]/page.tsx loads a conversation and repository and composes ChatPanel. The application must have one coherent investigation journey, not two flows that diverge in state or presentation.

**Fix:** define the canonical path from selecting a repository to creating/resuming a conversation, starting an investigation, viewing progress, and navigating to results. Preserve conversation/operation identity and source snapshot/graph context across transitions.

### 4.4 ChatPanel mixes multiple product responsibilities

apps/web/src/components/chat/chat-panel.tsx manages chat messages, events, errors, running state, analysis navigation, canonical-result presentation, ExecutionTimeline, EngineAccordion, impact relationships, risk/tests, and evidence display.

This file is doing substantially more than pure chat presentation, and the current visible composition is not the full designed execution/result workspace.

**Fix:** split presentation responsibilities into coherent child components only after mapping state ownership. Keep a single authoritative operation state and pass UI view models to each panel. Do not create multiple competing sources of truth while splitting the component.

### 4.5 The timeline and engine accordion are not the designed execution/pipeline screens

apps/web/src/components/execution/execution-timeline.tsx currently sorts events by sequence and renders a scrollable card with event text, sequence, timestamp, and status. apps/web/src/components/execution/engine-accordion.tsx groups events by names containing coie, tie, or chie and renders native details/summary accordions.

These are useful foundational components but do not, by themselves, reproduce the full execution and pipeline designs. The current accordion groups only the Rust engines and should not be mistaken for the whole LLM → CoIE → TIE → ChIE → LLM reasoning path.

**Fix:** keep ordering and engine-event grouping where valid, and implement the exact hierarchy, state indicators, stage details, timings, and navigation in the intended layouts using actual events. If a stage is not exposed independently by the runtime, represent the real known state honestly rather than simulating a false event.

### 4.6 Analysis page is not composed as the designed results workspace

apps/web/src/app/analysis/[analysisId]/page.tsx defines local result/context/evidence types and renders analysis details as a vertical series of section cards. It includes result details, risks, impacts, tests, evidence, limitations, memory inspection, and pipeline events. The prior audit found that the page does not compose the full designed tabbed result workspace, despite dedicated product components existing.

**Fix:** build the designed workspace composition and wire the existing summary, impact, risk, affected-file, test-impact, graph, evidence, recommendation, memory, and handoff components into the correct areas. Avoid reimplementing each component's logic inside the route. Keep absent information visibly absent/unknown rather than rendering invented metrics or treating recommended tests as executed tests.

### 4.7 Runtime route responsibilities need refinement, not replacement

apps/web/src/app/api/agent/route.ts imports auth, target planning, workflow, operation service, repository service, conversation service, Rust client, Rust progress stream, and progress-state conversion. It parses/validates the input and handles investigation coordination/errors. This is a broad route handler compared with docs/architecture.md, which requires thin route handlers.

**Fix:** keep request parsing, auth, authorization, validation, application invocation, and response/stream setup at the route. Extract lifecycle/workflow orchestration into existing server modules only where a clearer tested responsibility results. Preserve the current request/response contract unless a concrete bug requires a change.

### 4.8 UI projection is weak as a typed boundary

apps/web/src/lib/server/results/ui-projection.ts checks that the incoming object includes outcome and data, preserves several semantic fields, and returns data as unknown. The analysis route and ChatPanel also define their own local result shapes.

**Fix:** introduce a deliberate UI DTO with the fields/screens actually require; validate shape at the boundary; centralize status, evidence, context, completeness, freshness, limitation, and diagnostic formatting. Avoid leaking internal protobuf details or having every component reinterpret untrusted data. Do not move canonical engine calculations into the projection.

### 4.9 Progress streaming has duplicated handling that needs a consistency review

The source contains apps/web/src/lib/server/operations/sse.ts and apps/web/src/app/api/operations/[operationId]/events/route.ts. The latter directly imports streamRustProgress and also has local progress-stream framing, while the shared SSE module provides operationStream.

**Fix:** trace actual callers, ownership, and behavior before deciding whether to consolidate. Verify that Rust progress, locally generated agent events, terminal completion/failure, reconnects, and after-sequence behavior are handled consistently. Do not blindly merge the modules without tests.

### 4.10 Repository, settings, Help, and state components are less complete than the designs

Repository list/detail/setup routes already exist and carry repository metadata/status. Settings has a section model and SettingsNavigation; Help has topic sections. Shared EmptyState, ErrorState, LoadingState, and CompletenessNotice components also exist.

**Fix:** compare each actual rendered route/state with its reference. Reuse working API/state logic. Complete the composition and style; only expand behavior if the reference and existing application capabilities call for it.

---

# Part I — Logic and UI Files Requiring an Audit/Update

The files below are named from the inspected source tree. A file is not automatically slated for a full rewrite. Each should be read, compared, and changed only to address the described mismatch.

## 5. Global Layout and Design System

| File | Current logic | Why it needs an update / planned work | Paired tests |
|---|---|---|---|
| apps/web/src/app/layout.tsx | Loads Inter with next/font, page metadata, global CSS, and AppShell; conditionally enables local development-session bootstrap | Confirm the shell/font loading and metadata fit the actual approved design. Preserve the development-session behavior; change only global layout issues found in screenshot comparison | App-shell smoke and cross-route screenshot tests |
| apps/web/src/app/globals.css | Global Tailwind/theme tokens and application styles | Reconcile CSS variables, dark surfaces, palette, typography, radii, spacing, and global defaults with docs/design-system.md and the supplied design images | Primitive snapshots plus affected page screenshots |
| apps/web/src/components/ui/button.tsx | Base-UI Button plus CVA variants and size classes | Align default/variant heights, padding, radius, icons, loading, disabled, hover, focus, pressed states with design tokens | Primitive variant tests and screenshots |
| apps/web/src/components/ui/card.tsx | Shared Card, header/title/description/content/footer/action styles | Align border/background/radius/padding/shadow, avoid accidental rings/shadows or excessive rounding, and preserve needed variants | Primitive snapshot and page regression tests |
| apps/web/src/components/ui/input.tsx | Base-UI Input with sizing and Tailwind styling | Align height, radius, padding, border, placeholder, focus/error/success, icon placement, and disabled state | Form/component tests and composer/form snapshots |
| apps/web/src/components/ui/tabs.tsx | Base-UI Tabs wrappers and CVA tab-list variants | Ensure horizontal/vertical layouts, active indicator, padding, text styles, keyboard behavior, and panel switching match designs | Tab interaction tests and result/settings screenshots |
| apps/web/src/components/ui/badge.tsx | Shared status/label badge styling | Ensure the visual treatment and semantic meaning of status, risk, completeness, and freshness match the references | Status variant tests and page snapshots |
| apps/web/src/components/ui/chip.tsx | Shared compact label/chip styling | Check repository language/tags, filters, and compact metadata use the correct density and active treatment | Relevant component and repository snapshots |
| apps/web/src/components/ui/data-table.tsx | Shared table presentation | Use for structured lists when comparison/scanning is better than cards, as design-system requires; verify density, overflow, sorting/actions if supported | Row data, empty state, overflow tests |
| apps/web/src/components/ui/result-list.tsx | Shared structured result-list presentation | Align evidence/file/test/result rows and semantic states with design-system rules | Rendering and visual tests |
| apps/web/src/components/ui/icon-button.tsx | Icon-only action control | Align dimensions, hit target, tooltip, label, disabled/focus/hover states; ensure an accessible name | Accessibility/interaction and snapshot tests |
| apps/web/src/components/ui/metric-card.tsx | Metric display primitive | Match dashboard metrics if the screen designs actually use this component; avoid inventing unsupported metrics | State rendering and repository/dashboard screenshots |
| apps/web/src/components/ui/status-indicator.tsx | Status presentation | Preserve textual state labels and avoid meaning encoded by color alone | State mapping tests and visual states |
| apps/web/src/components/ui/timeline.tsx | General timeline primitive | Compare spacing, connector, node/state styling, and layout with the execution reference before reusing | Timeline state snapshots |
| apps/web/src/components/ui/accordion.tsx | Shared accordion control | Align expand/collapse visuals and keyboard interactions; avoid inconsistent styles between shared accordion and native details/summary use | Interaction and screenshot tests |
| apps/web/src/components/ui/alert.tsx | Shared alert primitive | Align semantic severity styling, icon, spacing and action slots with error-state references | Variant tests and error-state screenshots |
| apps/web/src/components/ui/textarea.tsx | Shared multiline input | Align dimensions, radius, focus, disabled state, resize behavior, and composer use | Composer/keyboard and component visual tests |
| apps/web/src/components/ui/select.tsx | Shared select primitive | Align selected/empty/disabled/open/menu states with the designs and preserve accessible behavior | Selection and menu tests |
| apps/web/src/components/ui/modal.tsx and sheet.tsx | Overlay/panel primitives | Confirm size, backdrop, positioning, keyboard dismissal/focus behavior, and responsive layout match the relevant reference | Dialog/sheet functional and screenshot tests |
| Remaining apps/web/src/components/ui/* | Other primitives such as alert, separator, skeleton, tooltip, message, code block, sidebar, and badge-related UI | Audit all primitives against design-system conventions. Only update mismatched elements; do not assume every primitive needs a rewrite | Focused tests for changed primitives plus affected screen snapshots |

**Component rule:** use the existing Base UI/shadcn-based components and the current design-system conventions for standard elements. Do not hand-build a new button, tabs, select, modal, or other standard primitive when an appropriate existing component can be correctly configured and reused.

## 6. Shared Shell, Homepage, and Conversation

| File | Current logic | Why it needs an update / planned work | Paired tests |
|---|---|---|---|
| apps/web/src/components/shell/app-shell.tsx | Renders desktop sidebar/mobile navigation and supports a development session bootstrap state | Match shell dimensions/position and mobile behavior to designs; preserve dev-session bootstrap/error meaning | Shell navigation and all-route screenshot tests |
| apps/web/src/components/shell/sidebar.tsx | Loads repositories and recent conversations; supports search/actions/navigation; defines repository/settings/help destinations | Match complete designed sidebar, grouping, selected state, spacing, scroll behavior, search affordance, and conversation actions; verify all controls work | Sidebar search/delete/navigation tests and screenshot regression |
| apps/web/src/app/page.tsx | Loads repository data, provides initial question suggestions and composer submission, renders a capability/exploration area | Reproduce the full homepage design; unify the start-investigation path; wire repository selector/actions and all designed sections exactly | Homepage interactions, request flow, screenshot baseline |
| apps/web/src/app/conversations/[conversationId]/page.tsx | Loads conversation and repository, extracts snapshot/graph context from query parameters, and composes ChatPanel | Verify refresh/resume behavior, invalid/missing context handling, and transitions to execution/results; match conversation shell design | Conversation load, unavailable context, navigation, resume/refresh tests |
| apps/web/src/components/chat/chat-panel.tsx | Owns messages/events/error/running state, listens to operations, invokes submission, renders timeline/engine/detail results including risk/tests/evidence | Split responsibilities where necessary; one authoritative state source; make chat presentation match its design while linking to full execution/pipeline/results views | Chat submit/progress/error/navigation tests and screenshots |
| apps/web/src/components/chat/chat-composer.tsx | Composes question input and submit controls | Match input/toolbar/placeholder/attachments or selector controls that the reference actually contains; implement all displayed controls, disabled/submitting behavior, keyboard submit, and error handling | Input, submit, keyboard, disabled/loading visual tests |
| apps/web/src/components/chat/message-list.tsx | Renders conversation messages | Match spacing, alignment, message metadata, scrolling, and empty state; preserve stable message identity | Message state/scroll tests and screenshots |
| apps/web/src/components/chat/message-renderer.tsx | Renders individual message content | Preserve content semantics and safe formatting while matching text hierarchy, code rendering, and supporting metadata | Rendering tests for text/code/long responses |
| apps/web/src/components/chat/conversation-list.tsx | Renders conversation-history list | Match grouped/time-ordered history if shown by the design; ensure active selection and unavailable/empty states are clear | History grouping and active selection tests |
| apps/web/src/components/chat/conversation-search-modal.tsx | Provides conversation search UI | Compare with CodeMind Conversation Search Interface.png; match modal geometry, search state, result grouping, keyboard behavior, and no-result/error states | Modal search functional and visual tests |

## 7. Execution, Pipeline, and Result Workspace

| File | Current logic | Why it needs an update / planned work | Paired tests |
|---|---|---|---|
| apps/web/src/components/execution/execution-timeline.tsx | Sorts operation events by sequence and renders event message, sequence, timestamp, and status in a scrollable card | Recompose to the execution design, preserving event order but adding exact hierarchy and supported stage details; present honest empty/error states | Event ordering/mapping tests and in-progress/completed/failed screenshots |
| apps/web/src/components/execution/engine-accordion.tsx | Groups events by CoIE/TIE/ChIE and renders expand/collapse sections | Make detailed engine cards match the pipeline design. Do not treat the engine-only grouping as the complete LLM-to-engine-to-LLM pipeline | Engine stage grouping, expansion, no-events and failure-state tests |
| apps/web/src/components/execution/operation-status.tsx | Maps an operation status to a display indicator | Centralize correct text/icon/state mapping; verify pending/running/partial/completed/failed/unavailable states against design-system semantics | Status mapping tests and variant snapshots |
| apps/web/src/app/analysis/[analysisId]/page.tsx | Fetches analysis/operation data and directly composes a vertical sequence of result and execution sections; defines local data/result shapes | Replace generic vertical composition with designed results workspace; wire tab navigation and product components; make partial/missing data explicit; remove duplicate view logic where a typed UI model can own it | Analysis route, tab, projection, state, and screenshot tests |
| apps/web/src/components/analysis/summary-panel.tsx | Renders summary content | Align exact summary heading, text hierarchy, facts, and absent-data state; consume the agreed UI result view model | Full/partial/empty summary tests and snapshot |
| apps/web/src/components/analysis/impact-card.tsx | Displays impact details | Match the design's impact content and labels; do not conflate impact with risk or confidence | Impact variants, missing data, and screenshot tests |
| apps/web/src/components/analysis/risk-card.tsx | Displays risk assessment | Match severity presentation and rationale; preserve unknown factors and uncertainty without invented scores | Risk-state and rationale tests |
| apps/web/src/components/analysis/affected-files-panel.tsx | Displays affected-file information | Match list/table hierarchy, paths, scope/status and navigation; long paths must not break layout | Long path, empty list, navigation, overflow tests |
| apps/web/src/components/analysis/test-impact-panel.tsx | Displays test candidates/impact | Match groups and states; distinguish a recommended test from a test that was actually executed | Candidate/empty/partial-state tests |
| apps/web/src/components/analysis/recommendations-panel.tsx | Renders recommendations | Align information hierarchy and ordering; ensure controls are functional rather than decorative | Populated/empty/long-text and action tests |
| apps/web/src/components/graph/dependency-graph.tsx | Renders graph relationships | Match the exact graph design and supported controls; normalize data outside the render logic; ensure the graph remains usable with dense relationships and long labels | Deterministic graph fixture and interaction/screenshot tests |
| apps/web/src/components/graph/graph-path-panel.tsx | Shows paths/relationships | Preserve real evidence-path ordering and provenance; match path presentation and selection/navigation | Valid/empty/long path tests |
| apps/web/src/components/evidence/evidence-panel.tsx | Displays source evidence | Match source path, line, snapshot, derivation, and evidence grouping; retain provenance without implying unsupported certainty | Evidence completeness and source-navigation tests |
| apps/web/src/components/memory/memory-inspector.tsx | Shows retrieved/validated memory context | Keep memory subordinate to main investigation hierarchy while exposing the actual memory/validation state accurately | No-memory/retrieved/invalidated state tests |
| apps/web/src/components/memory/handoff-status.tsx | Shows handoff status | Display server-owned lifecycle truth, with correct next actions and error/recovery states | Handoff status tests and screenshots |

## 8. Repository, Settings, Help, and Shared States

| File | Current logic | Why it needs an update / planned work | Paired tests |
|---|---|---|---|
| apps/web/src/app/repositories/page.tsx | Fetches and parses repository list/status/metadata and renders basic cards, search, and actions | Match Repository Dashboard image including metrics/status/actions and add-repository area; only display supported metadata | Empty/populated/search/status/action tests and page screenshot |
| apps/web/src/app/repositories/[repositoryId]/page.tsx | Loads one repository and its readiness/snapshot/graph metadata | Match the designed repository detail behavior, ensure Open/Re-analyze/Progress paths retain identity/context | Loading/not-found/unavailable/detail tests |
| apps/web/src/app/repositories/new/page.tsx | Implements source selection and submission result states for GitHub/local/example repository types | Match staged repository setup image. Test source selection/configuration/indexing/completion/failure rather than a single happy-path form | Stage-by-stage functional tests and reference screenshots |
| apps/web/src/components/repository/repository-card.tsx | Renders repository identity, readiness and metadata | Align exact card geometry, labels, language/source details, menus/actions, and status visuals | Card variant tests and snapshot |
| apps/web/src/components/repository/repository-list.tsx | Composes the repository collection | Avoid duplicating page/list behavior; match filtering, empty states, ordering, and responsive arrangement | List/search/empty-state tests |
| apps/web/src/components/repository/repository-setup-form.tsx | Manages source type, source/name inputs, submitting state, response and error feedback using POST /api/repositories | Refactor only if necessary to support the designed staged state model; ensure mutually exclusive valid states and retry behavior | Validation, accepted/indexing/completed/partial/failed tests |
| apps/web/src/app/settings/page.tsx | Defines setting sections and presents current supported configuration/status data with tabs/navigation | Match Settings Dashboard and AI Settings Dashboard after resolving how they relate; do not display a control as functional unless its behavior exists | Section navigation, supported control behavior, settings screenshots |
| apps/web/src/components/settings/settings-navigation.tsx | Builds vertical/horizontal settings navigation from section descriptors | Match selected state, labels/descriptions/icons and responsive arrangement exactly | Selection/keyboard and screenshot tests |
| apps/web/src/app/help/page.tsx | Defines Help topics, summaries, keywords, and tab content | Match Help & Docs design while preserving actual help content and valid links | Help navigation/link and visual regression tests |
| apps/web/src/components/common/empty-state.tsx | Shared icon/title/description/optional-action state | Check spacing, border/surface, typography, and actions against the error/state designs; avoid one generic message for different causes | Route-specific empty state snapshots |
| apps/web/src/components/common/error-state.tsx | Shared error panel with retry/action support | Include actionable recovery, accurate failed/partial state, and accessible alert semantics matching the error-state reference | Retry behavior, error variants, screenshots |
| apps/web/src/components/common/loading-state.tsx | Spinner/detail/optional staged placeholder | Use skeletons for content loading and real staged progress for meaningful operations; avoid misleading static staged progress where no stage is known | Loading/progress state tests and screenshots |
| apps/web/src/components/common/completeness-notice.tsx | Exposes completeness/limitations notices | Keep incomplete/truncated/unknown analysis explicit and visually aligned; do not allow partial evidence to appear complete | Completeness state tests and result screenshots |

## 9. Server Runtime, Operation Flow, and Result Projection

These are logic changes, not visual-only changes. Refactor only when the code-level review or tests demonstrate that the current responsibilities or behavior violate the architecture or prevent an intended flow.

| File | Current logic | Why it needs an update / planned work | Paired tests |
|---|---|---|---|
| apps/web/src/app/api/agent/route.ts | Validates the request, loads session/auth context, resolves the request, coordinates conversation/operation, invokes runtime/Rust/progress paths, and handles terminal errors/results | Thin the route by extracting well-defined workflow/lifecycle operations while preserving endpoint behavior. Avoid moving engine reasoning or memory policy into the handler | Route validation/auth tests plus workflow/operation integration |
| apps/web/src/app/api/operations/[operationId]/events/route.ts | Authenticates and authorizes an operation/context, emits SSE, and handles Rust/local progress paths | Compare its framing/state lifecycle against operations/sse.ts. Reconcile only after proving the intended single source of stream behavior | SSE sequence, reconnect, terminal-state, auth, cancel tests |
| apps/web/src/lib/server/agent/runtime.ts | Owns the LLM/tool runtime entry point | Preserve core orchestration; ensure it returns validated canonical data and does not duplicate UI presentation rules | runtime.test.ts and integration fixtures |
| apps/web/src/lib/server/agent/tool-loop.ts | Owns tool-call loop details | Test bounded iteration, tool errors, invalid responses, and termination; modify only if the actual runtime behavior needs it | runtime.test.ts, guardrail and workflow tests |
| apps/web/src/lib/server/agent/target-resolver.ts | Plans/resolves the investigation target from a question | Keep planning distinct from deterministic canonical engine findings; verify ambiguity and selected target behavior | Runtime/target resolution tests |
| apps/web/src/lib/server/operations/state.ts | Defines operation state/status shape | Ensure statuses and terminal semantics cover what the UI and streams actually need, without UI-specific style values | Operation lifecycle tests |
| apps/web/src/lib/server/operations/events.ts | Owns event/state publishing helpers | Verify ordering, uniqueness, and sequence behavior across local and Rust events | Operation recovery/progress tests |
| apps/web/src/lib/server/operations/service.ts | Coordinates operation persistence, access and lifecycle | Verify owner binding, status transitions, result persistence, duplicate starts, expiry/recovery behavior | operations/recovery.test.ts and API integration |
| apps/web/src/lib/server/operations/sse.ts | Creates SSE stream from Rust progress, frames sequence IDs, and handles abort/cancel | Reconcile with the API events route; support stable replay and terminal behavior | Stream tests and browser integration |
| apps/web/src/lib/server/operations/rust-progress.ts | Maps a Rust progress event to an operation state/status | Verify that all supported stages map correctly. Current mapping detects TIE/ChIE/CoIE by string inclusion and defaults to planning; unknown-stage behavior must be explicit and tested | operations/rust-progress.test.ts |
| apps/web/src/lib/server/rust-client/client.ts | Calls typed Rust gRPC services | Preserve as the only application-side Rust boundary; validate request context, operation identity, contract errors and timeout/failure behavior | Mocked client and transport integration tests |
| apps/web/src/lib/server/rust-client/progress-stream.ts | Adapts Rust progress stream | Ensure cursor/sequence/reconnect/abort/terminal handling and transport-to-app mapping are correct | progress stream/recovery tests |
| apps/web/src/lib/server/results/ui-projection.ts | Checks canonical result minimum fields and returns a UI result shape; data remains unknown | Upgrade to an explicit, validated UI DTO consumed consistently by chat and analysis components; do not duplicate canonical reasoning in UI | Add ui-projection tests for valid/full/partial/malformed results |
| apps/web/src/lib/server/results/validate.ts | Validates canonical results | Keep canonical validation distinct from UI-specific projection and LLM prose; align with supported result contract | Validation and LLM projection tests |
| apps/web/src/lib/server/results/llm-projection.ts | Builds a result projection for explanation | Preserve source semantics and provenance; ensure display projection and LLM projection remain separate | Result validation/LLM projection tests |
| apps/web/src/lib/server/conversations/service.ts | Loads, creates, and saves conversations | Preserve history and stable IDs; verify creation/update/refresh behavior for the canonical route flow | conversations/service.test.ts and web journey tests |
| apps/web/src/lib/server/repositories/service.ts | Resolves repository state/source/metadata | Preserve authorization and snapshot/version semantics; verify UI route actions use the right repository context | API repository tests and setup/dashboard E2E |
| apps/web/src/lib/server/workflows/investigate-change.ts | Owns a focused investigation workflow | Keep actual orchestration here rather than in the API route; test errors and partial/canonical result lifecycle | workflows/investigate-change.test.ts |
| apps/web/src/lib/server/workflows/prepare-repository.ts | Prepares repositories before investigation | Keep setup/indexing state accurate and distinct from browser form presentation | Repository setup workflow tests |
| apps/web/src/lib/server/workflows/finalize-review.ts | Finalizes review result/context | Verify final result, operation lifecycle, and durable result ownership | Workflow and operation integration tests |
| apps/web/src/lib/server/config/env.ts | Reads and validates environment/configuration | Audit only if UI/runtime testing reveals unclear/missing configuration behavior; do not add unsupported features to settings | Environment/config tests if modified |
| apps/web/src/lib/server/auth/authorize.ts and auth/context.ts | Own request authentication and repository/memory authorization context | Preserve security boundaries while route/workflow code is split | Auth/context, route, and repository-access tests |

### Runtime ownership rule

Do not refactor every listed runtime file solely to make the audit look comprehensive. The required change is to fix the proven responsibility/behavior issue and prove the previous behavior still works. Any no-change decision should be recorded as “reviewed and retained” when the relevant flow is verified.

---

## 10. Rust Engine and Protobuf: Preserve Unless a Contract Defect Is Reproduced

No Rust engine production file is identified as a required change merely to fix the designed web UI. The current architecture and file organization are directionally correct. Therefore the default engine action is **review and preserve**.

| Area | Source files to preserve by default | Required action |
|---|---|---|
| Engine entry/lifecycle | engine/src/main.rs, engine/src/lib.rs, engine/src/engines/pipeline.rs, engine/src/lifecycle/* | Confirm startup/operation/progress semantics remain compatible. Do not restructure for visual work |
| CoIE | engine/src/engines/coie/* | Preserve deterministic code graph/extraction/query ownership. Change only if an integration contract bug is reproduced |
| TIE | engine/src/engines/tie/* | Preserve test discovery/mapping and typed graph behavior |
| ChIE | engine/src/engines/chie/* | Preserve change, impact, risk, and evidence semantics |
| Storage | engine/src/graph_storage/* | Preserve storage port/adapter and graph persistence; do not bypass it in the web layer |
| Source context | engine/src/source/* | Preserve source identity/snapshot boundaries; verify request context from the application if a defect appears |
| Memory | engine/src/memory/* | Preserve memory validation, retrieval, scope, and handoff semantics |
| Transport | engine/src/transport/* | Change only for a reproducible gRPC contract or progress defect |
| Protobuf sources | proto/v1/common.proto, intelligence.proto, memory.proto, operation.proto, repository.proto | Source of truth. Update and regenerate only when an approved transport change is necessary |
| Generated TS bindings | apps/web/src/lib/server/rust-client/generated/* | Generated artifacts; never hand-edit |

The most likely required work is web composition and tests, not Rust domain-engine changes. If the audit finds a concrete engine problem, first write down the reproduced input, expected contract, actual result, responsible layer, and regression test. Then fix the narrowest responsible engine module.

---

# Part II — Test Coverage Audit

## 11. Current Test Inventory and Strength

The repository does have substantial Rust test coverage and a smaller set of server/runtime tests. The weak point for this repair is browser-level and visual verification.

### 11.1 Relatively strong: Rust logic and integration layers

Current integration test files under engine/tests include:

- bounded_queries.rs
- canonical_graph.rs
- chie_impact.rs
- day1_boundaries.rs
- falkordb_integration.rs
- grpc_contract.rs
- handoff_lifecycle.rs
- language_adapters.rs
- level3_engines.rs
- level3_memory.rs
- level3_operation_grpc.rs
- level3_storage.rs
- level3_transport.rs
- memory_persistence.rs
- operation_recovery.rs
- support.rs
- tie_discovery.rs

These areas cover core graph contracts/queries, engine intelligence, language adapters, storage, gRPC/transport, memory/handoffs, and operation recovery. Inline unit tests also exist across engine source. This is the strongest established testing layer for canonical engine logic.

**Important:** a list of tests establishes test presence, not a claim that every test passes in the current environment. Run the applicable Rust tests before and after any related contract change.

### 11.2 Moderate: Agent Runtime and server logic

Existing Vitest files include:

- apps/web/src/lib/server/agent/runtime.test.ts
- apps/web/src/lib/server/conversations/service.test.ts
- apps/web/src/lib/server/operations/recovery.test.ts
- apps/web/src/lib/server/operations/rust-progress.test.ts
- apps/web/src/lib/server/results/llm-encoding.test.ts
- apps/web/src/lib/server/tools/guardrails.test.ts
- apps/web/src/lib/server/workflows/investigate-change.test.ts

These test selected runtime, conversation, operation, result encoding, guardrail, and workflow behavior. They should be extended where the repair affects their contracts. There is no reason to replace them with broad, redundant tests.

### 11.3 Existing cross-contract tests, but not browser E2E

The following files are under src/tests:

- apps/web/src/tests/e2e/track01.spec.ts
- apps/web/src/tests/graph-conformance/canonical-model.spec.ts
- apps/web/src/tests/graph-conformance/relationship-semantics.spec.ts
- apps/web/src/tests/graph-conformance/snapshot-isolation.spec.ts

The inspected track01.spec.ts imports Vitest, uses vi.mock, and imports runAgent, projectForUi, validateCanonicalResult, and authorization logic. It is a server/runtime integration-style Vitest test, not a Playwright browser E2E test despite its directory/name.

The graph-conformance files are contract/conformance specifications. Keep them in the appropriate test runner; do not count these as browser screenshot coverage.

### 11.4 Weak/missing: Playwright browser coverage

Verified on the inspected web-audit tree:

- apps/web/playwright.config.ts sets testDir to ./playwright.
- No files were present under apps/web/playwright/ in the inspected repository tree.
- The package script apps/web/package.json defines test:e2e as playwright test.
- The config excludes tests tagged @engine by default unless CODEMIND_E2E_WITH_ENGINE=1, but there are no tests discovered under the configured directory on the inspected tree.

This means browser E2E discovery is likely empty on that branch. Verify with the actual command before implementation, then correct the test directory/layout and configuration. Do not claim the browser E2E suite passes if no tests are discovered.

### 11.5 Missing/not established: reference screenshot regression

No dedicated set of screenshot baselines for all supplied design images was established in the inspected tree. The Playwright config captures screenshots/videos on failures, but failure artifacts are not the same as expected design snapshots.

Required: add actual visual regression baselines for every approved design screen and the required interaction states. Use deterministic fixtures and a controlled browser environment.

---

## 12. Test Files to Update or Add

Tests must be changed alongside the logic/UI file they exercise. Use existing test files when their responsibility matches, add focused tests where none exist, and do not put browser tests into Vitest merely because the existing directory is named e2e.

| Test target | Current role / required changes | Production area covered |
|---|---|---|
| apps/web/src/lib/server/agent/runtime.test.ts | Keep runtime/orchestration regressions; add operation/request failures and result flow only where affected | agent/runtime.ts, tool-loop.ts, target-resolver.ts |
| apps/web/src/lib/server/conversations/service.test.ts | Test creation, retrieval, update, stable IDs, failure conditions as required by unified conversation path | conversations/service.ts and conversation route |
| apps/web/src/lib/server/operations/recovery.test.ts | Test terminal states, reconnect/resume, missing/expired state and recovery contracts actually supported | operations/service.ts, state.ts, events.ts |
| apps/web/src/lib/server/operations/rust-progress.test.ts | Cover known engine stages, unknown stage, ordering/status/terminal mapping and duplicate sequence behavior | operations/rust-progress.ts, progress-stream.ts |
| apps/web/src/lib/server/results/llm-encoding.test.ts | Preserve encoding contract; add only if projection refactor changes the canonical-to-LLM boundary | results/llm-encoding.ts and related projections |
| apps/web/src/lib/server/tools/guardrails.test.ts | Keep authorization/validation/guardrail regressions | tools/guardrails.ts and tool system |
| apps/web/src/lib/server/workflows/investigate-change.test.ts | Test end-to-end workflow logic at runtime level, including relevant failures/partial results | workflows/investigate-change.ts, route coordination |
| apps/web/src/tests/e2e/track01.spec.ts | Keep as Vitest runtime/result contract test unless deliberately moved and converted. Rename/reorganize only to accurately identify runner and purpose | Agent Runtime, UI projection, canonical validation, authorization |
| apps/web/src/tests/graph-conformance/canonical-model.spec.ts | Preserve canonical contract tests | Canonical model and results |
| apps/web/src/tests/graph-conformance/relationship-semantics.spec.ts | Preserve relationship contract tests | Relationship semantics/result projection |
| apps/web/src/tests/graph-conformance/snapshot-isolation.spec.ts | Preserve repository/snapshot isolation tests | Repository context/authorization/storage contracts |
| apps/web/playwright.config.ts | Fix testDir or move/write browser specs to the configured directory; ensure default versus engine-backed suites are intentional | Browser test discovery and execution |
| NEW: homepage Playwright spec | Verify homepage content, interactions, repository selection, navigation and exact reference screenshot | app/page.tsx, shell, shared UI |
| NEW: conversation/execution Playwright spec | Exercise question submission, operation progress, navigation, error and completed states | ChatPanel, conversation route, timeline, operation stream |
| NEW: pipeline Playwright spec | Verify stage order, engine detail disclosure, lifecycle states and screenshots | execution/pipeline composition and progress model |
| NEW: analysis/results Playwright spec | Verify tab navigation, data rendering, partial/empty states and screenshots | analysis page and result components |
| NEW: repository dashboard/setup Playwright spec | Cover list/search/actions and each setup stage/state | repository page/cards/setup form |
| NEW: settings/help Playwright spec | Verify section navigation, controls, help routes and screenshots | settings, Help, shared shell |
| NEW: shared component visual tests | Cover button/card/input/tabs/status and other shared primitives in meaningful variants | components/ui/* and globals.css |
| NEW: UI projection contract test | Validate full, partial, malformed canonical result and the stable UI DTO contract | results/ui-projection.ts |
| NEW: progress-to-UI integration test | Verify real/fixture events map to the same state across ChatPanel/timeline/pipeline and preserve terminal events | rust-progress, SSE, event route, UI state |

For tests with no exact companion filename yet, use the repository's existing Vitest/Playwright conventions and create one narrowly named test. Do not duplicate an existing test with a different filename merely to satisfy this plan.

### 12.1 Browser configuration acceptance

After the test-directory fix:

1. Run the Playwright test-list command and verify it discovers the expected tests.
2. Run the ordinary browser suite.
3. Run the engine-backed suite with CODEMIND_E2E_WITH_ENGINE=1 when the required services are available.
4. Record discovered, passed, failed, skipped, and blocked counts.
5. Confirm the browser suite actually visits and exercises the real routes.
6. Confirm screenshot assertions compare against the intended reference states, not only element existence.

---

# Part III — Ordered Implementation Phases

## 13. Phase 0: Confirm Baseline and Map References

**Files:** no production changes.

- Compare the checked-out worktree with the uploaded ZIP and branch revision.
- Record uncommitted changes; do not overwrite or revert them.
- Inspect every design image in docs/design at full resolution.
- Resolve the two pipeline images and two settings-related images as explicit design references.
- Confirm the route for every design and the exact state the image depicts.
- Inventory actual test discovery and baseline failures.
- Capture current screenshots at agreed reference viewports.
- Record each mismatch and its likely owning component.

**Exit condition:** exact file tree, route/design mapping, test inventory, and initial screenshot diffs are recorded. No production implementation has changed.

## 14. Phase 1: Repair Shared Tokens, Primitives, and Shell

**Targets:** globals.css, layout.tsx, components/ui/*, shell/app-shell.tsx, shell/sidebar.tsx.

- Align shared tokens with design-system.md.
- Correct button, card, input, tabs and status component states.
- Align shell/sidebar and navigation dimensions and scroll behavior.
- Add focused tests for the primitives that changed.
- Compare every affected page, not just a component playground.

**Exit condition:** the foundation matches the documented system and reference screen; shared changes cause no new route-level visual regressions.

## 15. Phase 2: Homepage and Unified Entry Flow

**Targets:** app/page.tsx, chat-composer.tsx, conversation-list.tsx, conversation-search-modal.tsx, conversations/[conversationId]/page.tsx, sidebar.tsx.

- Implement the full approved homepage image.
- Make repository selection and setup actions functional.
- Unify home submission with the conversation/operation flow.
- Preserve the repository, source snapshot, graph version, conversation ID, and operation ID as needed throughout navigation.
- Cover empty, validation, loading and unavailable states.

**Exit condition:** homepage reference diff is accepted; the user can start an investigation and arrive at a coherent conversation/execution experience.

## 16. Phase 3: Conversation, Execution Timeline, and Pipeline

**Targets:** chat-panel.tsx, execution-timeline.tsx, engine-accordion.tsx, operation-status.tsx, actual pipeline composition, progress mapping and streaming paths.

- Separate chat, detailed execution, and detailed pipeline responsibilities.
- Render the complete supported pipeline without fabricating events.
- Keep all progress tied to operation/event data.
- Handle failure, partial completion, reconnection, and empty streams.
- Preserve operation identity and context when switching views.

**Exit condition:** a real or deterministic engine-backed investigation traverses the supported UI lifecycle and matches execution/pipeline references at the required states.

## 17. Phase 4: Results Workspace and Product Components

**Targets:** analysis/[analysisId]/page.tsx, results/ui-projection.ts, all analysis/*, graph/*, evidence/*, memory/* components.

- Define a stable UI-safe result DTO.
- Compose the intended result layout and all designed tabs.
- Align each subcomponent individually.
- Preserve provenance, completeness, freshness, limitations and uncertainty.
- Separate recommendations from executed results and risk from impact/confidence.
- Handle unavailable or partial evidence consistently.

**Exit condition:** each result area works, data remains truthful, all approved result references pass, and projection/component contract tests pass.

## 18. Phase 5: Repository Dashboard and Setup

**Targets:** repository routes/pages, repository card/list/setup form, repository service as needed.

- Reproduce dashboard metrics/cards/actions supported by the design.
- Complete the staged source/configuration/indexing/completion/error flow.
- Verify every visible action and status.
- Add screenshots for each materially different setup state.

**Exit condition:** dashboard and setup references match; no displayed repository action is a dead control.

## 19. Phase 6: Settings, Help, Search, and Error States

**Targets:** settings page/navigation, Help, search modal, common empty/error/loading/completeness components.

- Match each reference screen and state.
- Keep settings controls honest about actual supported behavior.
- Ensure search supports the designed interactions and no-results/failure behavior.
- Align loading/error/empty variants to docs/design/CodeMind Error States Dashboard.png and related references.

**Exit condition:** relevant reference images and interaction states pass visual/functional acceptance.

## 20. Phase 7: Runtime Boundary and Regression Fixes

**Targets:** agent API route, operations service/state/events/SSE, Rust client/progress stream, workflows, repositories/conversations services, result projection.

- Refactor only demonstrated responsibility duplication or incorrect state behavior.
- Add contract tests before changing boundaries.
- Ensure one consistent view of operation lifecycle and progress.
- Preserve gRPC/protobuf semantics.
- Touch Rust production code only if the responsible defect is reproduced inside Rust.

**Exit condition:** route and workflow boundaries are clearer without regressions in auth, canonical results, progress or recovery.

## 21. Phase 8: Full Verification

Run the commands already defined by the monorepo and apps/web package scripts, and use the project's documented Rust/engine setup. At minimum, verify:

- Web lint
- Web typecheck
- Web unit tests
- Playwright browser suite, with test discovery confirmed
- Visual regression suite for each approved reference screen/state
- Engine-backed browser suite when required services are available
- Rust unit/integration tests when the engine or transport contracts are touched
- End-to-end path from homepage through conversation, execution, pipeline and final results
- Repository setup and settings/help flows
- Clean/repeatable runs without stale state

Record exact commands and results. A skipped test is not a pass; a test runner that discovers zero browser tests is not evidence of E2E coverage.

---

# Part IV — Screenshot and Visual Acceptance Protocol

## 22. Controlled Screenshot Environment

Use one fixed environment for visual comparisons:

- fixed browser version;
- fixed viewport per design;
- fixed device scale factor;
- fonts loaded before capturing screenshots;
- deterministic repository/result fixture data;
- stable timestamps and labels;
- animations/transitions stabilized for screenshot capture while preserving separate interaction testing;
- consistent browser and operating system rendering.

If a design image is from a different viewport than the application's target, record the target viewport before making implementation assumptions.

## 23. Comparison Procedure

For every mapped reference:

1. Navigate to the real product route using the real page composition.
2. Create the required deterministic application state.
3. Capture a screenshot at the declared viewport.
4. Produce an overlay/difference comparison with the supplied reference.
5. Inspect differences in macro layout, spacing, typography, surfaces, controls, content, and state styling.
6. Fix the responsible shared token/component/page composition.
7. Retake the screenshot and rerun related page screenshots.
8. Mark the design matrix accepted only after the diffs have been reviewed and genuine mismatches resolved.

Do not create a fake screen whose only purpose is to resemble the screenshot. Visual tests must capture the actual production route/components.

## 24. Definition of Done Per Screen

A screen is complete only if:

- its reference image and state are authoritative and mapped;
- the rendered layout matches at the canonical viewport;
- controls and transitions work as specified;
- data is from correct application/runtime sources;
- loading, empty, error, partial, running, and completed states are correct where applicable;
- accessibility and keyboard behavior are preserved;
- relevant unit/integration/browser tests pass;
- screenshot differences are inspected and resolved;
- no global-token or component regression appears on another screen.

---

# Part V — Scope and Change Control

## 25. Files to Preserve Unless a Defect Is Demonstrated

Preserve by default:

- engine/src/engines/coie/*
- engine/src/engines/tie/*
- engine/src/engines/chie/*
- engine/src/engines/pipeline.rs
- engine/src/graph_storage/*
- engine/src/source/*
- engine/src/memory/*
- engine/src/lifecycle/*
- engine/src/transport/*
- proto/v1/*.proto and generated bindings
- existing canonical model and conformance tests
- semantic tool definitions/guardrails
- working repository snapshot and authorization semantics

These paths still need to be read when a related web contract changes. They are not default UI repair targets. Do not introduce a new engine defect while “cleaning up” a working domain implementation.

## 26. Component Quality Checklist

For every component updated, answer:

- Does it have one clear responsibility?
- Does it follow the authoritative design image and design-system tokens?
- Does it consume a validated view model rather than reinterpret raw backend data?
- Is the state owned in one place?
- Do all displayed buttons/links/tabs/menus have real behavior?
- Are loading, empty, partial, error, disabled and completed states correct?
- Is evidence/source provenance preserved where relevant?
- Are long names, paths, and dense data handled without breaking layout?
- Is keyboard and screen-reader behavior preserved?
- Is there a focused test for important states?
- Has the full page been screenshot-checked after the shared component changed?

Choose the smallest appropriate action:

- **Reuse:** already correct in responsibility, behavior and design.
- **Refactor:** valid responsibility, but tangled state, structure or styling.
- **Recompose:** component is useful but not used in the required page.
- **Replace:** abstraction conflicts with product/architecture requirements and cannot be corrected reasonably.

## 27. Final Acceptance Checklist

### Architecture and behavior

- [ ] Rust remains canonical for deterministic code/test/change intelligence.
- [ ] UI does not access FalkorDB or call Rust directly.
- [ ] Route handlers remain thin boundaries after targeted refinement.
- [ ] Canonical result, LLM projection, and UI projection remain separate.
- [ ] Real progress/terminal state reaches the UI without invented activity.
- [ ] Conversation, repository, snapshot, graph and operation identity remain consistent.
- [ ] Memory/handoff status is accurate.
- [ ] No production engine change is made without a reproduced engine/contract defect and regression test.

### Screens and components

- [ ] All relevant assets in docs/design have been visually inspected and mapped.
- [ ] Variant/duplicate pipeline and settings references have been resolved explicitly.
- [ ] Homepage is accepted against its reference.
- [ ] Conversation and composer are accepted against their designs.
- [ ] Execution timeline is accepted against its reference.
- [ ] Pipeline screen is accepted against its reference.
- [ ] Analysis/result workspace and each major tab are accepted.
- [ ] Repository dashboard/detail/setup are accepted.
- [ ] Settings, Help, conversation search and error states are accepted.
- [ ] Shared primitives comply with the documented design system.
- [ ] All visible controls have real, tested behavior.
- [ ] No genuine unexplained visual mismatch remains.

### Testing

- [ ] Test runner discovery has been verified.
- [ ] Unit, integration and browser tests have been run and their counts recorded.
- [ ] Screenshot baselines cover every approved screen and required state.
- [ ] Screenshot diffs are reviewed rather than blindly accepted.
- [ ] Engine-backed tests are not confused with ordinary web-only tests.
- [ ] Failed/blocked/skipped tests are accurately reported.
- [ ] Existing tests have not been deleted or weakened merely to obtain a green run.

## 28. Final Decision

Do not rebuild CodeMind from scratch. Preserve the Rust engine and typed boundary, retain the modular Agent Runtime, and reuse the product components that genuinely fit. Correct the design-system mismatches, compose the actual web product around the intended journey, make progress and results truthful and complete, and add genuine browser/visual regression coverage.

The implementation, architecture, rendered UI, and tests must agree. Functionality alone is insufficient. Design similarity alone is insufficient. The final product must be behaviorally correct and faithfully match every approved designed screen at its agreed reference viewport.
