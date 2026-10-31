# Scriberig Architecture Context

## Purpose

This file is the compact architecture spine for Scriberig.

It is intended to be attached permanently to model/agent project context so that long conversations do not lose the system's controlling invariants while focusing on one subsystem.

This file does **not** replace the numbered design documents. It summarizes them and points to the authoritative sources. When a detail matters, read the referenced Level-1/2/3 document before changing code or design.

The most important rule for using this file is:

> Do not silently fall back to the architecture of a conventional coding agent. Scriberig is a semantic knowledge system with recursive agent execution, durable plans, structured change, and projection-based files.

---

# 1. What Scriberig Is

Scriberig is a universal semantic knowledge platform plus integrated agent/product surfaces.

It supports source code, Markdown, imported documents, sessions, branches, plans, decisions, model execution, derived analysis, migrations, and future domains through one semantic substrate.

CLI, TUI, HTTP server, Rust clients, reusable web packages, the integrated editor, and agents are consumers of shared application semantics. They do not each own independent versions of project/session/change behavior.

Authoritative:

- `level-1/001. vision and principles.md`
- `level-1/002. universal knowledge model.md`
- `level-1/011. runtime libraries protocol and product surfaces.md`

---

# 2. Canonical Knowledge Is Semantic, Not File Text

Known content is represented as typed semantic entities with properties, relations, collections, stable identities, immutable revisions, and exact snapshots.

For source code this may include:

```text
Project
  Module
    Type
    Function
      Parameter
      CodeGroup
        Statement
          Expression
    Comment
```

For Markdown/document content this may include:

```text
Document
  Section
    Paragraph
    List
    Table
      Row
        Cell
```

The physical storage granularity may differ from semantic granularity, but agents and applications should address meaningful semantic objects whenever a domain understands them.

A function is primarily a function, not a path plus line range. A paragraph is primarily a paragraph, not a byte span.

Authoritative:

- `level-1/002. universal knowledge model.md`
- `level-1/007. domain modeling and adapters.md`

---

# 3. Stable Identity, Immutable Revisions, Exact Snapshots

Keep these questions distinct:

```text
EntityId   -> What conceptual thing is this?
RevisionId -> What exact state of that thing is this?
SnapshotId -> What exact state of the canonical knowledge world is this?
```

Semantic edits create new immutable revisions and new snapshot state rather than mutating history in place.

Exact historical/context-sensitive work must resolve against explicit revisions/snapshots.

Authoritative:

- `level-1/003. identity revisions and snapshots.md`

---

# 4. Files Are Projections

For understood domains, `.rs`, `.md`, and similar files are external projections of canonical semantic state.

Normal direction:

```text
canonical semantic state
        -> render
        -> source/document projection
```

External editing remains supported through:

```text
edited projection
  -> parse
  -> candidate semantic structure
  -> reconcile identity/structure
  -> semantic migration
  -> validated canonical state
  -> rerender
```

The filesystem is an interoperability surface, not the final authority.

Do not design normal agent editing around generic `write_file`, textual patching, line offsets, or shell substitution when a semantic adapter exists.

Raw file editing remains a fallback for unknown/opaque formats, bootstrap, interoperability, or carefully bounded exceptional cases.

Authoritative:

- `level-1/006. projections and bidirectional editing.md`
- `level-1/007. domain modeling and adapters.md`
- `level-1/019. semantic agent operations and projection-first editing.md`

---

# 5. Agents Use Semantic Operations First

The universal interaction grammar is based on semantic handles and operations such as:

```text
inspect
list
follow
find
query
diff
history
render
branch
apply
commit
```

Domains add higher-level operations such as:

```text
code.add_function
code.replace_expression
code.insert_statement
rename_symbol
extract_function
document.insert_paragraph
table.insert_row
```

Those operations still resolve through the shared change/migration model.

When a discussion says "write a file" as shorthand, translate it into the semantic intent unless the file/domain is genuinely opaque.

For example, prefer:

```text
create Module
add Function
insert Statement
replace Expression
validate Candidate
render projection
```

over:

```text
write_file("foo.rs", ...)
```

Authoritative:

