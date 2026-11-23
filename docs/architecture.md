# Architecture

A modular monolith: one Rust binary (API + MCP gateway + static web), one PostgreSQL
database. Crate boundaries enforce the important separations.

```
lp-core  ◄──  lp-course-engine
   ▲                ▲
   └── lp-extensions │
          ▲          │
          └── lp-api ┘
```

- **`lp-core`** — the stable contract: `Extension` trait, `ExtensionManifest`,
  `StepTypeSpec`, `ValidationOutcome`, `StepStatus` / `SessionStatus` state machines.
- **`lp-course-engine`** — knows step *categories* and *completion strategies*, never what
  an integration does. Parses, validates and compiles course files; drives progression.
- **`lp-extensions`** — integrations implementing the contract. They depend only on
  `lp-core`, so neither engine nor extensions know about each other's internals.
- **`lp-api`** — HTTP, persistence, auth, MCP. Organized by domain
  (`auth`, `account`, `users`, `courses`, `sessions`, `progress`, `extensions`, `integrations`), each with
  `domain` / `dto` / `repository` / `service` / `handlers` as needed. Handlers are thin;
  services own orchestration; repositories own SQL.

## Course engine

**Compilation.** YAML → `CourseDefinition` (strict serde, unknown fields rejected) →
`Course`. The compiler inlines `reusable_section`s (ids prefixed), resolves default
dependencies (previous step), resolves each step's extension from the step-type registry,
runs built-in and extension config validation, detects dependency cycles, and checks
declared extensions. All problems are reported at once with paths.

**Rendering per learner.** A compiled course is a template. `CourseEngine::render` applies the
learner's language overlay and fills in `{{theme.…}}` placeholders from their project theme,
then compiles the result; step ids and structure never change, so progress applies to every
rendering. The API caches renderings per (version, locale, theme) and uses them for views,
MCP and validation, so hidden checks run against the learner's own theme probes. A learner's
profile (`users.locale`, `learner_projects`) also feeds `EngineContext.has_project`, which
completes `project_setup` steps.

**Step types** come from a registry built from engine built-ins plus every extension's
manifest. Each type has a category (`content`, `assessment`, `practical`, `workflow`) and a
completion strategy:

| Strategy | Completes when | Examples |
| --- | --- | --- |
| `acknowledge` | the learner confirms | explanation, code_example, manual_completion |
| `submission` | an extension validates submitted evidence | quiz, http_endpoint_check, filesystem_validation |
| `event` | an extension accepts a platform event | mcp_connection_check |
| `automatic` | it unlocks | automatic_completion |
| `derived` | engine state says so | conditional, prerequisite |

**State machine.** Steps: `locked → available → in_progress → completed | failed`,
`failed → in_progress | completed`, `locked|available|in_progress → skipped`. Completed and
skipped are final. Sessions: `active → completed | abandoned`, `abandoned → active`.
Transitions are validated in `lp-core` and every transition is recorded in
`session_events`.

**Progression** (`settle`) repeatedly unlocks steps whose dependencies are resolved,
evaluates conditionals, completes automatic steps and satisfied prerequisites, until a
fixpoint. A course completes when all required steps are resolved.

**Versioning.** Course versions are immutable rows in `course_versions` (definition stored
as JSONB). Sessions pin a version and are always replayed against it, so publishing 1.1.0
never changes an in-flight 1.0.0 session. `reconcile` adds state for steps a newer
definition introduces, which is the hook for future opt-in upgrades.

## Validation flow and concurrency

Submissions are evaluated *outside* the session lock (extensions may make network calls),
then applied inside a transaction holding `SELECT … FOR UPDATE` on the session row, against
freshly loaded state. Browser actions and agent actions therefore never interleave, and a
step that changed meanwhile is rejected with `409 step_not_actionable`.

## Extensions

| Id | Step types | Evidence |
| --- | --- | --- |
| `quiz` | quiz, multiple_choice, free_text, interactive_exercise | answers; keys are redacted from public configs |
| `http` | http_endpoint_check | response of one request made by the learner's browser |
| `filesystem` | filesystem_validation, configuration_validation (+ generic types) | files sent by the agent; glob paths, frontmatter, JSON pointers |
| `git` | git_repository_check | repository state reported by the agent |
| `mcp` | mcp_connection_check | `mcp.tool_called` events from the gateway |
| `service` | service_check, mcp_server_check | responses to hidden HTTP cases, or an MCP session, run by the learner's browser |

`config` is learner-visible (after `public_config` redaction); `validation` is private and
never leaves the server. Adding an extension = implement `Extension`, add it to
`builtin_extensions()`. No engine, API or frontend change is needed — the frontend falls
back to a generic renderer based on completion strategy.

Extensions are compiled in. The trait is async and takes only serializable inputs, so an
out-of-process or sandboxed implementation can sit behind the same interface later.

## MCP gateway

`rmcp`'s Streamable HTTP service mounted at `/mcp`, behind its own auth middleware:

- Bearer tokens (`lp_mcp_…`, 256-bit random, SHA-256 stored, revocable), obtained through
  OAuth (below) or created manually (90-day expiry).
- A 401 carries `WWW-Authenticate: Bearer resource_metadata=…`, which starts OAuth discovery.
- Every tool resolves sessions **for the token's user only**; another user's session id
  returns the same error as a non-existent one.
