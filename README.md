# Zerpto IAM

| Folder | What |
| --- | --- |
| [`service/`](service) | The Rust service (Loco): users, tenants, roles, permissions, extensions, JWT. See its [README](service/README.md) |
| [`federation/`](federation) | The IAM settings screens, loaded by `web.app`. See its [README](federation/README.md) |

One git repository covers both. The CI workflows in `.github/` run inside `service/`.

## Docker

One image holds the backend and the frontend. Build it from the root of this repository; nothing
outside it is needed on disk:

```sh
docker build --secret id=GIT_AUTH_TOKEN -t srv-iam .
```

The shared code is downloaded from GitHub during the build, at the commits in the lockfiles:

| Repository | Version set in |
| --- | --- |
| `zerpto/srv-core`, `zerpto/contract-proto` | [`service/Cargo.toml`](service/Cargo.toml) |
| `zerpto/web-ui`, `zerpto/web-sdk` | [`federation/package.json`](federation/package.json) |

The service pins exact tags. The frontend takes a patch range, and moves to a newer patch only when
you run `pnpm update @zerpto/sdk @zerpto/ui` in `federation/`. After changing a version, commit it
with its lockfile (`Cargo.lock` or `pnpm-lock.yaml`); the build refuses a lockfile that does not
match.

For private repositories, `GIT_AUTH_TOKEN` is an environment variable holding a GitHub token that
can read all four. It is passed as a build secret and is not stored in the image.

### Released images

Publishing a GitHub release builds the image for amd64 and arm64 and publishes it to the GitHub
registry as `ghcr.io/<owner>/iam`, tagged with the version, its `major.minor` and `latest` (a
pre-release gets only the version; see
[release-build.yaml](.github/workflows/release-build.yaml)). The release's tag must be a version,
such as `v0.1.0`. Publish it on GitHub, or with:

```sh
gh release create v0.1.0 --target main --generate-notes
```

The repository needs the secret `REPO_READ_TOKEN`: a GitHub token that can read the four
repositories above.

### Run

Postgres, Redis and NATS run outside the container:

```sh
docker run --env-file <env-file> \
  -p 4001:4001 -p 5001:5001 -p 3001:3001 -v iam-data:/data srv-iam
```

| Port | What | Setting |
| --- | --- | --- |
| `4001` | Backend HTTP | `PORT` |
| `5001` | Backend gRPC | `GRPC_PORT` |
| `3001` | Frontend | `FRONTEND_PORT` |

The env file must set:

| Variables | Used for |
| --- | --- |
| `DATABASE_URL`, `REDIS_URL`, `NATS_URL`, `RELAY_REDIS_URL` | Database, queue and cache, events, the relay service's queue |
| `JWT_SECRET` | Token signing; identical in every service |
| `HOST`, `API_URL`, `WEB_APP_URL` | Public addresses and the CORS origin |
| `REFERENCE_DATA_GRPC_URL` | Reference-data service |
| `IAM_MANIFEST_URL`, `REFERENCE_DATA_MANIFEST_URL`, `RELAY_MANIFEST_URL` | Where each core extension serves its manifest |
| `R2_BUCKET`, `R2_ENDPOINT`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | Uploaded files |

Also set `EXTENSION_URL` to the frontend's public address; `IAM_MANIFEST_URL` is that address plus
`/manifest.yaml`.

Any command other than `serve` runs as given, for example `iam scheduler`.
