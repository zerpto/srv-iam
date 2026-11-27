"use client";

/** `next/link` as the screens use it: the host's link, with a string `href`. */
import type { ExtensionLinkProps } from "@zerpto/ui/extension";
import { useHost } from "./host";

export default function Link(props: ExtensionLinkProps) {
  const { Link: HostLink } = useHost();
  return <HostLink {...props} />;
}