- `level-1/010. universal query and operation model.md`
- `level-1/019. semantic agent operations and projection-first editing.md`

---

# 6. All Canonical Writes Converge on One Change Model

Humans, agents, CLI, web, TUI, importers, and external projection reconciliation do not receive privileged mutation paths.

Canonical change is represented through branches/workspaces, candidate state, migrations, validation, merge/conflict semantics, and explicit promotion/commit.

Important distinction:

```text
semantic operation
    -> candidate/migration
    -> validation
    -> review/policy
    -> promotion
    -> current canonical root
```

not:

```text
agent
    -> directly overwrite live source/database
```

Concurrent work is normal. Disjoint semantic entities/collections can often merge more accurately than file-level patches.

Authoritative:

- `level-1/005. changes migrations branches and merge.md`
- `level-1/014. agent native semantic self hosting.md`

---

# 7. Validation May Use External Processes, but Shell Is Not the Agent's Main Editing Model

Compilation, tests, formatters, package tools, and other validators may require filesystem/process execution.

Prefer application capabilities such as:

```text
validate(candidate, profile)
dependency.add(...)
projection.materialize(...)
```

which may internally run external processes in isolated candidate workspaces.

An unrestricted model-facing shell is not the ordinary source mutation interface.

Validation evidence binds to the exact candidate state it checked.

Authoritative:

- `level-1/014. agent native semantic self hosting.md`
- `level-1/019. semantic agent operations and projection-first editing.md`

---

# 8. Sessions Are Recursive Trees, Not Linear Transcripts

A session contains recursively nested branches of focused work.

Branches may be created by:

```text
user historical continuation
agent delegation
plan execution
runtime isolation
retry/repair
validation
alternative implementation
conflict resolution
research/investigation
```

User-created forks and agent-created children use the same branch architecture.

A branch is primarily a **context-isolation boundary**, not a visual indentation device.

Authoritative:

- `level-1/009. agent interaction and sessions.md`
- `level-1/016. model driven agent execution and branch isolation.md`
- `level-1/017. recursive agent decomposition and context isolation.md`

---

# 9. Recursive Decomposition Is Expected Agent Behavior

An agent may recursively divide a task into smaller tasks when that reduces context pollution or creates clearer validation boundaries.

Example:

```text
Implement feature
|
+-- Discover architecture
|     -> BranchResult
|
+-- Implement parser capability
|   |
|   +-- modify semantic source structure
|   +-- validate parser
|       -> BranchResult
|   -> BranchResult
|
+-- Write tests
|     -> BranchResult
|
+-- Integrate
      -> BranchResult
```

Not every atomic operation creates a child branch. Independent reasoning loops/speculative work/validation/delegation are good branch boundaries.

Authoritative:

- `level-1/017. recursive agent decomposition and context isolation.md`
- `level-2/023. agent runtime model provider and branch orchestration.md`

---

# 10. BranchResult Is the Main Information Boundary

A child receives selected context, not the complete parent transcript.

A parent normally receives a compact durable `BranchResult`, not the complete child transcript.

A result may include:

```text
status
summary
important semantic handles
candidate/migration/workspace handles
touched entities
validation evidence
unresolved questions
failure summary
```

The complete child history remains durable and inspectable by reference.

Sibling branches communicate through semantic state/results rather than inherited conversational noise.

This result-collapse mechanism is central to Scriberig's context scaling.

Authoritative:

- `level-1/009. agent interaction and sessions.md`
- `level-1/016. model driven agent execution and branch isolation.md`
- `level-1/017. recursive agent decomposition and context isolation.md`

---

# 11. Context Is Compiled, Not Transcript-Concatenated

A model invocation receives a deterministic projection of relevant state.

Typical worker context is:

```text
branch purpose
exact continuation anchor
semantic scope/snapshot/workspace
selected ancestor BranchResults
fulfilled dependency results
accepted decisions/constraints
local branch history
capability/tool catalog
resolved execution configuration
```

It normally excludes:

```text
unrelated sibling transcripts
completed child transcripts already represented by BranchResult
resolved retry chatter
other providers' local execution history
arbitrary later wall-clock activity
```

UI expansion/collapse never changes model context.

Authoritative:

