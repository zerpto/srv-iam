import clsx from "clsx";

/** Grey placeholder block. Parents set `animate-pulse` and `aria-busy`. */
export function Bone({ className }: { className?: string }) {
  return <div className={clsx("rounded-md bg-zinc-200 dark:bg-white/10", className)} />;
}
