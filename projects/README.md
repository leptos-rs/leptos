# `projects` README

The `projects` directory is intended as a collective of medium-to-large-scale examples: a place to show a variety of use cases and integrations between Leptos and other libraries. Over time, our hope is that this allows us to showcase a wider variety of user examples, without the main `examples` directory becoming too overwhelming to be useful.

The `examples` directory is included in our CI, and examples are regularly linted and tested. The barrier to entry for the `projects` directory is intended to be lower: Example projects will generally be built against a particular version, and not regularly linted or updated. Hopefully this distinction allows us to accept more examples without worrying about the maintenance burden of constant updates.

Feel free to submit projects to this directory via PR!


## Index

### meilisearch-searchbar 
[Meilisearch](https://www.meilisearch.com/) is a search engine built in Rust that you can self-host. This example shows how to run it alongside a leptos server and present a search bar with autocomplete to the user.

### nginx-mpmc 
[Nginx](https://nginx.org/) Multiple Producer Multi Consumer, this example shows how you can use Nginx to provide different clients to the user while running multiple Leptos servers that provide server functions to any of the clients.

### ory-kratos 
[Ory](https://www.ory.sh/docs/welcome) is a combination of different authorization services. Ory Kratos is their Identification service, which provides password storage, emailing, login and registration functionality, etc. This example shows running Ory Kratos alongside a leptos server and making use of their UI Node data types in leptos. TODO: This example needs a bit more work to show off SSO passwordless etc 

### tauri-from-scratch
This example walks you through in explicit detail how to use [Tauri](https://tauri.app/) to render your Leptos App on non web targets using [WebView](https://en.wikipedia.org/wiki/WebView) while communicating with your leptos server and servering an SSR supported web experience. TODO: It could be simplified since part of the readme includes copying and pasting boilerplate.

### counter_dwarf_debug
This example shows how to add breakpoints within the browser or visual studio code for debugging.

### bevy3d_ui
This example uses the bevy 3d game engine with leptos within webassembly.
