import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import svelteParser from "svelte-eslint-parser";
import tsParser from "@typescript-eslint/parser";
import tseslint from "@typescript-eslint/eslint-plugin";
import globals from "globals";

export default [
  js.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      "@typescript-eslint": tseslint,
    },
    rules: {
      "svelte/no-navigation-without-resolve": "off",
    },
  },
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tsParser,
        extraFileExtensions: [".svelte"],
      },
    },
  },
  {
    files: ["**/*.ts"],
    languageOptions: {
      parser: tsParser,
      sourceType: "module",
    },
  },
  {
    ignores: [
      "dist/**",
      "build/**",
      ".svelte-kit/**",
      "node_modules/**",
      "src-tauri/target/**",
    ],
  },
];
