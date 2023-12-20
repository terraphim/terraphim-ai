export const CONFIG = {
  ServerURL: process.env.NODE_ENV !== 'production' ? 'http://localhost:8000' : location.protocol + '//' + window.location.host || '/',
  // ServerURL: location.protocol + '//'+window.location.host || '/',
  // ServerURL: "https://alexmikhalev.terraphim.cloud",
};
