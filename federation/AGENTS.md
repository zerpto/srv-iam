# Agent guide: IAM frontend

Read [README.md](README.md) and the [`web-ui` README](https://github.com/zerpto/web-ui#readme) (the
host and extension contract) first. `@zerpto/ui` and `@zerpto/sdk` come from GitHub, from tags in the
range in `package.json`; `node_modules/@zerpto/ui/README.md` is the copy for the installed version.

## Rules

- Stack: Next.js 16 (App Router) serves the manifest and the remote's files; Rsbuild with Module Federation 2.0
  builds `remote/`. React, TypeScript, Tailwind CSS, Headless UI, Heroicons, `@zerpto/ui`,
  `@zerpto/sdk`, TanStack Query, react-hook-form, zod and use-intl. pnpm only.
- This Next.js version differs from what you may remember: read `node_modules/next/dist/docs/`
  before writing Next.js code.
- No `next/*` or `next-intl` code in `remote/` beyond the shims in `remote/shims/`.
- API calls go through the SDK clients in `remote/lib/api/sdk.ts`, which use the host's `apiFetch`.
  Never set `Authorization`, and never call `/v1/auth/*` except `GET /v1/auth/me`.
- Permissions only hide controls; the service enforces access.
- `manifest.yaml` is the only copy of IAM's manifest: a new IAM permission or menu entry goes
  here. The service compiles it in too, so only the app fields may be `${name}` placeholders, and
  it must pass `../service/src/extensions/manifest_check.rs`.
- Hosted code adds no global CSS. `public/mf/` is generated; never edit or commit it.
- All UI text in English and Bahasa Indonesia (`remote/messages/`).
- Never read or print `.env`; nothing secret belongs here.
- Never run remote git operations. Local commits only when asked.

## Checks

`pnpm typecheck`, `pnpm format:check` and `pnpm build`.
