# CodeMind Demo Plan

**Event:** Graph Hacks: Context for AI Agents, powered by FalkorDB  
**Implementation window:** 15–18 October 2026 (IST)  
**Document purpose:** The authoritative demo story, recording script, capture checklist, rehearsal procedure, and submission-video quality gate for the hackathon MVP.

> **This is a pre-event demo plan, not evidence that the implementation already exists.** Implement and record only behavior that actually works in the event build and passes the checks in `testing.md`. Do not replace missing behavior with mock output, hard-coded results, a pre-rendered animation, or a demo-only execution path.

---

## 1. Official requirements and compliance

The official [Graph Hacks rules](https://www.wemakedevs.org/hackathons/falkordb/rules) currently require every submission to include:

- A **public source-code repository**.
- A README with setup instructions and the graph data model.
- The Cypher queries or graph algorithms the product depends on.
- A **demo video showing the working project**.
- A live deployment, or complete local setup instructions if it cannot be deployed.
- The track or tracks entered.

The rules also require FalkorDB to be the **primary graph database** and for it to perform central product work, not merely power a decorative visualization. The rules permit public, synthetic, or the participant's own data, and prohibit private, login-protected, paywalled, personal, or otherwise restricted information from being loaded into the graph. AI coding assistant use must be disclosed, and the participant must understand and be able to explain the architecture, graph model, queries, and technical decisions.

**The official Graph Hacks rules page checked on 12 October 2026 does not state a fixed demo-video duration.** This plan targets approximately **2 minutes 55 seconds** as a concise, judge-friendly recommendation, not as an official limit. Recheck the official rules and submission form on the event and submission days in case the organizers publish additional requirements.

Source: [Graph Hacks rules](https://www.wemakedevs.org/hackathons/falkordb/rules), [Graph Hacks overview and tracks](https://www.wemakedevs.org/hackathons/falkordb).

### 1.1 Tracks to declare

The planned entry is for these two tracks only:

1. **Track 01 — Agents That Act on Connected Data.** CodeMind follows real code and test relationships in FalkorDB, derives bounded multi-hop impact, and shows the path and evidence behind the result.
2. **Track 02 — Agent Memory and Coordination.** CodeMind persists a review episode, validated memory, and a structured handoff, then retrieves them in a new session and checks prior context against current evidence.

Do not select Track 03 unless the team deliberately changes scope and builds a real Company Brain capability. The current architecture does not target it.

### 1.2 Rules the demo must visibly respect

- **Show FalkorDB doing real work.** Demonstrate a real graph-backed result and an evidence path, and include a compact proof that changing a relevant persisted relationship changes the canonical result.
- **Show the working product.** Capture the actual user interface and real operation events. Avoid a slide deck that merely describes what would happen.
- **Use permitted data.** Default to the bundled synthetic `codemind-polyglot-demo` fixture, which is the deterministic offline path. If another repository is used, confirm that its source and contents are public or otherwise permitted before ingesting it.
- **Keep the repository inspectable.** The recorded workflow must correspond to the implementation and source in the submitted public repository.
- **Disclose AI assistance.** Include the required disclosure in the repository README or submission materials. Do not imply that the implementation was produced without assistance if it was not.
- **Make local execution reproducible.** If a stable live deployment is not available, show enough of the real local workflow and provide complete setup instructions in the README.
- **Do not claim unseen features.** If a feature does not survive the Day 4 acceptance gate, remove it from the spoken script and visible claims rather than implying it works.

---

## 2. The story in one sentence

**CodeMind helps a developer answer “What could this change affect?” by tracing real code and test relationships in FalkorDB, then carrying validated investigation context into a fresh session.**

The demo is not a tour of every page. It is one compact investigation story with two proofs:

- **Track 01 proof:** the graph's actual relationships determine impact and the result can be traced to evidence.
- **Track 02 proof:** review context and a structured handoff persist beyond the original session and are checked against current engine evidence before reuse.

These proofs matter more than showing every screen, language, setting, or secondary feature.

## 3. Who the demo is for

Assume the viewer is a technically capable judge who understands software development but may not know the internals of CodeMind. Explain the user problem plainly first, then make the graph and persistence mechanisms visible without turning the video into a lecture.

By the end, the viewer should be able to answer:

1. What problem does CodeMind solve?
2. What does the user actually do with it?
3. Where does FalkorDB affect the result?
4. How can the viewer inspect the path and evidence behind an answer?
5. What survives a session boundary, and how is old context checked?
6. What technologies and tracks does the project target, and how can it be run?

---

## 4. Pre-recording gate: nothing is assumed complete

The exact symbol, result paths, test candidates, risk factors, and memory labels used in the final recording must be selected **after** the implementation and Day 4 tests establish what is truly available. Replace the placeholders in this document only with values observed in the real fixture and UI.

Before recording, confirm all of the following:

- [ ] A clean start works using the documented setup.
- [ ] FalkorDB is running and its persistent graph can be inspected.
- [ ] The bundled polyglot fixture loads deterministically without relying on public-network access.
- [ ] Rust, TypeScript/JavaScript, and Python investigations pass their relevant end-to-end tests.
- [ ] The chosen main investigation produces a genuine multi-hop path in FalkorDB.
- [ ] The chosen result distinguishes direct and indirect impact where supported by the fixture.
- [ ] Test candidates come from the real TIE graph/evidence. Do not imply runtime test execution; MVP test discovery is static.
- [ ] Risk is presented using supported qualitative factors/tier, not an invented numeric percentage.
- [ ] The visible impact path links to actual evidence and the correct repository/snapshot or graph version.
- [ ] The controlled graph-edge test passes: removing/disabling one relationship required for a path changes the canonical result as expected.
- [ ] A review episode/finding, qualified memory, and structured handoff persist.
- [ ] A fresh session retrieves the handoff from storage, and the receiving stage genuinely consumes it.
- [ ] A fresh session retrieves authorized memory from persistence and checks applicability/freshness against current evidence.
- [ ] The stale-memory handling test passes. Prepare a controlled scenario where previously validated memory is tied to an older snapshot or changed relationship, and current graph evidence wins. Record a visible stale-memory clip only if the submitted UI genuinely exposes that state; otherwise keep the verified scenario ready for judge follow-up and do not claim a visible UI feature that is absent. Keep it separate from the positive applicable-memory path and use an isolated fixture/graph variant.
- [ ] At least one failure/partial state has been tested so the UI does not silently convert uncertainty into success.
- [ ] No secret, API key, token, private path, personal data, private repository, or restricted source is visible in the capture.
- [ ] Every statement in the script is supported by the actual build.

Use `testing.md`, especially the Track 01 E2E, FalkorDB dependency proof, Track 02 handoff, fresh-session memory, offline fallback, and Day 4 demo verification sections, as the source of truth for readiness.

If Track 02 fails its gate, do not quietly simulate it. Fix the implementation or accurately narrow the video and submission claims; the planned entry into Track 02 is only justified by a real persistent-memory/handoff proof.

---

## 5. Demo environment and stage setup

### 5.1 Preferred environment

Use the bundled polyglot fixture at:

```text
fixtures/repositories/codemind-polyglot-demo/
├── rust/
├── typescript/
└── python/
```

This fixture is preferred because it is deterministic, permitted, locally available, and aligned with the three-language conformance plan. It avoids making the central demo depend on GitHub availability or network access. Do not fabricate a shared dependency chain across languages; each language subproject must demonstrate its own source-supported relationships.

For the main short story, pick **one language and one symbol with the clearest verified multi-hop path**. Prefer the Rust fixture for the hero scenario, as planned in the final discussion, provided its path is actually supported by the checked-in source and passing E2E tests. If TypeScript/JavaScript or Python has a clearer verified path, choose that instead and still verify the remaining adapters separately. Never invent a fixture symbol or dependency chain.

### 5.2 Processes

Only the three planned runtime processes should be required for the central local demo:

1. FalkorDB, normally launched with Docker Compose.
2. The Rust `codemind-engine` service.
3. The Next.js application, with the Agent Runtime inside it.

Do not introduce a second Express service or a separately deployed Agent Runtime just for the recording.

### 5.3 Prepare before capture

- [ ] Use a clean browser profile or a demo account with only safe fixture data.
- [ ] Close unrelated tabs, notification popups, terminals, and applications.
- [ ] Increase UI/browser zoom enough for small labels and evidence paths to be legible at video resolution.
- [ ] Confirm the microphone is clear and the desktop capture does not expose credentials.
- [ ] Confirm the repository graph is seeded and the target symbol is easy to select.
- [ ] Prepare a disposable isolated graph/fixture variant for the graph-dependency proof. Never mutate the main demo graph destructively during a single take.
- [ ] Prepare a fresh-session route that proves persistence without using browser-local state or hidden prompt replay. Prefer a genuinely new conversation/session; restart a process as well if the acceptance test requires it.
- [ ] Ensure current graph/snapshot details shown in the UI belong to the selected fixture.
- [ ] Keep a known-good seed/reset command ready, but do not display secrets or destructive reset options in the final capture.
- [ ] Verify actual terminal commands against the scripts in the final repository. Do not rely on commands that exist only in this plan.

### 5.4 Visible interface order

The capture should follow the natural user workflow:

1. Home/chat entry.
2. Select or open the bundled workspace.
3. Ask a natural-language change question.
4. Watch the real operation timeline.
5. Review Summary / Impact Analysis / Test Impact / Evidence.
6. Open a graph path and supporting evidence.
7. Show compact FalkorDB-backed proof.
8. Persist the investigation context and handoff.
9. Start a new session and retrieve/validate persisted context.
10. Show a brief Rust / TypeScript-JavaScript / Python coverage montage using actual results.
11. Finish on one compact architecture view and the declared tracks.

Do not spend the video's limited time touring Settings, Help, Repository Management, or cosmetic preferences. These pages can be explored from the application if the judges want to inspect more.

---

## 6. Primary recorded video: target approximately 3:00

**Recommended final video length:** approximately 2 minutes 55 seconds to 3 minutes. This is a production recommendation, not an official duration limit. The official rules and final submission form take precedence; check them again on the event and submission days.

The main video should be one connected story, not a tour of pages. Keep the result and evidence readable. Capture the actual app and actual runs; short edits are acceptable for the fresh-session transition and the isolated graph/memory proofs only when the clips come from the submitted build.

### 0:00–0:12 — Hook: define the problem

**On screen:** CodeMind's chat-first home and the bundled demo workspace.

**Narration:**

> “When you change a function, the hard part is finding what depends on it, which tests are relevant, and why those conclusions are trustworthy. CodeMind follows real code relationships instead of asking an LLM to guess from snippets.”

### 0:12–0:30 — Ask the investigation question

Open the prepared workspace and select the actual tested symbol. Submit:

> **“What could this change affect?”**

When useful, expand it to: “If I change `[ACTUAL_FIXTURE_SYMBOL]`, what could be affected, which tests should I review, and what evidence supports your answer?” Replace the placeholder before recording.

**Narration:**

> “I am using the bundled fixture so this investigation is repeatable offline. CodeMind resolves a real symbol and starts the same analysis pipeline used by the application.”

Show the selected repository, snapshot/context, and symbol so the query is visibly bound to actual source.

### 0:30–0:50 — Show the real engine pipeline

**On screen:** The real operation timeline advances through **CoIE → TIE → ChIE**, followed by the LLM reasoning/explanation step. If the actual UI exposes tool execution details, briefly reveal the relevant approved semantic tool and its real bounded result; do not create a fake tool-call overlay just for the recording. Progress must originate from typed operation events and the real engine-completion/publication signal, never a timer or hard-coded status.

**Narration:**

> “CoIE establishes canonical code relationships. TIE adds statically discovered test intelligence, without claiming the tests were executed. ChIE follows supported relationships to derive impact and evidence.”

Make clear that CoIE, TIE, and ChIE are modules in one Rust engine, not independently deployed services.

### 0:50–1:15 — Read the structured result

Show the implemented Summary / Impact Analysis / Test Impact / Evidence content backed by the actual result. If supported, visibly include one direct impact, one indirect or multi-hop impact, one genuine test candidate, one qualitative risk factor, and snapshot/graph provenance or a limitation.

**Narration:**

> “The result distinguishes direct impact from impact reached through additional relationships. Test candidates and risk factors are tied to the evidence available. The LLM explains the validated result; it does not create the canonical graph facts.”

Every visible count, name, path, risk factor, and status must come from the actual canonical result. Do not show invented scores, risk percentages, coverage numbers, confidence percentages, latency numbers, or unsupported claims.

### 1:15–1:30 — Make the evidence path inspectable

Expand one verified path from the selected symbol to an affected entity or test candidate. Show the actual relationship labels and directions and its source/provenance evidence. Leave it on screen long enough to read. A visually attractive but unreadable graph is not proof.

**Narration:**

> “This is the path behind the answer. You can inspect the relationship chain and evidence, including any unresolved link or limitation, rather than accepting an unexplained answer.”

### 1:30–1:45 — Prove FalkorDB materially changes the answer

Show a real persisted graph relationship/path, then a controlled rerun in an isolated graph or fixture variant after removing/disabling one edge required by that path. Show the canonical result/path changing or becoming honestly unresolved/incomplete. Never mutate the primary presentation graph destructively.

**Narration:**

> “The graph is not decoration. When a relationship required for this path changes in FalkorDB, the canonical result changes with it.”

Use the real integration-tested query/algorithm and source revision. This proof is mandatory for making the central Track 01 claim. If it failed, fix it or remove the claim; never substitute a mock or static diagram.

### 1:45–1:58 — Persist the investigation and handoff

Show the actual persisted review episode/finding and structured handoff. Include the handoff's scope/source context and lifecycle status if those details are implemented. Do not imply all conversation text is automatically trusted memory.

**Narration:**

> “CodeMind stores the useful finding and a structured handoff with its source context, rather than relying on the current chat or an in-memory object.”

### 1:58–2:30 — Fresh session: retrieve, consume, and validate

Open a genuinely new conversation/session and ask a related follow-up, for example:

> “Continue the review of `[ACTUAL_FIXTURE_SYMBOL]`. What previous context still applies, and what should I verify now?”

Show the receiving stage retrieving and consuming the persisted handoff from storage, then retrieving authorized memory and checking applicability/freshness against the current graph or snapshot. If the UI/test fixture supports it, briefly show a second previously validated memory that is now **stale** because the graph/snapshot changed; make clear it is retained as historical context but does not override current facts. Keep this stale record in a separate controlled fixture/graph variant so the positive memory path remains repeatable. If a stale-memory UI proof is not implemented and tested, do not stage it; use the verified positive flow and keep the stale case as a secondary proof clip only after it passes.

**Narration:**

> “In a fresh session, the receiving stage reads the handoff back from persistent storage. CodeMind validates old context against current evidence. Applicable memory can guide the next investigation, but stale or unknown memory never outranks the current graph.”

A previous assistant message in the same chat, browser-local state, hidden prompt replay, or an in-memory JavaScript handoff is not sufficient evidence.

### 2:30–2:40 — Close the investigation loop

Return to the current finding and evidence path. Keep a clear distinction between what is currently confirmed, what came from historical memory, and what remains uncertain.

**Narration:**

> “The graph explains what is connected, while persisted review context helps the next investigation begin with useful questions without treating memory as unquestionable truth.”

### 2:40–2:48 — Three-language proof

Use a quick, readable montage of actual completed investigations against the bundled Rust, TypeScript/JavaScript, and Python fixtures. Show the real workspace/language and actual result entry for each. Do not imply cross-language calls or edges unless the graph contains source-supported links.

**Narration:**

> “The same canonical investigation workflow works across Rust, TypeScript, and Python; each adapter contributes only the relationships supported by its source.”

If any language's E2E gate fails, omit the claim for that language and repair the build or narrow the submission claims.

### 2:48–2:55 — Architecture and track close

Show one compact diagram: **Next.js UI + Agent Runtime → gRPC/Protobuf → one Rust engine (CoIE/TIE/ChIE) → Graph Storage Port → FalkorDB**. Label the project and the two tracks: **Track 01 — Agents That Act on Connected Data** and **Track 02 — Agent Memory and Coordination**.

**Narration:**

> “CodeMind investigates connected code, traces evidence, and carries validated engineering context forward. FalkorDB is part of the reasoning path, not just a visualization.”

Do not finish on an error, spinner, empty page, unsupported deployment claim, or unverified metric.

---

## 7. Optional expanded language-coverage montage

The primary video already includes a brief 5–8 second three-language proof. For judge follow-up or a longer allowed cut, prepare an **expanded montage** showing the same investigation question running against each bundled language fixture:

- Rust.
- TypeScript/JavaScript.
- Python.

For each, show the real selected workspace/language and the real result entry. Do not imply that cross-language calls or relationships exist unless the fixture source and graph actually support them. The montage complements the detailed primary investigation; it does not replace the three end-to-end tests.

If adding this montage pushes the video beyond the time allowed by the final form, omit the montage. Preserve the graph-dependency proof and Track 02 fresh-session proof first.

---

## 8. Secondary proof clips (for questions or a longer allowed video)

These clips are not required in the concise primary cut. Keep them ready in case the form allows a longer video or a judge asks for a technical walkthrough.

### Architecture and graph-model backup clip

Prepare one legible architecture diagram showing **Next.js UI + Agent Runtime → gRPC/Protobuf → Rust `codemind-engine` (CoIE/TIE/ChIE modules) → Graph Storage Port → FalkorDB**. Also prepare the actual graph model with supported code, test, change, review, memory, and handoff entities/relationship types. Keep names and edges identical to the submitted implementation; do not include speculative entities from the broader product vision. This clip is supporting material, not a replacement for showing the working app.

### 8.1 Graph schema and query

Show the relevant Cypher or graph algorithm used by the implementation and explain the relationship pattern it follows. The query must be the actual query/algorithm the submitted product depends on, not a handwritten illustrative query that the application never executes. The README must also include the implemented graph data model and the important Cypher/algorithm details, as the rules require. Be ready to explain why removing or changing an edge changes the result and why a text-only or decorative graph would not provide the same guarantee.

### 8.2 Bounded traversal and honest incompleteness

Show a genuine partial/truncated/unresolved case and how CodeMind reports the limitation. This makes clear that the system does not claim omniscient repository understanding. Only include this if the behavior has been verified and the UI state is legible.

### 8.3 Static test intelligence boundary

Show how test candidates are discovered and connected to code. Say “test candidate” or the exact supported evidence terminology. Do not imply that CodeMind executed the repository's tests or proved coverage if the MVP has only static discovery/mapping.

### 8.4 Scope isolation

If this is particularly easy to demonstrate, show that memory for one workspace/scope is not retrieved for another. Use disposable synthetic data, not real tenant or user information. Never reveal actual credentials or secrets.

### 8.5 Stale-memory proof

Using the isolated, tested fixture variant, show a previously validated memory becoming stale after its referenced snapshot or relationship changes. Show the explicit freshness/applicability label and the current graph-backed result that takes precedence. Do not create a fake stale badge by editing UI state or changing only the display text. Keep this clip ready even if the short primary video can only show the positive cross-session retrieval path.

### 8.6 Local setup

For a local-only submission, record the documented setup instructions and show the actual app running against local FalkorDB. A live deployment is preferred by the overview, but the official rules accept complete local setup instructions when deployment is not available. Do not imply that a local recording itself is a hosted deployment.

### Judge-question rehearsal (notes, not a slide deck)

Prepare concise, evidence-backed answers to these questions; never rely on claims that the build cannot demonstrate:

- **Why a graph?** The result depends on following explicit multi-hop code/test relationships and preserving the evidence path; ordinary snippets alone do not establish that path.
- **Why FalkorDB?** The persisted FalkorDB edges are read by the actual bounded graph query/traversal, and changing a required edge changes the canonical result. If FalkorDB is removed, CodeMind loses that authoritative connected-data path rather than merely losing a visualization.
- **What does the LLM do?** It interprets the question, selects approved semantic tools, and explains bounded canonical results. It does not author canonical nodes, relationships, counts, test associations, or risk facts.
- **What does Rust do?** It owns deterministic CoIE/TIE/ChIE intelligence, graph access behind the storage port, canonical contracts, and evidence-backed results. The three engines are modules within one Rust service.
- **What happens when memory is wrong?** Memory has separate validity and freshness; it is retrieved within scope, checked against current graph/snapshot evidence, and marked stale/unknown when needed. Current canonical facts win.
- **Why three languages?** Each adapter uses real Rust, TypeScript/JavaScript, or Python source evidence but maps supported constructs to a shared canonical contract. Do not claim cross-language edges unless implemented.
- **Why no vector database / advanced autonomous swarm?** The MVP prioritizes structured graph retrieval, bounded semantic tools, and one real persisted handoff; embeddings and broader coordination are explicitly deferred.
- **What about MCP?** If MCP is genuinely implemented, show the real path. If it was cut to protect the core, explain that the MVP uses the approved semantic-tool registry and do not claim MCP support. MCP must not be simulated for the demo.
- **How can judges run it?** Point to the public repository, tested README setup instructions, actual graph model and queries/algorithms, and either the working deployment or verified local setup.
- **What AI assistance was used?** Point to the required disclosure and explain the code/architecture decisions directly; disclosure is not a substitute for understanding the implementation.

---

## 9. Narration principles

### 9.1 Say what the system did, not what it might theoretically do

Prefer:

> “For this selected symbol, these persisted relationships lead to these affected entities.”

Avoid:

> “CodeMind understands every repository perfectly.”

Prefer:

> “These are statically discovered test candidates supported by the current graph.”

Avoid:

> “CodeMind proves all affected tests pass.”

Prefer:

> “The prior finding was retrieved and checked against the current snapshot.”

Avoid:

> “The memory is always correct forever.”

### 9.2 Explain roles correctly

- **FalkorDB** stores the graph and supplies the persisted relationships used by connected reasoning and relevant persistent review context.
- **CoIE** builds/maintains canonical code facts.
- **TIE** contributes static test discovery and supported test relationships.
- **ChIE** derives change impact, qualitative risk, test candidates, and evidence-backed paths from those facts.
- **Rust** owns the deterministic engine-side domain logic.
- **The Agent Runtime in Next.js** orchestrates semantic tools and communicates with the Rust engine through gRPC/Protobuf.
- **The LLM** helps interpret the request and explain bounded canonical results; it is not the source of canonical graph facts.
- **The UI** renders typed progress and structured results; it does not scrape logs or infer risk from prose.

Do not call CoIE, TIE, and ChIE independent deployed microservices. They are modules inside one Rust service/crate.

### 9.3 Keep the result language honest

Preserve the differences between:

- Direct impact and indirect impact.
- A risk tier and confidence/completeness.
- A test candidate and a test actually executed.
- A declared association and confirmed coverage.
- A memory being valid and a memory being fresh.
- A handoff being created and a handoff being retrieved/consumed.
- A complete result and a partial, truncated, unresolved, unsupported, unavailable, or failed result.

A successful-looking UI is not permission to collapse these distinctions.

---

## 10. Capture, editing, and audio checklist

### Screen capture

- [ ] Capture at a resolution that keeps code paths and labels readable; 1920×1080 is a practical starting point if the machine can record smoothly.
- [ ] Use a steady pointer and pause briefly after every important state change.
- [ ] Avoid tiny fonts, excessive zoomed-out panels, and rapid tab switching.
- [ ] Capture evidence paths without hiding the path or relationship type behind hover states.
- [ ] Show the browser URL only if it helps; hide local usernames or sensitive directory paths.
- [ ] If showing FalkorDB Browser or a terminal, keep the relevant query/result legible and crop unrelated output.
- [ ] Do not show `.env` files, API keys, authorization headers, connection strings, or cloud credentials.
- [ ] If a stage is slow, trim waiting time in editing but preserve the authenticity and order of the actual operation. Do not speed up progress animations to suggest a false runtime.

### Audio

- [ ] Record a clear voice-over at consistent volume.
- [ ] Keep background music absent or very quiet; narration and product audio/alerts take priority.
- [ ] Use short sentences and explain acronyms the first time: “Code Intelligence Engine,” “Test Intelligence Engine,” and “Change Intelligence Engine,” if time permits.
- [ ] Do not narrate while the screen shows a different action from the one being described.
- [ ] Re-record any line that overstates functionality; do not try to repair a factual mismatch with subtitles alone.

### Editing

- [ ] Start with the user problem, not a logo animation.
- [ ] Cut dead time, accidental pauses, and irrelevant windows.
- [ ] Preserve the logical sequence of input → engine work → graph evidence → persisted handoff → fresh-session retrieval.
- [ ] Use captions for key terms if available, especially “FalkorDB relationship traversal,” “evidence path,” and “fresh-session handoff.”
- [ ] Do not use mockups or Figma screens in place of the working project.
- [ ] Avoid fabricated metrics, fabricated graph nodes, and edited results that imply the app returned data it did not return.
- [ ] End with project name and declared tracks, rather than a long credits screen.

---

## 11. Demo failure and recovery plan

Do not attempt to repair the application live during the final recording. Use the Day 4 runbook and the final known-good fixture state.

| Problem during rehearsal | Correct response |
|---|---|
| Public GitHub import fails | Use the bundled offline fixture. The main demo must not depend on public networking. |
| LLM provider unavailable | Run the tested offline fallback if it supports the intended demo. If it does not produce a real investigation, do not stage a fake response or claim the full workflow passed. |
| FalkorDB unavailable | Stop and repair the environment. A static graph illustration is not a substitute for the required primary graph database. |
| Rust engine/gRPC unavailable | Stop and repair the real end-to-end path. Do not switch to a mocked endpoint for the submission recording. |
| Impact path is empty or wrong | Reset/reseed the isolated fixture, verify the chosen symbol and edge, rerun the test, and only then record. Do not hand-edit the displayed output. |
| No useful test candidate is supported | Select a different fixture symbol with a tested association, or state the limitation honestly. Do not label a guessed test as mapped evidence. |
| Handoff does not survive a fresh session | Track 02 is not proven. Fix and retest the persisted flow before recording it. |
| Retrieved memory is stale or unknown | Show that state honestly if it is a purposeful scenario. For the primary positive story, use memory that has passed the current-evidence check. |
| UI displays the wrong snapshot or stage status | Treat as a defect. Do not cover it with voice-over. |
| Screen/audio recording is corrupted | Re-record the affected sequence after confirming application health. |
| Feature fails the gate on Day 4 | Remove the unproven claim and reevaluate the track selection; never claim a feature solely because it appears in the plan. |

Keep at least one clean local rehearsal before filming. Use a spare isolated graph for relationship mutation and the main seeded graph for the positive story.

---

## 12. Final review: viewer and compliance checklist

### Story and product

- [ ] The first 12 seconds explain the code-change investigation problem.
- [ ] The video shows a real question asked in the actual CodeMind UI.
- [ ] The execution timeline reflects real CoIE → TIE → ChIE progress.
- [ ] The output contains canonical findings and an inspectable evidence path.
- [ ] Direct/indirect impact labels are supported by the graph.
- [ ] Test candidates are described accurately as static evidence where applicable.
- [ ] Risk uses only implemented evidence-backed factors; no invented percentage is visible.
- [ ] The graph-dependency proof is shown and its test passes.
- [ ] Persisted review context and a structured handoff are visible.
- [ ] A new session proves persistent retrieval and the receiving-stage handoff flow.
- [ ] Memory is checked against current evidence; it is not presented as unquestionable truth.
- [ ] The primary video includes a real three-language proof and a compact architecture view, or the spoken/screen claims are narrowed if one language is not ready.
- [ ] A tested stale-memory proof clip is available for judge follow-up; stale historical context is not treated as current fact.
- [ ] The final cut contains no obvious error, secret, unrelated window, or fake metric.

### Hackathon compliance

- [ ] The final rules and submission form were checked again close to submission.
- [ ] The public source repository opens in a signed-out browser.
- [ ] The README includes setup instructions and the actual graph data model.
- [ ] The repository documents the Cypher queries/graph algorithms on which the product depends.
- [ ] The AI-assistance disclosure is present in the repository/submission material.
- [ ] The video corresponds to the same code revision that is submitted.
- [ ] The declared tracks are exactly the tracks for which the video provides proof.
- [ ] The project uses public, synthetic, or own permitted data only.
- [ ] Either the live deployment works, or complete local setup instructions are tested and included.
- [ ] The video link is accessible to reviewers under the access setting accepted by the submission form.
- [ ] Submission is completed before the official **18 October 2026, 11:59 PM IST** deadline.

---

## 13. Final recording sign-off

Fill this out immediately before recording the final cut and again before submission.

```text
Recording date/time (IST):
Git commit hash shown in the recording:
Submitted Git commit hash:
Demo workspace/fixture:
Main language:
Main selected symbol:
Verified multi-hop path shown:
Test candidate shown:
Risk factor/tier shown:
Graph relationship dependency proof:
Track 02 review episode/finding shown:
Persisted handoff identifier/reference:
Fresh session used:
Current-evidence validation shown:
Stale-memory fixture/test reference and visible-proof status (if supported):
Rust / TypeScript-JavaScript / Python E2E status:
Track 01 gate status:
Track 02 gate status:
Clean-start status:
Video duration:
Final video URL:
Repository URL:
Live deployment URL or local setup path:
AI-assistance disclosure location:
Rules/submission form rechecked at:
Final reviewer:
Sign-off:
```

### Final rule

**Record the system that passed the tests, not the system the plan hoped to build.** The most persuasive demo is a short, reproducible proof: a real graph relationship changes the investigation result, the evidence path is inspectable, and meaningful engineering context is retrieved and validated in a genuinely new session.
