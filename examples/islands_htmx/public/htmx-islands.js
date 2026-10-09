// Keeps Leptos islands working in HTML that htmx swaps in, with htmx 2 (2.0.11) and 4 (4.0.0). Loaded by the shell next to htmx.

// htmx parses responses with Document.parseHTMLUnsafe, which drops the comments Leptos hydrates by; without it, htmx uses DOMParser.
delete Document.parseHTMLUnsafe;

{
  // The islands of the first page are Leptos's own: it wakes them once the wasm has loaded.
  for (const el of document.querySelectorAll("leptos-island"))
    el.$$hydrated = true;

  // Wakes every island not yet woken; one that arrives before the wasm is ready is left to Leptos's first pass.
  const wake = () => {
    for (const el of document.querySelectorAll("leptos-island")) {
      if (el.$$hydrated) continue;
      el.$$hydrated = true;
      if (window.__islandsReady)
        window.__hydrateIsland(el, el.dataset.component);
    }
  };

  // htmx fires its swap event on the element that asked, which may be gone with the old content, so the page is watched instead.
  new MutationObserver((records) => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (
          node.nodeType === 1 &&
          (node.matches("leptos-island") || node.querySelector("leptos-island"))
        ) {
          wake();
          return;
        }
      }
    }
  }).observe(document.documentElement, { childList: true, subtree: true });
}
