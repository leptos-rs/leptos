# A Running List of Small Developer Experience Improvements

## Autocompletion inside `#[component]` and `#[server]`

Because of the nature of macros (they can expand from anything to anything, but only if the input is exactly correct at that instant) it can be hard for rust-analyzer to do proper autocompletion and other support.

But you can tell rust-analyzer to ignore certain proc macros. For `#[component]` and `#[server]` especially, which annotate function bodies but don't actually transform anything inside the body of your function, this can be really helpful.

Note that this means that rust-analyzer doesn't know about your component props, which may generate its own set of errors or warnings in the IDE.

VSCode `settings.json`:

```json
"rust-analyzer.procMacro.ignored": {
	"leptos_macro": [
		"component",
		"server"
	],
}
```

neovim with lspconfig:

```lua
require('lspconfig').rust_analyzer.setup {
  -- Other Configs ...
  settings = {
    ["rust-analyzer"] = {
      -- Other Settings ...
      procMacro = {
        ignored = {
            leptos_macro = {
                "component",
                "server",
            },
        },
      },
    },
  }
}
```

Helix, in `.helix/languages.toml`:

```toml
[[language]]
name = "rust"

[language-server.rust-analyzer]
config = { procMacro = { ignored = { leptos_macro = ["component", "server"] } } }
```

```admonish info
The Jetbrains `intellij-rust` plugin (RustRover as well) currently does not support dynamic config for macro exclusion.
However, the project currently maintains a hardcoded list of excluded macros.
As soon as [this open PR](https://github.com/intellij-rust/intellij-rust/pull/10873) is merged, the `component` and
`server` macro will be excluded automatically without additional configuration needed.  

Update (2023/10/02):  
The `intellij-rust` plugin got deprecated in favor of RustRover at the same time the PR was opened, but an official
support request was made to integrate the contents of this PR.
```
