import './lib/themeManager'; // Injects Bulmaswatch stylesheet & keeps it in sync with the theme store

import { mount } from 'svelte';
import App from './App.svelte';
// Novel's compiled CSS lives in the package's dist directory.
import './styles/novel.css';

const target = document.getElementById('app');

if (!target) {
	throw new Error('Mount element #app not found');
}

const app = mount(App, {
	target,
});

export default app;
