import { readFile } from "node:fs/promises";
import path from "node:path";
import { remoteShared, ROUTES_EXPOSE } from "@zerpto/ui/federation";
import { urls } from "../../lib/config";

// Filled on each request, so the URLs follow the settings the server started with.
export const dynamic = "force-dynamic";

/** `manifest.yaml` from the project root, with every `${name}` filled. */
export async function GET() {
  const values: Record<string, string> = {
    remote: urls().remote,
    expose: ROUTES_EXPOSE,
    // JSON is valid YAML.
    shared: JSON.stringify(
      Object.fromEntries(
        Object.entries(remoteShared()).map(([name, config]) => [name, config.requiredVersion]),
      ),
    ),
  };
  const template = await readFile(path.join(process.cwd(), "manifest.yaml"), "utf8");
  const body = template.replace(/\$\{(\w+)\}/g, (placeholder, name: string) => {
    const value = values[name];
    if (value === undefined) throw new Error(`manifest.yaml: unknown ${placeholder}`);
    return value;
  });
  return new Response(body, {
    headers: { "Content-Type": "application/yaml; charset=utf-8", "Cache-Control": "no-cache" },
  });
}