- `level-1/009. agent interaction and sessions.md`
- `level-1/016. model driven agent execution and branch isolation.md`
- `level-1/017. recursive agent decomposition and context isolation.md`
- `level-2/023. agent runtime model provider and branch orchestration.md`

---

# 12. Durable Plans Are Structured Session Knowledge

`/plan` creates a durable structured plan, not only a prose checklist.

A plan has:

```text
goal
revision/status
plan nodes
branch/future-branch identities
dependencies
assignments
scheduling state
coordinator
```

Plan-node scheduling states may include:

```text
planned
blocked
ready
running
completed
failed
cancelled
skipped
```

Planned nodes appear in the same visible session tree as executed work.

Authoritative:

- `level-1/018. durable plans dependency scheduling and multi-model execution.md`

---

# 13. Branch Tree and Dependency Graph Are Different Structures

Do not turn the branch tree itself into a DAG.

```text
parent/child branch relation
    -> context ownership, decomposition, provenance, result collapse

depends_on relation
    -> scheduling constraint between plan nodes
```

A node becomes ready when required dependencies are successfully fulfilled according to policy.

Dependencies are fulfilled by durable results, not merely because a worker stopped running.

A dependent worker consumes dependency results, not dependency transcripts.

Authoritative:

- `level-1/018. durable plans dependency scheduling and multi-model execution.md`

---

# 14. One Auto-Advancing Coordinator, Many Workers

One active plan has at most one global auto-advancing coordinator.

The coordinator may:

```text
determine ready nodes
dispatch unassigned work
react to results/failures
revise the plan
add decomposition
advance the coordinator cursor
decide plan completion
```

Workers own one focused branch and may recursively decompose their own assigned branch according to policy, but they do not autonomously claim/reorder unrelated sibling plan nodes.

Many workers may run concurrently on different branches.

Authoritative:

- `level-1/018. durable plans dependency scheduling and multi-model execution.md`

---

# 15. Models and Providers Are Per-Branch/Per-Node Configurable

The same session/plan tree may use different model/provider profiles concurrently.

Example:

```text
architecture   -> local fast model
parser         -> OpenAI profile
research       -> OpenRouter profile
integration    -> another OpenAI/model profile
documentation  -> local model
```

Execution configuration resolution is scoped and explicit. Running invocations are immutable with respect to later configuration changes.

Credentials remain opaque references and never become session/activity/prompt content.

One branch normally has one active executing owner. Independent competing attempts should use sibling/attempt branches rather than racing mutations in one branch.

Authoritative:

- `level-1/018. durable plans dependency scheduling and multi-model execution.md`
- task 133 model/thinking controls

---

# 16. Provider Continuation Is Secondary Branch-Local Runtime State

Providers may offer server-side conversation/session/response continuation.

Scriberig may use it for efficiency, continuity, or preservation of provider-side reasoning state.

The invariant is:

```text
Scriberig Session/Branch
    = canonical identity

Provider conversation/session/response handle
    = opaque branch-local runtime continuation state
```

Do not map the entire Scriberig session tree to one provider conversation.

A child/sibling branch normally receives independent provider continuation seeded from its selected compiled Scriberig context.

Continuation reuse is guarded by exact context lineage/fingerprints. Provider/profile/context divergence causes explicit fork/reseed/fallback rather than blind reuse.

Loss of provider continuation must not make Scriberig history unrecoverable.

Authoritative:

- `level-1/020. provider continuation state and observable model execution.md`
- `level-2/025. provider continuation and observable model execution.md`

---

# 17. OpenAI Provider State Does Not Replace Scriberig Sessions

The OpenAI adapter may use provider mechanisms such as a durable Conversation or previous-response chaining when supported.

Those are adapter strategies only.

A normal mapping is:

```text
one isolated Scriberig branch execution lineage
    -> one compatible provider continuation lineage
```

Do not make OpenAI Conversation IDs public semantic session IDs.

Do not assume provider retention is Scriberig durability.

When provider state is unavailable, recompile canonical Scriberig context and establish new provider state.

Authoritative:

- `level-1/020. provider continuation state and observable model execution.md`
- `level-2/025. provider continuation and observable model execution.md`

