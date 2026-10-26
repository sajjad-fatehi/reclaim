import eslint from '@eslint/js';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import typescript from 'typescript-eslint';

export default typescript.config(
  {
    ignores: [
      'dist/**',
      'target/**',
      'src-tauri/gen/**',
      '.Codex/worktrees/**',
      '.codex/worktrees/**',
      '.claude/worktrees/**',
    ],
  },
  eslint.configs.recommended,
  typescript.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: { 'react-hooks': reactHooks, 'react-refresh': reactRefresh },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': 'error',
    },
  },
);
