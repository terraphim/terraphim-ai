import './lib/themeManager'; // Injects Bulmaswatch stylesheet & keeps it in sync with the theme store

import App from './App.svelte';
// Novel's compiled CSS lives in the package's dist directory.
import './styles/novel.css';

const app = new App({
	target: document.getElementById('app'),
});

export default app;
