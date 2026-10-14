# CodeMind Design System

**Version:** 1.0  
**Theme:** Dark-first  
**Product:** CodeMind  
**Visual direction:** Clean, minimal, modern, AI-focused, developer-friendly, consistent.

This document defines the reusable visual language and UI primitives for CodeMind. It does **not** define product pages, chat layouts, navigation, dashboards, repository screens, or investigation workflows. Those consume this system.

---

## 1. Design Principles

### Clean
- Minimal decoration.
- Clear hierarchy.
- Subtle borders.
- Controlled use of shadows.
- Content remains the focus.

### Developer-focused
The visual language is designed for code, graphs, tests, history, evidence, analysis, and technical workflows.

### AI-focused
Use the accent color for AI/semantic states without making the entire interface purple.

### Evidence over decoration
Technical conclusions should visually connect to evidence, relationships, confidence, and limitations.

### Consistent
Shared semantic states must always use the same visual treatment.

---

# 2. Color System

## 2.1 Primary

| Token | Hex | Usage |
|---|---|---|
| `primary` | `#3B82F6` | Primary actions, active states |
| `primary-hover` | `#2563EB` | Hover state |
| `primary-light` | `#60A5FA` | Highlights and secondary emphasis |

## 2.2 Accent

| Token | Hex | Usage |
|---|---|---|
| `accent` | `#8B5CF6` | AI/semantic features and special emphasis |

Purple must not replace blue for normal actions.

## 2.3 Semantic Colors

| Token | Hex | Usage |
|---|---|---|
| `success` | `#22C55E` | Successful/completed/valid states |
| `warning` | `#F59E0B` | Warnings, attention, degraded states |
| `danger` | `#EF4444` | Errors, high risk, destructive states |

## 2.4 Dark Surfaces

| Token | Hex | Usage |
|---|---|---|
| `background` | `#080F14` | Application background |
| `surface` | `#111827` | Main cards and panels |
| `surface-alt` | `#1F2937` | Secondary surfaces |
| `border` | `#374151` | Borders and dividers |

## 2.5 Text

| Token | Hex | Usage |
|---|---|---|
| `text-primary` | `#F9FAFB` | Main text |
| `text-secondary` | `#9CA3AF` | Supporting text |
| `text-muted` | `#6B7280` | Low-priority metadata |

## Color Rules

- Do not introduce arbitrary component colors.
- Use semantic tokens rather than raw color values inside components.
- Do not communicate important state through color alone.
- Reserve bright colors for interaction and meaningful state.

---

# 3. Typography

## 3.1 Font Families

Primary UI font:

```text
Inter, system-ui, sans-serif
```

Technical/code font:

```text
JetBrains Mono, Fira Code, ui-monospace, monospace
```

## 3.2 Type Scale

| Token | Size | Line Height | Weight | Usage |
|---|---:|---:|---|---|
| `display-1` | 48px | 56px | Bold | Major product headings |
| `display-2` | 36px | 44px | Bold | Large headings |
| `heading-1` | 30px | 36px | Semibold | Major sections |
| `heading-2` | 22px | 30px | Semibold | Subsections |
| `heading-3` | 18px | 26px | Medium | Card/component titles |
| `body` | 16px | 24px | Regular | Primary text |
| `small` | 14px | 20px | Regular | Supporting text |
| `caption` | 12px | 16px | Regular | Metadata/captions |

Technical identifiers, filenames, paths, symbols, IDs, and code use the code font.

---

# 4. Spacing

Use a 4px base scale.

```text
space-1   = 4px
space-2   = 8px
space-3   = 12px
space-4   = 16px
space-5   = 20px
space-6   = 24px
space-8   = 32px
space-10  = 40px
space-12  = 48px
space-16  = 64px
```

Prefer tokens over arbitrary spacing.

---

# 5. Border Radius

```text
radius-sm   = 6px
radius-md   = 8px
radius-lg   = 12px
radius-xl   = 16px
radius-full = 9999px
```

Typical usage:

- `sm`: small controls
- `md`: buttons and inputs
- `lg`: cards and panels
- `xl`: large containers/modals
- `full`: chips, badges, status pills

