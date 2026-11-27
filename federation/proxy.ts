import { type NextRequest, NextResponse } from "next/server";
import { webAppOrigin } from "./lib/config";

/**
 * CORS for everything the web app reads from another origin. Done here rather than in
 * `next.config.ts`, whose headers are fixed at build time, so `WEB_APP_URL` applies at start.
 */
export function proxy(request: NextRequest) {
  const preflight = request.method === "OPTIONS";
  const response = preflight ? new NextResponse(null, { status: 204 }) : NextResponse.next();
  response.headers.set("Access-Control-Allow-Origin", webAppOrigin());
  if (preflight) {
    response.headers.set("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS");
    response.headers.set("Access-Control-Allow-Headers", "Content-Type");
    response.headers.set("Access-Control-Max-Age", "600");
  }
  return response;
}

export const config = {
  matcher: ["/manifest.yaml", "/mf/:path*"],
};
