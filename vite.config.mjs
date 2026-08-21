import { defineConfig } from 'vite';
import { resolve } from 'path';
import svgLoader from 'vite-svg-loader';
import vue from '@vitejs/plugin-vue';

const root = resolve(import.meta.dirname, 'src');
const outDir = resolve(import.meta.dirname, 'dist');

// https://vitejs.dev/config/
export default defineConfig({
    root,
    resolve:{
        alias:{
            '@' : resolve(import.meta.dirname, './src')
        },
    },
    plugins: [vue(), svgLoader()],
    build: {
        outDir,
        emptyOutDir: true,
        rolldownOptions: {
            input: {
                settings: resolve(root, 'pages', 'settings', 'index.html'),
                about: resolve(root, 'pages', 'about', 'index.html')
            }
        }
    }
});