---

# 6. Borders

Default:

```text
1px solid #374151
```

Borders should be subtle. Avoid boxing every nested element.

Focused/selected controls may use the primary color plus a restrained focus ring.

---

# 7. Shadows

The visual language is mostly flat.

```text
shadow-sm
shadow-md
shadow-lg
```

Use elevation mainly for:

- dropdowns
- popovers
- modals
- floating panels

Cards normally rely on `surface + border`.

---

# 8. Buttons

## Primary

For the main action.

```text
background: primary
text: white
radius: radius-md
```

States:

```text
default
hover
active
focus
disabled
loading
```

## Secondary

```text
background: surface
border: border
text: text-primary
```

Hover uses `surface-alt`.

## Ghost

```text
background: transparent
border: transparent
```

Hover uses `surface-alt`.

## Destructive

Use `danger` only for genuinely destructive actions.

## Sizes

```text
sm = 32px
md = 40px
lg = 44px
```

Default:

```text
height: 40px
padding-x: 16px
radius: 8px
```

Buttons may contain icons.

---

# 9. Icon Buttons

Icon-only buttons:

- require an accessible label
- should provide a tooltip when the meaning is not obvious
- use at least a 32px hit area
- use consistent icon sizing

```text
sm = 32px
md = 36px
lg = 40px
```

---

# 10. Icons

Use one consistent outlined icon family.

Core categories include:

```text
chat
repository
file
code
search
graph
history
git
bug
test
settings
copy
download
external-link
```

Default icon color:

```text
text-secondary
```

Active state:

```text
text-primary / primary
```

Avoid mixing filled and outlined styles without a clear reason.

---

# 11. Inputs

Default:

```text
background: surface
border: border
text: text-primary
placeholder: text-secondary
radius: radius-md
```

Focus:

```text
border: primary
```

Supported states:

```text
default
hover
focus
disabled
error
success
```

Error uses `danger`; success uses `success`.

---

# 12. Textareas

Textareas follow the same visual contract as inputs.

Rules:

- consistent padding
- readable line height
- appropriate minimum height
- vertical resizing where useful
- code content uses the code font

---

# 13. Selects and Dropdowns

Dropdown surfaces:

```text
surface
border
shadow-md
radius-lg
```

Hovered option:

```text
surface-alt
```

Selected option may use a subtle primary-tinted background.

---

# 14. Chips and Tags

Use chips for compact entity/category labels.

Examples:

```text
File
Function
Module
Test
```

Recommended:

```text
height: 24px
padding-x: 8px
radius: full
font-size: 12px
```

Visual pattern:

```text
[ ● File ] [ ● Function ] [ ● Module ] [ ● Test ]
```

---

# 15. Badges

Badges communicate state.

Examples:

```text
High Risk
Medium Risk
Low Risk
Completed
Running
Failed
Partial
Truncated
```

Semantic mapping:

```text
success → green
warning → amber
danger  → red
neutral → gray
accent  → purple
```

Always include a textual meaning.

---

# 16. Cards

Cards are a primary CodeMind primitive.

Default:

```text
background: surface
border: 1px solid border
radius: radius-lg
padding: 16px
```

Large analytical cards may use 20–24px padding.

Structure:

```text
Card
├── CardHeader
├── CardTitle
├── CardDescription
├── CardContent
└── CardFooter
```

Cards should establish hierarchy, not simply wrap every piece of content.

For affected files, findings, test candidates, tool outputs, and other repeated structured records, use a compact `DataTable` or `ResultList` when comparison and scanning matter more than card-by-card reading. Rows must retain relevant evidence/status affordances, use the code font for paths/symbols, and support horizontal overflow or a clear stacked presentation on small screens. Do not put a long repository-wide result into one oversized card merely to avoid a table.

---

# 17. Analytical Cards

Specialized analytical components extend the base card.

```text
ImpactCard
RiskCard
TestImpactCard
EvidenceCard
GraphPathCard
MemoryCard
OperationStatus
HandoffStatus
MemoryContextCard
```

