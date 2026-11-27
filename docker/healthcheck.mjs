/** Healthy when the backend's `/_health` and the frontend's `/manifest.yaml` both answer. */
import { readFile } from "node:fs/promises";

const extension = JSON.parse(
  await readFile(`${process.env.FEDERATION_DIR}/extension.json`, "utf8"),
);
const backend = process.env.PORT || "4001";
const frontend = process.env.FRONTEND_PORT || String(extension.port);

const urls = [
  `http://127.0.0.1:${backend}/_health`,
  `http://127.0.0.1:${frontend}/manifest.yaml`,
];

try {
  await Promise.all(
    urls.map(async (url) => {
      const response = await fetch(url, { signal: AbortSignal.timeout(4000) });
      if (!response.ok) throw new Error(`${url}: ${response.status}`);
    }),
  );
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