---

# 18. Full Prompt Means Complete Safe Effective Scriberig Request

Users may independently choose to inspect the complete prompt/context supplied to a model invocation.

`Full prompt` means the complete safe provider-neutral Scriberig request immediately before provider-specific serialization, including as applicable:

```text
instructions
conversation/context sections
semantic context
selected ancestor results
fulfilled dependency results
tool/semantic capability definitions
resolved model/provider/thinking config
provider continuation strategy/reference
```

It does **not** mean raw outbound HTTP JSON.

This matters when provider continuation is used: the wire request may contain only a continuation ID plus new input, while the effective model context is much larger.

A bounded activity projection may be truncated, but `Full` must resolve the genuinely complete retained safe request. Never label a larger truncation limit as `Full`.

Authoritative:

- `level-1/020. provider continuation state and observable model execution.md`
- `level-2/025. provider continuation and observable model execution.md`
- task 136

---

# 19. Provider-Visible Reasoning Is Ordered and Independently Presentable

Some providers/models expose user-visible reasoning summaries or reasoning content. Others expose none.

Scriberig may capture only reasoning content explicitly available for user/application display.

Reasoning may appear interleaved:

```text
reasoning segment
semantic/tool operation
result
reasoning segment
tool operation
result
reasoning segment
assistant output
```

Preserve this deterministic order.

Do not request, expose, or invent private hidden chain-of-thought.

Provider-internal/encrypted reasoning needed for continuation remains provider execution state, not user-visible trace.

Authoritative:

- `level-1/020. provider continuation state and observable model execution.md`
- `level-2/025. provider continuation and observable model execution.md`
- task 136

---

# 20. Prompt, Reasoning, Tool, and Result Presentation Are Independent

Execution configuration and presentation preferences are different systems.

Execution affects what the model does:

```text
model/provider/profile
thinking level
generation parameters
branch/invocation assignment
```

Presentation affects only what a client renders:

```text
prompt: Hidden | Summary | Full
reasoning: Hidden | Summary | FullAvailable
operation/tool invocation detail
operation/tool result detail
branch expansion
plan/dependency detail
```

Valid combinations include:

```text
prompt Full + reasoning Hidden
prompt Hidden + reasoning FullAvailable
```

Changing presentation must never rerun the model/tool, alter context, move reply anchor, change scheduling, or mutate semantic history.

A preset may set several preferences at once but must not replace their independent fields.

Authoritative:

- task 133
- `level-1/020. provider continuation state and observable model execution.md`
- task 136

---

# 21. There Are Several Different Graphs — Do Not Collapse Them

At minimum distinguish:

```text
1. Knowledge graph
   Project -> Module -> Function -> Statement -> Expression

2. Session branch tree
   Goal -> Branch -> Child Branch -> BranchResult

3. Plan dependency graph
   PlanNode A ----\
                   -> PlanNode C
   PlanNode B ----/

4. Execution/activity causality
   ModelInvocation -> Reasoning/Operation -> Result -> Output
```

A UI may project several structures together, but their semantics remain distinct.

Authoritative:

- `level-1/002. universal knowledge model.md`
- `level-1/017. recursive agent decomposition and context isolation.md`
- `level-1/018. durable plans dependency scheduling and multi-model execution.md`
- `level-1/020. provider continuation state and observable model execution.md`

---

# 22. The Default TUI Is a Tree-Aware Conversation Projection

The TUI should feel immediately usable like a modern coding-agent terminal:

```text
launch
-> type immediately
-> conversation/tree viewport
-> persistent composer
```

But it is not a flat append-only transcript.

New activity is inserted at its structural branch/plan location.

Completed child branches collapse to compact results by default and remain expandable.

Vertical depth rails show derived ancestry.

Width changes wrapping/detail, not the product mode; Explorer/Inspector/etc. do not automatically appear because the terminal is wide.

Auxiliary views are explicit slash-command destinations.

Authoritative:

- `level-1/015. terminal user interface principles.md`
- task 134
- task 135

---

# 23. Navigation Cursor, Reply Anchor, and Coordinator Cursor Are Different

Do not represent any of these only by terminal row index.

