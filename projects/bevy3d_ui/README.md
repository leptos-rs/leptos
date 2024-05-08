# Bevy 3D UI Example

This example combines a leptos UI with a bevy 3D view.  
Bevy is a 3D game engine written in rust that can be compiled to web assembly by using the wgpu library.  
The wgpu library in turn can target the newer webgpu standard or the older webgl for web browsers.

In the case of a desktop application, if you wanted to use a styled ui via leptos and a 3d view via bevy
you could also combine this with tauri.  

## Quick Start

  * Run `trunk serve to run the example.
  * Browse to http://127.0.0.1:8080/

It's best to use a web browser with webgpu capability for best results such as Chrome or Opera.
