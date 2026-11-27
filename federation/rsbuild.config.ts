/** Builds the federated UI (`remote/`) into `public/mf/`, which Next.js serves. */
import { pluginModuleFederation } from "@module-federation/rsbuild-plugin";
import { defineConfig } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import tailwindcss from "@tailwindcss/postcss";
import { remoteName, remoteShared, ROUTES_EXPOSE } from "@zerpto/ui/federation";
import extension from "./extension.json" with { type: "json" };

export default defineConfig({
  // Also under `--watch`: a production web app can load production builds only.
  mode: "production",
  plugins: [
    pluginReact(),
    pluginModuleFederation({
      name: remoteName(extension.key),
      exposes: { [ROUTES_EXPOSE]: "./remote/Routes.tsx" },
      // No shared library is bundled; the host provides them all.
      shared: remoteShared(),
      dts: false,
    }),
  ],
  source: {
    // The host never loads the entry, only the exposed module.
    entry: { index: "./remote/hosted.ts" },
    tsconfigPath: "./tsconfig.remote.json",
  },
  resolve: {
    dedupe: ["react", "react-dom", "@headlessui/react", "@heroicons/react", "clsx"],
    // The screens came from web.app: what they take from Next.js is replaced here (and in
    // tsconfig.remote.json), so no part of Next.js is in the remote.
    alias: {
      "@": "./remote",
      "next-intl": "use-intl",
      "next/link": "./remote/shims/next-link.tsx",
      "next/navigation": "./remote/shims/next-navigation.ts",
    },
  },
  // Rsbuild would copy `public/`, which holds its own output, into the output.
  server: { publicDir: false },
  tools: {
    htmlPlugin: false,
    // Here and not in a postcss.config file, which Next.js would read too.
    postcss: {
      postcssOptions: {
        plugins: [
          tailwindcss(),
          {
            // Writes the extension key into the scope selector of `remote/styles.css`.
            postcssPlugin: "zerpto-extension-key",
            Rule(rule: { selector: string }) {
              if (rule.selector.includes("__EXTENSION_KEY__")) {
                rule.selector = rule.selector.replaceAll("__EXTENSION_KEY__", extension.key);
              }
            },
          },
        ],
      },
    },
  },
  output: {
    // Chunks load relative to wherever the host fetched the remote from.
    assetPrefix: "auto",
    distPath: { root: "public/mf" },
  },
});