```text
navigation cursor
    -> what the user is browsing

reply anchor
    -> exact stable semantic/session location where next human input attaches

coordinator cursor
    -> current location of the one auto-advancing plan coordinator
```

The reply anchor may follow the coordinator while idle, but once the user deliberately pins/begins composing against another node, coordinator movement must not steal the reply target.

Presentation changes/collapse/expand/live sibling insertion must not change anchor identity.

Authoritative:

- `level-1/017. recursive agent decomposition and context isolation.md`
- `level-1/018. durable plans dependency scheduling and multi-model execution.md`
- task 134
- task 135

---

# 24. Structural Rails and Reply Highlight Mean Different Things

Depth rails encode ancestry.

Reply-anchor highlight encodes where the next user message attaches.

Do not use one cue for both.

Rail color/indentation/background highlighting are presentation state derived from semantic identity. They are never persisted as canonical branch data.

Authoritative:

- `level-1/017. recursive agent decomposition and context isolation.md`
- task 134

---

# 25. Slash Commands Are Local Product Commands

Recognized commands such as:

```text
/model
/assign
/thinking
/view
/plan
/session
/explorer
/inspect
/find
/history
/diff
/projection
/changes
/operations
/status
/help
/new
```

are handled locally/shared-application-side and are never sent accidentally to the model.

Unknown slash commands should produce a local error/suggestion rather than silently becoming a model prompt.

Provide an explicit literal slash escape such as `//literal`.

Authoritative:

- task 134
- task 135

---

# 26. Product Surfaces Share Semantics

TUI, web, CLI, server, Rust clients, and future third-party applications consume the same application/session/change/model contracts.

Do not implement in a UI:

```text
session ID allocation
context compilation
provider execution
plan scheduling
canonical branch mutation
semantic migrations
provider continuation reuse/reseed
secret resolution
```

Those belong in shared lower layers.

Presentation state may be client-local; semantic state may not be duplicated for convenience.

Authoritative:

- `level-1/011. runtime libraries protocol and product surfaces.md`
- `level-2/024. agent operations protocol and cross surface product contract.md`
- tasks 133–136

---

# 27. Restart Must Reconstruct Semantics Without Provider Stream Replay

After restart, Scriberig reconstructs:

```text
project/snapshot state
session/branch tree
plans/dependencies
BranchResults
settled activity/model invocation history
candidate/validation state
presentation-independent observable records
```

Provider continuation may be reused when still valid or replaced when unavailable/stale.

The original provider token stream is not canonical history.

Authoritative:

- `level-1/009. agent interaction and sessions.md`
- `level-1/014. agent native semantic self hosting.md`
- `level-1/020. provider continuation state and observable model execution.md`

---

# 28. Security Boundaries

Never place resolved secrets into:

```text
session records
model request detail
provider continuation records
protocol DTOs
TUI/web state
logs/test snapshots
model profiles
```

Credentials remain opaque references resolved only at the provider/runtime boundary.

Full prompt means full **safe** prompt. Redacted fields should remain visibly marked where structure matters.

Hidden/private chain-of-thought is not a trace feature.

Authoritative:

- task 131 secure secrets management
- task 133
- `level-1/020. provider continuation state and observable model execution.md`
- task 136

---

# 29. Conversation-Drift Guardrails

When discussing or implementing Scriberig, stop and re-check design if reasoning begins to assume any of these conventional-agent shortcuts:

```text
"the repository is just files"
"edit means write_file/patch text"
"bash is the primary capability"
"session means flat transcript"
"only the user creates branches"
"child branch inherits full parent transcript"
"parent receives full child transcript"
"plan means prose checklist"
"branch tree and dependency graph are the same"
"one model/provider must own the session"
"provider conversation ID is the session ID"
"full prompt means raw HTTP request"
"thinking visibility is the same setting as prompt visibility"
"thinking means hidden chain-of-thought"
"new activity always appends at bottom"
"terminal row is reply identity"
"UI can implement missing semantics locally"
```

If one of these assumptions appears, read the relevant authoritative documents before proceeding.

---

# 30. Design Precedence and Synchronization

Higher-level architecture constrains lower-level implementation.

