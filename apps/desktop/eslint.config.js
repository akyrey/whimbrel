import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

const INVOKE = { name: "@tauri-apps/api/core", message: "Use the wrappers in src/ipc." };

export default tseslint.config(
  {
    ignores: [
      "dist",
      "coverage",
      "src-tauri",
      "src/ipc/generated",
      "playwright-report",
      "test-results",
    ],
  },
  {
    extends: [js.configs.recommended, ...tseslint.configs.strictTypeChecked],
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
      parserOptions: {
        project: ["./tsconfig.json", "./tsconfig.node.json"],
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: { "react-hooks": reactHooks, "react-refresh": reactRefresh },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/restrict-template-expressions": ["error", { allowNumber: true }],
      "no-console": "error",
      // Components never call `invoke` directly; only src/ipc does.
      "no-restricted-imports": ["error", { paths: [INVOKE] }],
    },
  },
  {
    files: ["src/ipc/**/*.ts"],
    rules: { "no-restricted-imports": "off" },
  },
  {
    // Mocked backends reject with plain `AppError` objects, as the real one does.
    files: ["src/**/*.test.{ts,tsx}", "src/test/**/*.ts", "e2e/**/*.ts"],
    rules: {
      "@typescript-eslint/prefer-promise-reject-errors": "off",
      "@typescript-eslint/only-throw-error": "off",
    },
  },
);
