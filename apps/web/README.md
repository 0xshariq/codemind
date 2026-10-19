# CodeMind Web

This workspace contains the Next.js application and the server-side Agent Runtime. The browser calls Next.js Route Handlers; only the server communicates with the Rust engine over gRPC. Rust owns canonical code, test, change, impact, and risk facts. FalkorDB is accessed through Rust, never directly from this application.

The repository root [README](../../README.md) describes the product, tracks, graph model, and full architecture. This file covers local web development.

## Local development

Use Node.js 22 or newer and the package manager version declared in `package.json` (`pnpm@11.21.0`). Install web dependencies from `apps/web/`, then compile the engine from the repository root:

```bash
pnpm --dir apps/web install --frozen-lockfile
cargo build --manifest-path engine/Cargo.toml
```

The application depends on three local processes:

1. Start FalkorDB with its persistent Compose volume:

   ```bash
   docker compose -f apps/web/docker-compose.yml up -d
   ```

2. Start the Rust service from the repository root:

   ```bash
   cargo run --manifest-path engine/Cargo.toml
   ```

3. In `apps/web/`, prepare local configuration and start Next.js:

   ```bash
   cp .env.example .env.local
   pnpm dev
   ```

Set a private random `CODEMIND_SESSION_SECRET` and, for model-backed investigations, an `OPENROUTER_API_KEY` in `.env.local`. Keep local credentials out of source control. The web app connects to the Rust gRPC endpoint; only the Rust service receives FalkorDB configuration.

For local development only, set `CODEMIND_DEV_AUTH_ENABLED=true` and configure the development user/workspace/scopes. After starting the app, a same-origin browser request to `POST /api/dev/session` sets an eight-hour HttpOnly session cookie. This endpoint is disabled by default and refuses production configuration.

Open <http://localhost:3000>. Stop the database with `docker compose -f apps/web/docker-compose.yml down`; its named volume is retained unless explicitly removed.

## Configuration

`apps/web/.env.example` lists the variables consumed by the current web runtime:

- `CODEMIND_ENGINE_ENDPOINT` configures the Rust gRPC connection. Investigation RPCs remain open until the engine responds; transport failures are reported by gRPC.
- `OPENROUTER_API_KEY` and `CODEMIND_LLM_MODEL` configure the server-side model provider.
- `CODEMIND_SESSION_SECRET` signs and verifies application sessions.
- `CODEMIND_DEV_AUTH_ENABLED`, `CODEMIND_DEV_USER_ID`, `CODEMIND_DEV_WORKSPACE_ID`, `CODEMIND_DEV_SCOPES` and `CODEMIND_DEV_TEAM_IDS` configure the opt-in local-only session bootstrap; it cannot be enabled in production.
- `CODEMIND_REPOSITORIES_JSON` is an optional trusted registration list for local and example repositories. Each entry must include a server-side `sourcePath` and a matching `workspaceId`; the Rust RepositoryService resolves the repository identity and snapshot.
- Public GitHub URLs are cloned with a shallow, non-interactive Git invocation into `CODEMIND_REPOSITORY_STORAGE_ROOT` (default: `apps/web/.codemind/repositories`) and registered by workspace. Set `CODEMIND_GITHUB_TOKEN` in the local environment to access private repositories and use authenticated GitHub limits; the token is sent as an HTTP authorization header, not embedded in the repository URL. Durable web operation ownership, progress and result records use this private storage root too. Keep it on persistent storage in deployments; only HTTPS `github.com/owner/repository` URLs are accepted.

Do not add FalkorDB host, graph, or credentials to the web environment. Rust remains the database owner.

## Checks

Run commands from `apps/web/`:

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm test:coverage
pnpm build
pnpm test:e2e
```

Unit and server-contract tests use Vitest. Browser flows use Playwright. Tests that require a live investigation also need the Rust service and FalkorDB running; individual tests document their prerequisites.

## Design and implementation

Before changing a screen, read [`docs/design-system.md`](../../docs/design-system.md) and inspect the applicable reference in [`docs/design/`](../../docs/design/). Compose product UI from the existing shadcn/Base UI primitives and shared tokens.

Follow the active checklist in [`internal-docs/codemind-web-audit-report.md`](../../internal-docs/codemind-web-audit-report.md), the root `AGENTS.md`, and the architecture in [`docs/architecture.md`](../../docs/architecture.md). The checklist is in progress; a listed capability is not considered complete until its required checks pass.

## Troubleshooting

- **gRPC connection failures:** confirm the Rust service is running and `CODEMIND_ENGINE_ENDPOINT` matches its listener.
- **Model calls fail:** confirm `OPENROUTER_API_KEY` is set in `.env.local`; the value is read only on the server.
- **Session checks fail:** set a long random `CODEMIND_SESSION_SECRET` and use the app's authenticated session flow.
- **Database connection failures:** verify the Compose service is healthy. The Next.js process should not connect to FalkorDB directly.
- **Generated route types are missing:** run `pnpm dev` or `pnpm build` to let Next.js regenerate `.next/types`; do not hand-edit `next-env.d.ts`.
