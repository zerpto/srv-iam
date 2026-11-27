#!/usr/bin/env bash
# `serve` (the default) runs the backend and the frontend together: signals go to both, and when one
# stops the other is stopped and the container exits. Any other command runs as given, for example
# `iam db migrate` or `iam scheduler`.
#
#   BACKEND_ARGS    arguments of the backend (default: start --server-and-worker)
#   PORT            backend HTTP port (default 4001, config/production.yaml)
#   GRPC_PORT       backend gRPC port (default 5001)
#   FRONTEND_PORT   frontend port (default: `port` in extension.json)
#   LOG_DIR         where backend.log and frontend.log are appended, besides the container's own
#                   output (default /data/logs; empty turns the files off). Nothing rotates them.
set -uo pipefail

if [[ "${1:-serve}" != "serve" ]]; then
  exec "$@"
fi

pids=()
stopping=0

stop_all() {
  # A process that already ended is not an error.
  kill -TERM "${pids[@]}" 2>/dev/null || true
}

on_signal() {
  stopping=1
  stop_all
}
trap on_signal TERM INT

# Output of each server goes to the container's output and, when LOG_DIR can be written, to a file.
backend_out=1
frontend_out=1
log_fds=()
tee_pids=()
if [[ -n "${LOG_DIR-}" ]]; then
  if mkdir -p "${LOG_DIR}" 2>/dev/null && [[ -w "${LOG_DIR}" ]]; then
    exec {backend_out}> >(exec tee -a "${LOG_DIR}/backend.log")
    tee_pids+=($!)
    exec {frontend_out}> >(exec tee -a "${LOG_DIR}/frontend.log")
    tee_pids+=($!)
    log_fds=("${backend_out}" "${frontend_out}")
  else
    echo "entrypoint: cannot write to ${LOG_DIR}; logging to the container output only" >&2
  fi
fi

read -r -a backend_args <<<"${BACKEND_ARGS:-start --server-and-worker}"

(cd /app/service && exec iam "${backend_args[@]}") >&"${backend_out}" 2>&1 &
pids+=($!)

# Both servers read PORT, so the frontend gets its own value. HOSTNAME is the address Next.js binds
# to; Docker sets it to the container id, so it is replaced.
(
  cd "${FEDERATION_DIR}" &&
    port="${FRONTEND_PORT:-$(node -p 'require("./extension.json").port')}" &&
    PORT="${port}" HOSTNAME=0.0.0.0 exec node server.js
) >&"${frontend_out}" 2>&1 &
pids+=($!)

# Returns when either process ends, or when a signal arrives.
wait -n "${pids[@]}"
status=$?

if ((stopping)); then
  status=0
else
  echo "entrypoint: a process exited with status ${status}; stopping the other" >&2
  # Neither server should end on its own, so a clean exit counts as a failure too.
  ((status != 0)) || status=1
  stop_all
fi

wait "${pids[@]}" 2>/dev/null
# Closing their input lets the log writers finish the last lines.
for fd in "${log_fds[@]}"; do exec {fd}>&-; done
((${#tee_pids[@]} == 0)) || wait "${tee_pids[@]}" 2>/dev/null
exit "${status}"