- Host allow-list (DNS rebinding) and rejection of browser `Origin`s.
- Per-connection rate limiting; 2 MiB body limit.
- Each tool call is dispatched to the engine as an `mcp.tool_called` event before the tool
  runs, so `get_current_step` already reflects a just-completed connection step.
- No filesystem access or command execution: agents send evidence, the platform validates.

## OAuth for MCP clients

`domains::oauth` is a small OAuth 2.1 authorization server following the MCP authorization
spec, so clients such as Claude Code, VS Code's Copilot harness, OpenCode, Cursor and Codex can
sign in without pasted secrets:

- Discovery: `/.well-known/oauth-protected-resource[/mcp]` (RFC 9728) and
  `/.well-known/oauth-authorization-server` (RFC 8414). Issuer and resource are derived from
  the request `Host` when it is on `MCP_ALLOWED_HOSTS`, so `localhost` and `127.0.0.1` both
  validate; otherwise `PUBLIC_BASE_URL` is used.
- `POST /oauth/register`: dynamic client registration (RFC 7591), public clients only.
  Redirect URIs must be https, loopback http (any port at runtime, RFC 8252) or a
  private-use scheme. Rate limited per IP.
- `GET /oauth/authorize`: validates the client and redirect URI (errors are shown, never
  redirected), then hands off to the web app's `/connect/authorize` consent page
  (`WEB_BASE_URL`). The page uses the normal cookie session and the CSRF-protected
  `/api/v1/oauth/authorization` endpoints.
- `POST /oauth/token`: authorization code with mandatory PKCE S256 (codes are single-use,
  5 minutes; a replayed code revokes the grant), and refresh tokens that rotate on use.
  Access tokens live 1 hour, grants 90 days from the last refresh.
- Each grant is an `integration_connections` row (`auth_method = 'oauth'`), so it shows up
  in Settings and is revoked like a manual token. Re-authorizing a client replaces its
  previous grant. Only hashes of codes and tokens are stored.

## Realtime

Mutations publish to an in-process broadcast hub; `GET /api/v1/sessions/{id}/stream` (SSE)
relays updates for the caller's own session. Clients refetch progress on each event and on
reconnect. For multiple instances, swap the hub for PostgreSQL `LISTEN/NOTIFY`.

## Checks against learner services

Steps that test something the learner runs (`http_endpoint_check`, `service_check`,
`mcp_server_check`) are executed by the learner's **browser**, never by the server:

1. The extension publishes the requests as `client_task` on the step view; expectations stay
   in `validation`.
2. The web app sends them to the learner's service (`localhost` works, nothing is exposed),
   substitutes a random `{{nonce}}`, inlines fixtures from `GET /api/v1/course-assets/{name}`
   and submits the raw responses (bodies capped at 128 KiB).
3. The extension evaluates the responses. Agent submissions are rejected for these steps.

The server makes no outbound requests, so there is no SSRF surface and validation is a few
milliseconds of JSON checks regardless of how slow a learner's model is. The trade-off is
trust: a determined learner could forge responses, which is acceptable for a learning platform.
Learner services must allow CORS from the platform origin; the step page shows the headers.

## Security notes

- Passwords: Argon2id; constant-time-ish login to avoid account enumeration.
- Browser sessions: random token in an `HttpOnly; SameSite=Lax` cookie, SHA-256 stored.
- Account emails (`domains::account`, `mail`): single-use links whose SHA-256 is stored in
  `email_tokens` with the address they were sent to; the secret travels in the URL fragment so
  it never reaches access logs. Reset links live 1 hour, change links 24 hours, verification
  links 48 hours. A link stops working once the account's address changes. Mail goes out in the
  background through SMTP (`lettre`), rate limited per recipient; "forgot password" answers the
  same for unknown addresses.
- Sensitive account changes (email, password, deletion) need the current password again, rate
  limited per account. Deletion acts only on the signed-in user (no endpoint takes a user id),
  cascades to everything the user owns, and refuses to remove the last super admin. A password
  reset signs out every browser session; a password change every other one.
- CSRF: state-changing cookie requests must carry `x-requested-with`; no CORS is granted.
- Authorization: every session query is scoped by `user_id`; other users' sessions → 404.
- Learner-service checks run in the browser; the server only validates submitted responses
  (URL must be http/https without credentials; payloads are size-limited).
- Evidence payloads are size-limited and paths validated; raw evidence is not persisted.
- Course markdown escapes raw HTML; links are restricted to safe schemes.
- Untrusted code is never executed. Future sandbox workers must run outside this process
  with resource limits, timeouts and network/filesystem isolation.

## API

Versioned under `/api/v1`; errors are `{"error": {"code", "message", "details?"}}`.

```
POST /auth/register | /auth/login | /auth/logout      GET|PATCH /users/me
POST /auth/password/forgot | /auth/password/reset | /auth/email/confirm
POST /users/me/email | /users/me/email/verification | /users/me/password | /users/me/delete
DELETE /users/me/email/pending                         GET /admin/users?q&status&page (super admin)
GET  /courses?q&category&difficulty&page&per_page    GET /courses/{id}
POST /courses/{id}/sessions                            GET /sessions?status&page
GET  /sessions/{id} | /progress | /events | /stream    POST /sessions/{id}/refresh
POST /sessions/{id}/steps/{step}/open | /complete | /submissions
GET  /progress/overview
GET  /extensions | /extensions/step-types
GET  /integrations   POST /integrations/mcp   DELETE /integrations/{id}
GET  /api/health     (unversioned)             /mcp (MCP Streamable HTTP)
```
