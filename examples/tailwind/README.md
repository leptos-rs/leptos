# Leptos Starter Template

This is a template demonstrating how to integrate [TailwindCSS](https://tailwindcss.com/) with the [Leptos](https://github.com/leptos-rs/leptos) web framework and the [cargo-leptos](https://github.com/akesson/cargo-leptos) tool.

## Getting Started

See the [Examples README](../README.md) for setup and run instructions.

## Tailwind

You can install Tailwind using `npm`:

```bash
npm install -D tailwindcss
```

If you'd rather not use `npm`, you can install the Tailwind binary [here](https://github.com/tailwindlabs/tailwindcss/releases).

## Setting up with VS Code and Additional Tools

If you're using VS Code, add the following to your `settings.json`

```json
  "emmet.includeLanguages": {
    "rust": "html",
    "*.rs": "html"
  },
  "tailwindCSS.includeLanguages": {
      "rust": "html",
      "*.rs": "html"
  },
  "files.associations": {
      "*.rs": "rust"
  },
  "editor.quickSuggestions": {
    "other": "on",
    "comments": "on",
    "strings": true
  },
  "css.validate": false,
```

Install [Tailwind CSS Intellisense](https://marketplace.visualstudio.com/items?itemName=bradlc.vscode-tailwindcss).

Install [VS Browser](https://marketplace.visualstudio.com/items?itemName=Phu1237.vs-browser) extension (allows you to open a browser at the right window).

Allow vscode Ports forward: 3000, 3001.

### Attribution

Many thanks to GreatGreg for putting together this guide. You can find the original, with added details, [here](https://github.com/leptos-rs/leptos/discussions/125).
