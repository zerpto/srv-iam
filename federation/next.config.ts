import path from "node:path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  devIndicators: false,
  poweredByHeader: false,
  // Keeps `next dev` from writing its own blocks into AGENTS.md and CLAUDE.md.
  agentRules: false,
  // Two directories up, from where the scripts run. The standalone server keeps this directory's
  // path below that root, and the Dockerfile copies it from there: change both together.
  turbopack: { root: path.join(process.cwd(), "../..") },
};

export default nextConfig;
