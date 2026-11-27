/** Server-side settings, read at run time. */
import extension from "../extension.json";

/** Public address of this app, as the browser reaches it, without a trailing slash. */
export function publicUrl() {
  // scripts/run.mjs sets PORT to the port it starts Next.js on.
  const local = `http://localhost:${process.env.PORT || extension.port}`;
  return (process.env.EXTENSION_URL || local).replace(/\/+$/, "");
}

export function urls() {
  const base = publicUrl();
  return {
    manifest: `${base}/manifest.yaml`,
    remote: `${base}/mf/mf-manifest.json`,
  };
}

/** Origin of the web app, allowed (CORS) to load the remote. */
export function webAppOrigin() {
  return new URL(process.env.WEB_APP_URL || "http://localhost:3000").origin;
}