These components must use the same tokens as primitive cards. Memory/context components must visually separate current graph facts from historical memory and display scope, provenance, freshness/applicability, and lifecycle whenever the structured result supplies them. Handoff status must reflect persisted state, not merely the existence of a local object.

---

# 18. Metric Cards

Use for meaningful quantitative values.

Structure:

```text
value
label
optional status/context
```

Example:

```text
4
Direct dependencies
```

Do not use metric cards merely to decorate a screen.

---

# 19. Tabs

Tabs switch between closely related views.

Active:

```text
text-primary
primary indicator
```

Inactive:

```text
text-secondary
```

Tabs should not look like unrelated buttons. The active tab may reflect navigation state only; do not style it as a risk/severity state. If a tab contains partial or unavailable data, show that state in the panel or a textual count/status rather than silently making the tab look complete.

---

# 20. Accordion / Disclosure

Use for secondary technical detail:

- raw evidence
- graph traversal details
- diagnostics
- tool execution details
- historical context

Progressive disclosure is preferred over displaying all technical detail at once.

---

# 21. Modal

```text
background: surface
border: border
radius: radius-xl
shadow-lg
```

Structure:

```text
Title
Description
Content
Actions
```

Use modals for focused tasks, not large workflows.

---

# 22. Side Panel

Side panels are appropriate for secondary technical context.

Typical content:

- file details
- history
- tests
- evidence
- graph details

Structure:

```text
Header
────────────
Context
────────────
Content
────────────
Actions
```

---

# 23. Tooltips

Tooltips clarify unfamiliar icon-only actions.

```text
background: surface-alt
border: border
text: text-primary
radius: radius-sm
font: 12px / 16px
```

Essential information must not exist only in a tooltip.

---

# 24. Status Indicators

Use semantic status indicators consistently.

```text
● Running
● Completed
● Warning
● Failed
```

Mapping:

```text
running / active engine stage → primary
completed                    → success
warning / degraded           → warning
failed / security denied     → danger
LLM reasoning / semantic step → accent
waiting / unknown             → neutral / text-secondary
```

Important statuses should include text. Do not make users infer status from color or shape alone.

### Analysis state semantics

The UI must preserve these as separate dimensions; do not compress them into one badge or one numeric score:

| Dimension | Example values | Presentation rule |
|---|---|---|
| Operation lifecycle | `created`, `running`, `waiting`, `completed`, `failed`, `cancelled` | Describe what the operation is doing/has done. |
| Outcome | `success`, `degraded`, `failure`, `cancelled` | Describe the result of the operation, not the progress animation. |
| Completeness | `complete`, `partial`, `truncated`, `unknown` | Keep limitations adjacent to the affected result/path. |
| Evidence freshness | `fresh`, `stale`, `unknown` | Make stale/unknown evidence explicit; never style it as current confirmed truth. |
| Memory validity | `candidate`, `validated`, `rejected`, `superseded` | Describe the memory's lifecycle, independent of freshness. |
| Handoff lifecycle | `pending`, `claimed`, `consumed`, `expired` | Show the actual persistence/claim/consume state, separate from operation completion. |

A `validated + stale` memory is possible and must be rendered as such. A consumed handoff does not by itself mean the overall operation succeeded. Partial, truncated, unavailable, unresolved, unsupported, invalid, failed and cancelled states must not share one generic success treatment. Use existing semantic tokens; only add an explicit text label/icon/context, not an arbitrary new color.

---

# 25. Loading

Use the appropriate loading primitive:

- spinner for short operations
- skeleton for content loading
- staged progress for multi-step operations

Meaningful stages are preferable to an unexplained infinite spinner.

---

# 26. Skeletons

Skeletons should mirror the structure of the content being loaded.

Use the dark surface hierarchy, especially `surface-alt`.

Animations must remain subtle and respect reduced-motion preferences.

---

# 27. Error States

An error state should communicate:

1. What happened.
2. Whether the operation failed or is partially available.
3. What the user can do next.

Structure:

```text
[Error icon]

Analysis failed

We couldn't complete the dependency analysis.

[Try Again] [View Details]
```

Raw stack traces should not be the primary user-facing message.

---

# 28. Empty States

Empty states explain why content is absent.

Structure:

```text
Icon
Title
Explanation
Optional action
```

Avoid vague text such as `Nothing here.`

---

# 29. Code Blocks

Code is a first-class content type.

Use:

```text
font-code
background: surface
border: border
radius: radius-lg
```

Code blocks may support:

- syntax highlighting
- line numbers
- copy
- file/path context
- selected lines
- expandable sections

---

# 30. Technical Entity Representation

Use consistent visual identities.

| Entity | Representation |
|---|---|
| File | File icon + filename |
| Function | Function/code icon + symbol |
| Module | Module icon + name |
| Test | Test icon + test name |
| Repository | Repository icon + name |
| Commit | Git icon + short SHA |

Technical identifiers should use the code font where appropriate.

---

# 31. Risk Visualization

Risk is never represented only by color.

Use:

```text
High Risk
Medium Risk
Low Risk
Unknown
```

A risk component should provide:

- qualitative risk label and explanation;
- the supported factors/evidence behind the label;
- an icon and semantic color as secondary cues.

Keep **impact, risk, confidence, completeness, and review severity** visibly distinct when the structured result supplies them. Do not compute or imply a composite score in the UI, and do not display percentages or numerical precision unsupported by canonical data. `Unknown` must remain a valid state rather than being styled as low risk.

---

# 32. Graph Visualization Tokens

Graph visuals must follow the same semantic system.

Suggested node mapping:

```text
Current entity      → primary
Direct dependency   → success
Indirect dependency → accent
Test                → primary-light
External entity     → neutral
Risk-related        → warning/danger
```

Edges remain quieter than nodes.

```text
default     → border / secondary
highlighted → primary
selected    → accent
```

Graph colors should communicate meaning, not decoration.

Additional relationship/path rules:

- Show edge direction with an arrowhead or another unambiguous directional affordance; do not rely on color alone.
- Show or expose the canonical relationship type for every path edge, especially when expanded or selected. Preserve distinctions between relationship types instead of renaming them to a generic “dependency” label.
- Provide a text/list representation of the selected path and its evidence so keyboard and assistive-technology users are not forced to infer meaning from a canvas.
- Distinguish direct from indirect impact with labels and relationship depth, not only two colors. Highlight the selected evidence path without visually implying that adjacent/unselected edges prove the same conclusion.
- If the graph or traversal is partial/truncated, mark the view accordingly; do not make a partial graph look complete because the visible layout is tidy.

---

# 33. Graph Controls

Graph controls are normal CodeMind controls.

Examples:

```text
Fit
Layout
Zoom
Reset
Filter
```

Use standard button, icon-button, select, and tooltip primitives.

---

# 34. Timeline

Use timelines for:

- commit history
- operation stages
- analysis progression
- historical events

Completed/current/failed events should use the shared status semantics. Keep stage progress tied to real typed operation events; do not use a timer-driven animation or invented percentage to imply progress. If a stage is waiting for engine publication or a result, label that wait rather than marking it completed. Reconnect/duplicate events must not visually rewind a timeline to an earlier state.

---

# 35. Alerts

Supported types:

```text
info
success
warning
error
```

Structure:

```text
Icon
Title
Description
Optional action
```

Use restrained tinted backgrounds.

---

# 36. Accessibility

Every interactive component must provide:

- keyboard accessibility
- visible focus state
- accessible icon labels
- sufficient contrast
- state communication beyond color
- meaningful button names
- associated form labels

Touch targets should remain usable on smaller screens.

---

# 37. Responsive Design

Desktop is the primary environment, but components must remain responsive.

Suggested breakpoints:

```text
sm  = 640px
md  = 768px
lg  = 1024px
xl  = 1280px
2xl = 1536px
```

Components should reflow rather than simply shrink.

---

# 38. Motion

Motion is functional, not decorative.

Use it for:

- state transitions
- loading
- panel opening
- graph interactions
- progress updates
- focus/hover feedback

