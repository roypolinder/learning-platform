# Learning Platform

An interactive platform for learning practical technical skills — MCP, AI agents, agent
skills, APIs — by building real things with your own editor and AI coding agent. The
platform verifies the work itself: agents connect over MCP, submit evidence, and steps
complete automatically.

Courses are **declarative YAML files**. The course engine and a set of reusable extensions
do the rest; adding a course needs no backend, frontend or database changes.

## Courses

Listed by difficulty as a learning path. Every practical step is verified by the platform, and
every step has a **Help** button with a coaching prompt for the learner's own agent.

| Level | Courses | Proof |
| --- | --- | --- |
| Beginner | MCP connection, tool calling, agent skills, semantic matching | agent events, submitted config, hidden tool calls and paraphrase queries |
| Intermediate | vector databases, multimodal AI, RAG, advanced skills, build an MCP server | hidden search queries and live upserts, unseen receipt images, grounded Q&A with injection and unanswerable questions, exact script output, MCP client session |
| Advanced | AI agents, knowledge graphs, agentic workflows | multi-step tasks with traces and refusals, entity resolution and multi-hop queries, ticket pipeline with routing and guardrails |

Learners build services in any language with any model provider. Their **browser** sends the
hidden test requests to the service (so `localhost` works and nothing is exposed) and the API
validates the responses (see [course authoring](docs/course-authoring.md#service-checks)).

## Stack

| Layer | Technology |
| --- | --- |
| Backend | Rust, Axum, Tokio, SQLx, Tracing, Tower HTTP |
| MCP | Official Rust SDK (`rmcp`), Streamable HTTP transport |
| Database | PostgreSQL (native in WSL for development) |
| Frontend | SvelteKit 3, Svelte 5 (runes), TypeScript (strict), Tailwind CSS 4 |
| Tooling | Cargo, rustfmt, Clippy, pnpm, svelte-check |

## Quick start

Prerequisites: Rust (rustup), Node.js ≥ 22.17 via nvm, pnpm, PostgreSQL.

```bash
pnpm install                 # frontend + root tooling
pnpm db:setup                # only if PostgreSQL isn't set up yet (see below)
pnpm dev                     # PostgreSQL check + API (:8080) + web (:5173)
```

Open <http://localhost:5173>, create an account, and start **Connect your AI agent over MCP**.

Run pieces independently:

```bash
pnpm dev:api                 # Rust API with auto-reload (cargo-watch if installed)
pnpm dev:web                 # SvelteKit dev server, proxies /api to :8080
pnpm db:start | db:stop | db:status
pnpm db:migrate              # migrations also run automatically on API start
pnpm courses:check           # validate course files without a database
```

### PostgreSQL

Any PostgreSQL ≥ 14 works; the app only needs `DATABASE_URL` (see `.env.example`).

- **System install** (`sudo apt install postgresql`): create a `learning_platform` role
  with `CREATEDB` (integration tests create throwaway databases) and a
  `learning_platform` database, then copy `.env.example` to `.env`.
- **No root access**: `pnpm db:setup` extracts Ubuntu's own `postgresql-14` packages into
  `~/.local/postgresql`, initializes a cluster in `~/.local/share/postgresql/14/data`
  (localhost only, SCRAM auth), creates the role/databases with random passwords and writes
  `.env`. `pnpm db:start` starts it after a reboot (`pnpm dev` does this for you).

MariaDB, if installed, is not used or touched.

## Connecting an agent (VS Code)

The MCP endpoint is an OAuth 2.1 protected resource, so agents sign in through the browser
instead of handling tokens. This works in VS Code's Copilot harness, which cannot forward
`${input:…}` secrets.

1. Add the server to `.vscode/mcp.json` — no token, header or input prompt needed:

   ```json
   {
     "servers": {
       "learning-platform": { "type": "http", "url": "http://localhost:8080/mcp" }
     }
   }
   ```
2. When VS Code shows the server needs authentication, choose **Authenticate**. The browser
   opens the platform's consent page (sign in first if needed); select **Allow**.
3. Ask the agent *"What is my current step on the learning platform?"*.

Connections appear under **Settings → Connected tools**, where they can be revoked. Scripts
and other clients without OAuth support can still use a personal token from
`POST /api/v1/integrations/mcp`, sent as `Authorization: Bearer lp_mcp_…`.

Tools: `get_course_context`, `get_current_step`, `report_task_result`, `get_session_status`.

## Testing and quality

```bash
pnpm test        # cargo test --workspace (needs PostgreSQL) + svelte-check
pnpm lint        # rustfmt --check, clippy -D warnings, svelte-check
```

The Rust suite covers the state machine, course validation, every extension (including a
live HTTP check), progression scenarios, and API/MCP integration tests that run against
throwaway PostgreSQL databases with a real `rmcp` client.

## Production build

```bash
pnpm build       # static web build + release API binary
pnpm start       # API serves the SPA from apps/web/build on :8080
```

Or `docker compose --profile app up --build` (requires Docker; see `docker-compose.yml`).
Set `COOKIE_SECURE=true`, `MUTABLE_COURSE_VERSIONS=false`, `PUBLIC_BASE_URL`,
`MCP_ALLOWED_HOSTS` and (behind a reverse proxy) `TRUST_PROXY_HEADERS=true` for any
non-local deployment.

## Production server

On a fresh Debian 12+ or Ubuntu 22.04+ server, clone the repository and run:

```bash
sudo ./setup.sh            # first install: asks for a domain and admin email
git pull && sudo ./setup.sh   # update: reuses the answers, deploys a new release
```

It installs PostgreSQL, Node.js, pnpm, Rust and Caddy (automatic HTTPS), creates the
database, service user and `/etc/learning-platform/learning-platform.env` (shown for review
before use), builds a versioned release in `/opt/learning-platform/releases`, runs migrations,
starts a hardened systemd service and rolls back automatically if the new release isn't
healthy. Optionally it configures ufw, daily `pg_dump` backups and unattended security
updates. Re-runs keep existing config and secrets. `sudo learning-platform <command>` runs
admin commands (e.g. `grant-super-admin`) with the production config. See
`./setup.sh --help` for non-interactive use.

Traffic statistics: Caddy's access log is rotated daily and deleted after 30 days. Every 15
minutes GoAccess turns it into an HTML report (IPs shortened, query strings removed, crawlers
left out) that only super admins can open at `/admin/traffic`; the API serves it from
`TRAFFIC_REPORT_PATH`. The `/privacy` page describes this and must stay in sync with the log
retention (`ACCESS_LOG_RETENTION_DAYS` in `setup.sh`).

The logo in the app is `apps/web/static/favicon.svg`; replace that file to rebrand.

## Project layout

```
apps/api/            Axum API, MCP gateway, persistence (domains/*, mcp/, migrations/)
apps/web/            SvelteKit SPA (lib/{components,features,services,state,types,utils})
crates/core/         Stable contracts: extension trait, statuses, outcomes
crates/course-engine/  Course model, compiler/validator, progression state machine
crates/extensions/   quiz, http, filesystem, git, mcp
courses/             Course definitions + schema/course.schema.json
docs/                Architecture and course authoring guides
infrastructure/docker/  Container image
scripts/             Database and dev helpers
```

Read next: [docs/architecture.md](docs/architecture.md) ·
[docs/course-authoring.md](docs/course-authoring.md)
