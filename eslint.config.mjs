// eslint.config.mjs
import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default [
  // Global ignores (generated / vendor / build output)
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/.svelte-kit/**',
      '**/target/**',
      '**/coverage/**',
    ],
  },

  // Reasonable defaults for JS
  js.configs.recommended,

  // TypeScript rules (non-type-aware, keeps it simple for now)
  ...tseslint.configs.recommended,

  // What to lint
  {
    files: ['**/*.{js,cjs,mjs,ts,tsx}'],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
    },
  },
];
