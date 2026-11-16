# Course authoring

A course is one YAML file in `courses/`. Drop it in, run `pnpm courses:check`, restart the
API (or let `pnpm dev` reload it) — that's all. Editors with the YAML language server get
autocompletion from `courses/schema/course.schema.json` via the first-line comment.

```yaml
# yaml-language-server: $schema=./schema/course.schema.json
id: docker-basics            # stable, never change after publishing
version: 1.0.0               # bump on every content change
title: Containers in 30 minutes
summary: One sentence for the catalog.
description: |
  Markdown shown on the course page.
category: Infrastructure
difficulty: beginner         # beginner | intermediate | advanced
order: 20                    # position within the difficulty level in the catalog
estimated_minutes: 30
help: |                      # coaching notes added to every step's help prompt
  The learner writes their first Dockerfile. Guide, don't write it for them.
objectives: [Explain images vs containers]
prerequisites:
  - title: Connect your AI agent over MCP
    course: mcp-first-connection
extensions: [quiz, filesystem]   # optional; validated against actual usage
sections:
  - id: basics
    title: Basics
    steps:
      - id: intro
        type: explanation
        title: What is a container?
        body: |
          Markdown…
```

## Steps

Common fields: `id`, `type`, `title`, `description`, `body` (markdown), `config` (visible to
learners), `validation` (private), `depends_on`, `optional`, `completion: manual`,
`estimated_minutes`, `extension` (only for generic types), `help` (coaching notes).

- **Catalog order**: courses are listed by difficulty, then `order`, then title.
- **Help**: every step has a *Help* button with a prompt learners paste into their agent. It
  asks the agent to coach without solving and appends the step's `help` and the course's
  `help`. Write them as guidance for a tutor: likely mistakes, what to inspect, what not to
  give away. They are visible to learners and agents, so never put answers in them.

- **Dependencies**: omitted → the previous step; `[]` → available immediately; a list →
  explicit DAG. Cycles are rejected.
- **Completion**: optional steps don't block course completion. `completion.required_steps`
  at course level overrides the default (all non-optional steps).

| Type | `config` | `validation` |
| --- | --- | --- |
| `explanation` | — (needs `body`) | — |
| `interactive_content` | `panels: [{title, body}]` | — |
| `video` | `url` (https), `duration_seconds?`, `transcript?` | — |
| `code_example` | `snippets: [{language, code, filename?, caption?}]` | — |
| `quiz` | `questions: [{id, prompt, multiple?, explanation?, options: [{id, text, correct?, explanation?}]}]`, `pass_score?` | — |
| `multiple_choice` | `prompt`, `options`, `multiple?`, `explanation?` | — |
| `free_text` | `prompt`, `placeholder?` | `min_words?`, `max_words?`, `required_concepts: [{label, any_of}]` |
| `interactive_exercise` | `prompt`, `template` with `{{blank}}`, `blanks: [{id, hint?}]` | `answers: {blank: [accepted…]}`, `case_sensitive?` |
| `mcp_connection_check` | `server_name?` | `required_tool?` |
| `http_endpoint_check` (run by the browser) | `path?`, `method?` (GET/HEAD) | `expect_status?`, `expect_json: {pointer: value}`, `body_contains?`, `max_latency_ms?` |
| `filesystem_validation` | — | `files: [{path (glob), label?, min_length?, max_length?, contains?, frontmatter?: {required, max_length}}]` |
| `configuration_validation` | `file`, `format` (json/yaml) | `rules: [{pointer (with * wildcards), equals?, contains?, one_of?, absent?, label?}]` (`absent: true` passes only if nothing matches) |
| `git_repository_check` | — | `branch?`, `min_commits?`, `commit_prefixes?`, `required_files?`, `require_clean?`, `require_remote?` |
| `extension_validation` | depends on `extension` | depends on `extension` |
| `external_task` | `instructions?` | needs `extension` or `completion: manual` |
| `conditional` | `when: {step, status \| score_below \| score_at_least}` | — |
| `prerequisite` | `courses: [ids]` | — |
| `manual_completion` / `automatic_completion` | `instructions?` | — |
| `reusable_section` | `section: <shared_sections key>` | — |
| `service_check` | `default_url?` | scenario of hidden HTTP test cases (below) |
| `mcp_server_check` | `path?` (default `/mcp`) | `tools: [{name, required_params, min_description_length, json}]`, `calls: [{id, label, tool, arguments, expect_error?, text_contains_any?, json?, graded?}]` |

