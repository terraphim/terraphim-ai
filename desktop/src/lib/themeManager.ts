import { theme } from "./stores";

// Keep a reference to the <link> that currently provides the Bulmaswatch theme so we can swap it
let current: HTMLLinkElement | null = null;

function applyTheme(name: string) {
  // Skip when running in SSR or during build where `document` is not defined
  if (typeof document === "undefined") return;

  const href = `/assets/bulmaswatch/${name}/bulmaswatch.min.css`;

  // Short-circuit if the requested theme is already active
  if (current?.href.endsWith(href)) {
    return;
  }

  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = href;
  link.id = "bulma-theme";

  // Once the new CSS has loaded, remove the old one to avoid flashing
  link.onload = () => {
    if (current && current !== link) {
      current.remove();
    }
    current = link;
  };

  document.head.appendChild(link);

  // Update <meta name="color-scheme"> if present (optional)
  const meta = document.head.querySelector('meta[name="color-scheme"]');
  if (meta) {
    meta.setAttribute("content", name);
  }
}

// Subscribe exactly once for the lifetime of the app.
// Each time the `theme` store changes we (re)apply the stylesheet.
// The subscription callback is immediately invoked with the current value,
// which injects the initial theme during startup.

/* eslint-disable @typescript-eslint/no-unused-vars */
const unsubscribe = theme.subscribe(applyTheme);
// We never unsubscribe because the application lives as long as the page.
