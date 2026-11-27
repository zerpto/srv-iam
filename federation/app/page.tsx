import { urls } from "../lib/config";

export const dynamic = "force-dynamic";

/** Says what this app serves. The screens themselves only run inside the web app. */
export default function Home() {
  const { manifest, remote } = urls();
  return (
    <main>
      <h1>IAM extension</h1>
      <p>Its screens are loaded by the web app under /settings.</p>
      <p>
        Manifest: <a href={manifest}>{manifest}</a>
      </p>
      <p>
        Module Federation manifest: <a href={remote}>{remote}</a>
      </p>
    </main>
  );
}