Use this interpretation order:

```text
Level 1 architectural invariant
    -> Level 2 concrete subsystem design
        -> Level 3 implementation task
            -> code
```

A lower-level document may refine a higher-level concept but must not silently contradict it.

For the current agent work:

- Level 1 `017` controls recursive decomposition/context isolation.
- Level 1 `018` controls durable plans/dependencies/multi-model execution.
- Level 1 `019` controls semantic-first agent editing.
- Level 1 `020` controls provider continuation/full prompt/observable reasoning.
- Level 2 `023` controls baseline runtime orchestration.
- Level 2 `024` controls cross-surface agent protocol.
- Level 2 `025` refines provider continuation/model observability.
- Level 2 `032` supersedes the prior local commodity-ownership target: Scriberig
  retains semantic session, branch, plan, context, and change semantics, while
  upstream `rpi` owns generic provider/model/agent/plugin/package/TUI machinery.
  Task 144 records the exact ownership and deletion boundary.
- Task 133 controls the shared agent console/model/thinking/trace baseline.
- Task 134 controls recursive TUI product behavior.
- Task 135 controls the main recursive semantic plan/tree implementation.
- Task 136 is the mandatory refinement for provider continuation, complete prompt inspection, and provider-visible reasoning.

Where older wording is less precise than a later explicit refinement, preserve the higher-level invariant and use the newer dedicated design for that concept.

---

# 31. Rich Project Artifacts

Spreadsheets, rich documents, CAD drawings, and geospatial datasets are
semantic domains over the shared project, change, validation, and projection
contracts. Preserve exact source evidence and opaque content, support scalable
collections, and materialize compound artifacts atomically. User interfaces
consume shared domain capabilities rather than implementing format semantics.

Authoritative:

- `level-1/025. compound artifacts scalable collections and rich domain editing.md`
- `level-2/035. rich project artifacts office cad and geospatial implementation plan.md`
- tasks 155–162

# 32. Recommended 20-File ChatGPT Project Attachment Set

Because the Project has a 20-file limit, prefer a stable architecture spine plus current implementation tasks rather than attaching all design files.

Recommended current set:

```text
01  AGENTS.md
02  design/CONTEXT.md
03  design/level-1/001. vision and principles.md
04  design/level-1/002. universal knowledge model.md
05  design/level-1/003. identity revisions and snapshots.md
06  design/level-1/005. changes migrations branches and merge.md
07  design/level-1/006. projections and bidirectional editing.md
08  design/level-1/007. domain modeling and adapters.md
09  design/level-1/009. agent interaction and sessions.md
10  design/level-1/010. universal query and operation model.md
11  design/level-1/014. agent native semantic self hosting.md
12  design/level-1/016. model driven agent execution and branch isolation.md
13  design/level-1/017. recursive agent decomposition and context isolation.md
14  design/level-1/018. durable plans dependency scheduling and multi-model execution.md
15  design/level-1/019. semantic agent operations and projection-first editing.md
16  design/level-1/020. provider continuation state and observable model execution.md
17  design/level-2/023. agent runtime model provider and branch orchestration.md
18  design/level-2/025. provider continuation and observable model execution.md
19  design/level-3/135. implement recursive semantic agent plans dependencies and tree tui.md
20  design/level-3/136. implement provider continuation full prompt and observable reasoning.md
```

When TUI implementation itself is the main topic, temporarily swap one less-central attachment (for example `001` or `014`) for:

```text
design/level-3/134. make agent conversation the default tui and move auxiliary views behind slash commands.md
```

When protocol/server work is the main topic, temporarily swap one less-central attachment for:

```text
design/level-2/024. agent operations protocol and cross surface product contract.md
```

Do not spend permanent attachment slots on old conversation exports after their durable decisions are represented in design documents.

---

# 33. Active Implementation Reading Rule

`CONTEXT.md` is an orientation map, not sufficient implementation instruction.

Before changing a subsystem, an implementing agent must still read:

```text
root + nearest AGENTS.md
relevant Level-1 design
relevant Level-2 design
active Level-3 task
current code owner
```

The purpose of this file is to prevent architectural amnesia, not to justify skipping detailed design.
