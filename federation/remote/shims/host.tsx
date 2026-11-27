"use client";

import type { ExtensionRoutesProps } from "@zerpto/ui/extension";
import { createContext, useContext } from "react";

/** What the host passes to `Routes`, for the code that came from web.app. */
export type Host = Pick<
  ExtensionRoutesProps,
  "path" | "search" | "permissions" | "navigate" | "Link"
>;

const HostContext = createContext<Host | null>(null);

export const HostProvider = HostContext.Provider;

export function useHost() {
  const host = useContext(HostContext);
  if (!host) throw new Error("This component only renders inside the extension's Routes.");
  return host;
}
