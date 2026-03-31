// @ts-nocheck
export const CONFIG = {
	ServerURL:
		import.meta.env.MODE !== 'production'
			? 'http://localhost:8000'
			: `${location.protocol}//${window.location.host}` || '/',
	// ServerURL: location.protocol + '//'+window.location.host || '/',
	// ServerURL: "https://alexmikhalev.terraphim.cloud",
};