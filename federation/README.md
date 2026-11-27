# IAM frontend

The IAM settings screens (workspace, team and member permissions, roles, extensions) as a federated
extension the web app (`web.app`) loads at run time. The backend is in [`../service`](../service).

Next.js serves the built files; Rsbuild with Module Federation 2.0 builds the UI in `remote/` into
`public/mf/`. The host/extension contract is in the
[`web-ui` README](https://github.com/zerpto/web-ui#readme).

Sign-in, registration, password, invitation, onboarding and account pages stay in `web.app`: an
extension only runs for a signed-in user and the host refuses it session calls.

## Set up and run

Needs Node 22.12 or later and pnpm. `@zerpto/ui` and `@zerpto/sdk` are installed from GitHub
(`zerpto/web-ui` and `zerpto/web-sdk`) from tags in the version range in
[`package.json`](package.json), so Git needs read access to both. The SDK is built during the
install, which [`pnpm-workspace.yaml`](pnpm-workspace.yaml) allows. A change in a local checkout of
either is not picked up: release a tag first.

`pnpm-lock.yaml` keeps the installed commit. To take a newer patch release, run
`pnpm update @zerpto/sdk @zerpto/ui` and commit the lockfile; for a newer minor release, change the
range in `package.json` and run `pnpm install`.

```bash
pnpm install
pnpm dev          # http://localhost:3001, rebuilds the remote on change
pnpm build && pnpm start
```

| Command                               | What   |
| ------------------------------------- | ------ |
| `pnpm typecheck`, `pnpm format:check` | Checks |

Settings are in [`.env.example`](.env.example): `EXTENSION_URL` and `WEB_APP_URL`.

## How the web app finds it

`iam` is a core extension, so nothing is registered. This app serves the extension's manifest,
[`manifest.yaml`](manifest.yaml), at `/manifest.yaml`. IAM fetches it from `IAM_MANIFEST_URL`
(default `http://localhost:3001/manifest.yaml` in development) when it starts, retrying until this
app answers, and `web.app` then shows these screens under `/settings` in place of its built-in ones
(`web.app/src/components/extensions/core-settings-gate.tsx`). Restart IAM after a change to the
manifest. With the variable empty, IAM uses the copy compiled into it, which has the permissions and
menu entries but no screens, so the web app keeps its built-in pages. An IAM database that already
holds the fetched manifest drops the screens the same way on its next start.

A production web app can load production builds only, which `pnpm dev` also produces.

## Layout

```
extension.json     key, route, port
manifest.yaml      the extension's manifest: permissions, menu entries, remote
app/, lib/, proxy.ts   the Next.js server (serves the manifest; CORS for the web app)
remote/Routes.tsx  the exposed module: providers and page switch
remote/router.ts   app paths to screens
remote/components/, remote/lib/   the screens, as copied from web.app
remote/shims/      stand-ins for next/link and next/navigation
remote/messages/   English and Bahasa Indonesia
```

`@/` means `remote/`, and `next-intl` resolves to `use-intl`, so the copied files keep their
imports. The `web.app` originals are switched off, not deleted: until they are, a change to a
screen has to be made here.
