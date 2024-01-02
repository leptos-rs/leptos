# todo_app_sqlite_pavex

# Getting started

## Prerequisites

- Rust (see [here](https://www.rust-lang.org/tools/install) for instructions)
- `cargo-px`:
  ```bash
  cargo install --locked cargo-px --version="~0.1"
  ```
- [Pavex](https://pavex.dev)

## Useful commands

`todo_app_sqlite_pavex` is built using the [Pavex](https://pavex.dev) web framework, which relies on code generation.  
You need to use the `cargo px` command instead of `cargo`: it ensures that the
`todo_app_sqlite_pavex_server_sdk` crate is correctly regenerated when the 
application blueprint changes.

`cargo px` is a wrapper around `cargo` that will automatically regenerate the
server SDK when needed. Check out its [documentation](https://github.com/LukeMathWalker/cargo-px)
for more details.

### Build

```bash
cargo px build
```

### Run

```bash
cargo px run
```

### Test

```bash
cargo px test
```

## Configuration

All configurable parameters are listed in `todo_app_sqlite_pavex/src/configuration.rs`.

Configuration values are loaded from two sources:

- Configuration files
- Environment variables

Environment variables take precedence over configuration files.

All configuration files are in the `todo_app_sqlite_pavex_server/configuration` folder.
The application can be run in three different profiles: `dev`, `test` and `prod`.  
The settings that you want to share across all profiles should be placed in `todo_app_sqlite_pavex_server/configuration/base.yml`.
Profile-specific configuration files can be then used
to override or supply additional values on top of the default settings (e.g. `todo_app_sqlite_pavex_server/configuration/dev.yml`).

You can specify the app profile that you want to use by setting the `APP_PROFILE` environment variable; e.g.:

```bash
APP_PROFILE=prod cargo px run
```

for running the application with the `prod` profile.

By default, the `dev` profile is used since `APP_PROFILE` is set to `dev` in the `.env` file at the root of the project.
The `.env` file should not be committed to version control: it is meant to be used for local development only,
so that each developer can specify their own environment variables for secret values (e.g. database credentials)
that shouldn't be stored in configuration files (given their sensitive nature).
