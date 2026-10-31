import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import hooks from 'eslint-plugin-react-hooks';
import refresh from 'eslint-plugin-react-refresh';
import globals from 'globals';
export default tseslint.config({ ignores: ['dist', 'node_modules', 'src-tauri/target', 'src-tauri/gen'] }, js.configs.recommended, ...tseslint.configs.recommended, { files: ['src/**/*.{ts,tsx}'], languageOptions: { globals: globals.browser }, plugins: { 'react-hooks': hooks, 'react-refresh': refresh }, rules: { ...hooks.configs.recommended.rules, 'react-refresh/only-export-components': ['warn', { allowConstantExport: true }] } }, { files: ['scripts/*.mjs','*.js'], languageOptions: { globals: globals.node } });