Suggested durations:

```text
fast   = 100–150ms
normal = 150–250ms
slow   = 250–350ms
```

Respect:

```text
prefers-reduced-motion
```

---

# 39. Component States

Reusable components should use consistent state names:

```text
default
hover
active
focus
disabled
loading
error
success
selected
```

A component only needs states that make sense for its behavior.

---

# 40. Component Naming

Primitive components:

```text
Button
IconButton
Input
Textarea
Select
Badge
Chip
Card
MetricCard
Tabs
Accordion
Tooltip
Modal
SidePanel
Alert
Skeleton
CodeBlock
Timeline
StatusIndicator
DataTable
ResultList
```

CodeMind-specific components:

```text
ImpactCard
RiskCard
TestImpactCard
EvidenceCard
GraphPathCard
MemoryCard
MemoryContextCard
HandoffStatus
OperationStatus
```

---

# 41. Token Architecture

The implementation follows three layers:

```text
Design Tokens
      ↓
Primitive Components
      ↓
CodeMind Components
```

## Layer 1: Tokens

```text
colors
typography
spacing
radius
borders
shadows
motion
breakpoints
```

## Layer 2: Primitives

```text
Button
Input
Card
Badge
Tabs
Modal
Tooltip
DataTable
ResultList
...
```

## Layer 3: Product Components

```text
ImpactCard
RiskCard
EvidenceCard
GraphPathCard
TestImpactCard
MemoryCard
MemoryContextCard
HandoffStatus
OperationStatus
...
```

Product components must consume the existing system instead of inventing local visual tokens.

---

# 42. CSS Variable Reference

A token implementation can expose:

```text
--color-background
--color-surface
--color-surface-alt
--color-border

--color-primary
--color-primary-hover
--color-primary-light

--color-accent

--color-success
--color-warning
--color-danger

--color-text-primary
--color-text-secondary
--color-text-muted

--font-sans
--font-mono

--text-display-1
--text-display-2
--text-heading-1
--text-heading-2
--text-heading-3
--text-body
--text-small
--text-caption

--space-1
--space-2
--space-3
--space-4
--space-5
--space-6
--space-8
--space-10
--space-12
--space-16

--radius-sm
--radius-md
--radius-lg
--radius-xl
--radius-full
```

---

# 43. Explicit Scope Boundary

This document intentionally does **not** define:

- Homepage
- Chat page
- Repository page
- Analysis page
- Navigation
- Sidebar
- Dashboard
- Investigation workflow
- Agent orchestration UI
- CoIE/TIE/ChIE workflow screens
- Graph page layout
- History page layout
- Memory page layout
- Demo flow

Those are product-level UI specifications.

The design system defines the reusable visual language that those screens consume.

---

# 44. Final Hierarchy

```text
CodeMind Design System
│
├── Principles
├── Colors
├── Typography
├── Spacing
├── Radius
├── Borders
├── Shadows
├── Motion
├── Icons
│
├── Primitive Components
│   ├── Button
│   ├── IconButton
│   ├── Input
│   ├── Select
│   ├── Textarea
│   ├── Badge
│   ├── Chip
│   ├── Card
│   ├── Tabs
│   ├── Accordion
│   ├── Tooltip
│   ├── Modal
│   ├── SidePanel
│   ├── Alert
│   ├── Skeleton
│   └── Timeline
│
├── Technical Components
│   ├── CodeBlock
│   ├── MetricCard
│   ├── StatusIndicator
│   ├── DataTable
│   ├── ResultList
│   ├── RiskIndicator
│   ├── GraphNode
│   └── GraphEdge
│
├── CodeMind Components
│   ├── ImpactCard
│   ├── RiskCard
│   ├── TestImpactCard
│   ├── EvidenceCard
│   ├── GraphPathCard
│   ├── MemoryCard
│   ├── MemoryContextCard
│   ├── HandoffStatus
│   └── OperationStatus
│
├── Accessibility
├── Responsive Rules
└── Token Architecture
```

**Design-system rule:** product screens should consume this system, not redefine it.
