import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";
import betterTailwind from "eslint-plugin-better-tailwindcss";

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/node_modules/**",
      "**/generated/**",
      "src-tauri/**",
      "crates/**",
      "spikes/**",
      "target/**",
    ],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: { globals: { ...globals.browser } },
    plugins: { "react-hooks": reactHooks, "react-refresh": reactRefresh },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      // C1: playback only through libmpv.
      "no-restricted-syntax": [
        "error",
        {
          selector: "JSXOpeningElement[name.name=/^(video|audio)$/]",
          message: "C1: <video>/<audio> are forbidden, playback goes through libmpv.",
        },
        {
          selector: "NewExpression[callee.name='Audio']",
          message: "C1: new Audio() is forbidden, playback goes through libmpv.",
        },
        {
          selector:
            "CallExpression[callee.object.name='document'][callee.property.name='createElement'][arguments.0.value=/^(video|audio)$/]",
          message: "C1: <video>/<audio> are forbidden, playback goes through libmpv.",
        },
      ],
    },
  },
  {
    // Only correctness rules: class order is owned by prettier-plugin-tailwindcss.
    // no-unknown-classes is what rejects default palette classes (bg-blue-500),
    // since theme.css removes them with `--color-*: initial`. entryPoint is resolved
    // from cwd: a path from the repo root silently falls back to the default theme.
    files: ["ui/tv/**/*.tsx", "ui/shared/**/*.tsx"],
    ...betterTailwind.configs["correctness-error"],
    rules: {
      ...betterTailwind.configs["correctness-error"].rules,
      // Focus moved by script (spatial navigation, phone remote) may not match
      // :focus-visible, and the TV focus must always show (ARCHITECTURE.md section 6).
      "better-tailwindcss/no-restricted-classes": [
        "error",
        {
          restrict: [
            {
              pattern: "^(.*:)?focus-visible:(.*)$",
              fix: "$1focus:$2",
              message: "Use focus: on the TV, focus-visible: misses script-moved focus.",
            },
          ],
        },
      ],
    },
    settings: {
      "better-tailwindcss": { entryPoint: "src/index.css", cwd: "ui/tv" },
    },
  },
  {
    // Components reach the backend only through the injected ApiClient (ARCHITECTURE.md
    // section 6): Tauri stays in the TV's client implementation.
    files: ["ui/**/*.{ts,tsx}"],
    ignores: ["ui/tv/src/api/tauri.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["@tauri-apps/*"],
              message:
                "Use the ApiClient from useApi()/useTvApi(); only ui/tv/src/api/tauri.ts talks to Tauri.",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["*.js", "**/*.config.{js,ts}", "**/vite.config.ts"],
    languageOptions: { globals: { ...globals.node } },
  },
);
