# Sitemaps with Axum

This project demonstrates how to serve a [sitemap](https://developers.google.com/search/docs/crawling-indexing/sitemaps/overview) file using Axum using dynamic data (like blog posts in this case). An example Postgres database is used data source for storing blog post data that can be used to generate a dynamic site map based on blog post slugs. There's lots of [sitemap crates](https://crates.io/search?q=sitemap), though this example uses the [xml](https://crates.io/crates/xml) for example purposes.

## Quick Start

We use Docker to provide a Postgres database for this sample, so make sure you have it installed.

```sh
$ docker -v
Docker version 25.0.3, build 4debf41
```

Once Docker has started on you local machine, run (make sure to have `cargo-make` installed):

```sh
$ cargo make run
```

This will handle spinning up a Postgres container, initializing the example database, and launching the local dev server.
