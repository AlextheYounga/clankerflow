import js from "@eslint/js";
import tseslint from "typescript-eslint";
import eslintComments from "eslint-plugin-eslint-comments";
import importPlugin from "eslint-plugin-import";
import unicorn from "eslint-plugin-unicorn";
import globals from "globals";

export default tseslint.config(
  {
    ignores: ["node_modules/**", "dist/**", "coverage/**"],
  },

  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,

  {
    files: ["src/**/*.ts", "tests/**/*.ts"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      parserOptions: {
        projectService: true,
      },
      globals: {
        ...globals.node,
      },
    },
    plugins: {
      "eslint-comments": eslintComments,
      import: importPlugin,
      unicorn,
    },
    rules: {
      "max-lines-per-function": [
        "warn",
        {
          max: 50,
          skipBlankLines: true,
          skipComments: true,
          IIFEs: true,
        },
      ],
      "complexity": ["warn", { max: 10 }],
      "max-depth": ["warn", { max: 3 }],
      "max-params": ["warn", { max: 4 }],

      "eslint-comments/no-unlimited-disable": "error",
      "eslint-comments/no-unused-disable": "warn",

      "@typescript-eslint/consistent-type-imports": "error",
      "@typescript-eslint/no-confusing-void-expression": [
        "warn",
        { ignoreArrowShorthand: true },
      ],
      "@typescript-eslint/switch-exhaustiveness-check": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
      "@typescript-eslint/no-unnecessary-type-assertion": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/prefer-optional-chain": "warn",
      "@typescript-eslint/prefer-nullish-coalescing": "warn",
      "@typescript-eslint/require-await": "warn",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/no-unsafe-argument": "warn",
      "@typescript-eslint/no-unsafe-return": "warn",
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/restrict-plus-operands": "error",
      "@typescript-eslint/strict-boolean-expressions": "warn",
      "@typescript-eslint/no-base-to-string": "warn",
      "@typescript-eslint/restrict-template-expressions": [
        "warn",
        { allowNumber: true },
      ],

      "import/no-cycle": "error",
      "import/no-duplicates": "error",
      "import/order": [
        "error",
        { "newlines-between": "always" }
      ],

      "unicorn/consistent-function-scoping": "warn",
      "unicorn/no-array-for-each": "warn",

      "@typescript-eslint/naming-convention": [
        "warn",
        {
          selector: "default",
          format: ["camelCase"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        {
          selector: "typeLike",
          format: ["PascalCase"],
        },
        {
          selector: "variable",
          format: ["camelCase", "UPPER_CASE", "PascalCase"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        {
          selector: "function",
          format: ["camelCase"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        {
          selector: "parameter",
          format: ["camelCase"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        {
          selector: "memberLike",
          modifiers: ["private"],
          format: ["camelCase"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        {
          selector: "variable",
          types: ["boolean"],
          // After stripping the prefix, the remainder is PascalCase (e.g. isActive → Active).
          format: ["PascalCase"],
          prefix: ["is", "has", "can", "should", "did", "will"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        {
          selector: "parameter",
          types: ["boolean"],
          // After stripping the prefix, the remainder is PascalCase (e.g. isActive → Active).
          format: ["PascalCase"],
          prefix: ["is", "has", "can", "should", "did", "will"],
          leadingUnderscore: "allow",
          trailingUnderscore: "allow",
        },
        // IPC protocol objects use snake_case to match the Rust wire format.
        // Object literal properties and type properties are exempt from camelCase.
        {
          selector: "objectLiteralProperty",
          format: null,
        },
        {
          selector: "typeProperty",
          format: null,
        },
      ],
    },
  },

  {
    files: ["tests/**/*.ts"],
    rules: {
      "max-lines-per-function": [
        "warn",
        {
          max: 120,
          skipBlankLines: true,
          skipComments: true,
          IIFEs: true,
        },
      ],
      "@typescript-eslint/no-floating-promises": "off",
      // Boolean naming prefix rules are noisy in test assertions; disable for tests.
      "@typescript-eslint/naming-convention": "off",
    },
  }
);