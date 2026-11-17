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
order: 20                    # position in the course track
estimated_minutes: 30
help: |                      # coaching notes added to every step's help prompt
  The learner writes their first Dockerfile. Guide, don't write it for them.
objectives: [Explain images vs containers]
glossary:                    # key terms for the learner's sidebar; terms stay English
  - { term: Image, definition: A read-only template a container starts from. }
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

- **Catalog order**: courses form one track, listed by `order` (then difficulty and title). A
  course gates its practical section with a `prerequisite` step on the previous course.
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
| `diagram` | `nodes: [{id, label, detail?, row?, col?, kind?}]`, `edges: [{from, to, label?, dashed?}]`, `caption?` | — |
| `diagram_exercise` | `prompt`, `mode: slots\|order`, slots: `nodes` (with `slot: true`) + `edges`; both: `chips: [{id, label}]` | `slots: {node: chip}` or `order: [chip…]` |
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
| `project_setup` | — (learner picks a theme and stack; completes once chosen) | — |
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

## Themes: one course, every learner's own website

Learners build one website throughout the track. Write course text once and refer to the
learner's theme with `{{theme.<path>}}` placeholders, anywhere in text, `config` or
`validation`:

| Placeholder | Comes from `courses/themes/<id>.yaml` |
| --- | --- |
| `{{theme.name}}`, `{{theme.item}}`, `{{theme.items}}`, `{{theme.site_kind}}`, … | `name` and localized `vars` |
| `{{theme.data.site.name}}`, `{{theme.data.items.0.id}}` | `data`: the public seed data learners load (via MCP `get_project`) |
| `{{theme.probes.search.dutch.query}}` | `probes`: hidden test inputs and expectations that fit the data |

A string that is *only* a placeholder keeps the value's type, so
`includes_all: "{{theme.probes.catalog.category_ids}}"` becomes a list and
`min_items: "{{theme.probes.catalog.count}}"` a number.

Only add probes where a check depends on data the learner stores (catalog, search, RAG); checks
whose input travels in the request stay generic. The non-selectable `neutral` theme defines the
wording used before a project is chosen **and the shape of `data` and `probes`**: every theme
must define the same keys with the same kinds of values. `pnpm courses:check` renders every
course for every theme and language and fails on undefined placeholders or invalid configs.
Steps containing placeholders are validated on those rendered copies.

## Translations

Courses are authored in English. A Dutch overlay lives in `courses/locales/nl/<course id>.yaml`
and mirrors the course by section and step id. It may only replace **text**: strings replace
strings, objects merge by key, arrays of objects match by `id` (or by position, so use `{}` for
items you leave unchanged), and lists of plain strings such as `any_of` keyword lists are
replaced as a whole — include the English keywords too. Glossary entries are translated by
term (`glossary: { Embedding: … }`); terms themselves stay English. Keep developer terms such
as embedding, prompt and endpoint in English in the Dutch text, and quote flow-style values
that contain commas or `?`. Untranslated fields fall back to English.

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
