---
name: validate-structured-artifacts
description: Validate YAML, Markdown structure and agent/prompt frontmatter before sync or release gates.
user-invocable: true
disable-model-invocation: false
---


# validate-structured-artifacts

Validate all structured artifacts in the workspace before passing a release gate. This includes YAML files, agent frontmatter, prompt frontmatter, and canonical Markdown structure.

## When to use

Use this skill when:

- Before a release gate to verify structural integrity
- After bulk edits to agents, prompts, or canonical docs
- As part of the `validation-pack` prompt
- When troubleshooting sync failures

## Validation checks

### 1. YAML syntax validation

**Files**: `docs/project/backlog.yaml`, `docs/project/refined-stories.yaml`, `docs/project/quality-gates.yaml`

Backlog, refined stories, and quality gates are schema-backed by
`schemas/backlog.schema.yaml`, `schemas/refined-stories.schema.yaml`, and
`schemas/quality-gates.schema.yaml`.

| Check | Rule |
| --------------- | ---------------------------------------------- |
| Parse | File must parse as valid YAML without errors |
| Encoding | Must be UTF-8 |
| Structure | Root key matches expected format (list or map) |
| Required fields | Each entry has all required fields per schema |

**backlog.yaml required fields per story**:

- `id`, `title`, `status`, `priority`, `epic_id`, `acceptance_ref`

**refined-stories.yaml required fields per story**:

- `id`, `title`, `status`, `priority`, `owner_role`, `acceptance_ref`
- If `status` is not `draft`: `implementation_map` must exist

**quality-gates.yaml required fields per gate**:

- `id`, `name`, `status`, `owner_roles`

### 2. Agent frontmatter validation

**Files**: `.github/agents/*.agent.md`

| Check | Rule |
| ------------------ | ---------------------------------------------------------------- |
| YAML block | File starts with `---` delimiters enclosing valid YAML |
| `description` | Required, non-empty string |
| `tools` | Required, must be a list of known tool aliases |
| `agents` | If present, must be a list of valid agent names |
| `handoffs` | If present, entries must reference valid agent names |
| Known properties | No unknown/misspelled properties |
| Valid tool aliases | `agent`, `search`, `read`, `edit/editFiles`, `execute`, `web`, `todo` |

**Body structure checks**:

| Section | Required |
| ------------------------ | --------------------- |
| `## Personality` | Yes (after bootstrap) |
| `## Behavior contract` | Yes |
| `## Decision heuristics` | Yes |
| `## Anti-patterns` | Yes |
| `## Tone` | Yes |
| `## Memory interaction` | Yes |

### 3. Prompt frontmatter validation

**Files**: `.github/prompts/*.prompt.md`

| Check | Rule |
| ---------------- | ------------------------------------------------------ |
| YAML block | File starts with `---` delimiters enclosing valid YAML |
| `description` | Required, non-empty string |
| `agent` | If present, must reference a valid agent name |
| `tools` | If present, must be a list of known tool aliases |
| Known properties | No unknown/misspelled properties |

### 4. Canonical Markdown structure

**Files**: `docs/project/*.md`

| Check | Rule |
| ----------- | ----------------------------------------- |
| Non-empty | File must not be empty or whitespace-only |
| Has heading | Must have at least one `#` heading |
| UTF-8 | Must be valid UTF-8 |

**Markdown style checks**:

| Check | Rule |
| --- | --- |
| Line length | Prefer lines at or below 80 chars (`MD013`) |
| Heading spacing | Headings surrounded by blank lines (`MD022`) |
| Heading punctuation | No trailing punctuation in headings (`MD026`) |
| Ordered lists | Sequential numbering (`MD029`) |
| Fence spacing | Blank lines around fences (`MD031`) |
| Fence language | Every fence declares a language (`MD040`) |
| Table style | Consistent pipe spacing in each file (`MD060`) |

### 5. Cross-reference checks

| Check | Rule |
| ---------------- | -------------------------------------------------------------------- |
| Story references | Stories in `refined-stories.yaml` must exist in `backlog.yaml` |
| Epic references | `epic_id` in stories must match an epic in `backlog.yaml` |
| Acceptance refs | `acceptance_ref` must point to a section in `acceptance-criteria.md` |
| Agent references | `agents` property entries must match actual `.agent.md` filenames |
| Gate owner roles | `owner_roles` must be valid agent role names |

## Process

### Step 1 -- Discover files

Scan `.github/agents/`, `.github/prompts/`, `docs/project/` for all structured files.

### Step 2 -- Run checks

Execute each validation check. Collect results as:

```text
{ file, check, result: 'pass'|'fail'|'warn', detail? }
```

If `markdownlint` is available, run it against `docs/project/**/*.md`,
`.github/**/*.md`, and `.agents/**/*.md` and include its findings.

### Step 3 -- Report

Print a summary:

```text
Validation Report
=================
Files checked:  {n}
Checks passed:  {n}
Warnings:       {n}
Failures:       {n}

[FAIL] .github/agents/foo.agent.md -- missing required section: ## Personality
[WARN] docs/project/backlog.yaml -- story ST-003 missing acceptance_ref
[PASS] .github/prompts/bootstrap-from-prd.prompt.md -- all checks passed
```

### Step 4 -- Gate result

If any check has `result = 'fail'`:

- Return overall result: **FAIL**
- Create a finding if running in an agent context

If only warnings:

- Return overall result: **WARN**

If all pass:

- Return overall result: **PASS**

## Error handling

| Error | Action |
| ---------------------------- | ---------------------------------------------------- |
| File not found | Report as warning (optional files may not exist yet) |
| YAML parse failure | Report as failure with line/column if available |
| Encoding error | Report as failure |
| Unknown frontmatter property | Report as warning |
