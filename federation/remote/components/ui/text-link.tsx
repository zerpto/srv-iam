"use client";

import { type Size, textLinkStyles, type Variant } from "@zerpto/ui";
import Link from "next/link";
import { isChangePath, useAccess } from "@/lib/permissions";

type TextLinkProps = React.ComponentProps<typeof Link> & {
  /** Renders the link as a button (same looks as `Button`); without it, a plain text link. */
  variant?: Variant;
  /** Button size, only with `variant`. */
  size?: Size;
};

/**
 * The one link component. A text link looks like the text around it (same colour, size and
 * weight, no underline) and turns brand purple on hover; with `variant` it looks like a `Button`
 * but still navigates. Links to `/new` and `/edit` pages need `<module>.manage` (`lib/permissions`).
 */
export function TextLink({ variant, size, className, ...props }: TextLinkProps) {
  // A link to a `/new` or `/edit` page is hidden from people who may not change that area.
  const target = props.href;
  const changes = isChangePath(target);
  const access = useAccess();
  if (changes && !(access.ready && access.can("manage", target))) return null;

  return <Link {...props} className={textLinkStyles(variant, className, size)} />;
}
