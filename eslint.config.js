import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

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
    files: ["*.js", "**/*.config.{js,ts}", "**/vite.config.ts"],
    languageOptions: { globals: { ...globals.node } },
  },
);