Answer keys (`correct`, explanations) are stripped before configs reach learners or agents.

## Service checks

The strongest proof is behaviour: the learner enters the base URL of a service they built, and
their **browser** sends it the test requests and submits the responses. `localhost` works and
nothing has to be exposed. Requests are visible to learners (and agents); expectations in
`expect` are private and evaluated on the server. Agents cannot submit these steps.

```yaml
- id: search-api
  type: service_check
  title: Run the search checks
  validation:
    timeout_ms: 15000        # per request in the browser, max 120000
    parallel: false          # true lets the browser run 4 cases at once (only without ordering)
    min_pass_ratio: 0.75     # share of `graded` cases that must pass; others must all pass
    cases:
      - id: money
        label: “money” ranks finance documents highest
        graded: true         # tolerates model variance
        request: { method: GET, path: /search, query: { q: money } }   # or json: {...}
        expect:
          status: 200        # or [200, 201]; default any 2xx
          json:
            - { pointer: /results, top_k: 1, field: /category, includes: finance }
```

`json` rules take a `pointer` (`*` matches every item), optionally `field` (pointer applied to
each list item) and `top_k`, and any of:

- lists: `min_items`, `max_items`, `includes`, `includes_all`, `excludes`, `all_equal`,
  `in_order`, `sorted: asc|desc`
- values: `equals`, `one_of`, `contains`, `contains_any`, `contains_all`, `not_contains_any`,
  `min_length`, `type`, `approx` + `tolerance`, `min`, `max`
- `absent: true` (missing or `null`)

Strings compare case-insensitively; numeric strings such as `"€1,250.50"` count as numbers for
`approx`. `{{nonce}}` in requests and expectations becomes a fresh random value per run
(e.g. `probe-{{nonce}}` ids). `"{{asset:receipt.png}}"` as a JSON value sends
`courses/assets/receipt.png` as a data URL (max 2 MiB, checked at load time, served publicly
from `/api/v1/course-assets/`).

Design cases so that only a working implementation passes: fictional data a model cannot
know, paraphrases without shared keywords, unanswerable questions and prompt injections.
Learner services must allow CORS from the platform origin; the step page explains how.

## Reusable sections

```yaml
shared_sections:
  agent-connection:
    steps:
      - { id: reconnect, type: mcp_connection_check, title: Confirm your agent is connected }
sections:
  - id: build
    title: Build
    steps:
      - { id: connection, type: reusable_section, title: Agent connection, config: { section: agent-connection } }
```

Inlined steps get prefixed ids (`connection.reconnect`), so a shared section can be used
more than once.

## Versioning rules

Published versions are immutable. Change content → bump `version`. Learners on an older
version keep it until they start again. During local authoring, `MUTABLE_COURSE_VERSIONS=true`
(the `.env.example` default) lets you edit a version in place.

## Course studio and agent authoring

Super admins can also create courses in the web app (**Course studio**, `/admin/courses`) or let
their agent do it over MCP. Grant the role with `learning-platform grant-super-admin <email>` or by
listing existing accounts in `SUPER_ADMIN_EMAILS` (applied at startup).

- Studio courses use the same YAML format and are stored in the database (`source: authored`).
  Courses from files stay read-only in the studio; a file with the same `id` takes the course over.
- An unused version can be overwritten; once a learner starts it, publishing requires a version bump.
- A super admin's MCP token additionally exposes `authoring_get_guide`, `authoring_list_courses`,
  `authoring_get_course`, `authoring_validate_course` and `authoring_publish_course`.

## When you need code

Only for a genuinely new capability (e.g. validating a running Docker container): implement
`lp_core::Extension` in `crates/extensions`, declare its step types in the manifest, and add
it to `builtin_extensions()`. Courses can use the new step type immediately.
