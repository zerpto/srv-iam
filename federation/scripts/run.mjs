/**
 * `node scripts/run.mjs dev|start`: runs Next.js on the port of `extension.json`, or `PORT`.
 * `dev` also rebuilds the federated UI on change.
 */
import { spawn, spawnSync } from "node:child_process";
import extension from "../extension.json" with { type: "json" };

const mode = process.argv[2];
if (mode !== "dev" && mode !== "start") {
  console.error("Usage: node scripts/run.mjs dev|start");
  process.exit(1);
}

const port = process.env.PORT || String(extension.port);
const env = { ...process.env, PORT: port };
const commands = [["next", [mode, "--port", port]]];

if (mode === "dev") {
  // Build once first, so the remote is there when the server answers.
  const first = spawnSync("rsbuild", ["build"], { stdio: "inherit", env });
  if (first.status !== 0) process.exit(first.status ?? 1);
  commands.push(["rsbuild", ["build", "--watch"]]);
}

const children = commands.map(([command, args]) => spawn(command, args, { stdio: "inherit", env }));

let stopping = false;
/** Ends every process; one that stops on its own takes the others with it. */
function stop(code) {
  if (stopping) return;
  stopping = true;
  process.exitCode = code;
  for (const child of children) child.kill("SIGTERM");
}

for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => stop(0));
for (const child of children) {
  child.on("error", (error) => {
    console.error(error);
    stop(1);
  });
  child.on("exit", (code) => stop(code ?? 1));
}
