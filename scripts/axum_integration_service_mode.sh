#!/bin/sh
export LEPTOS_OUTPUT_NAME="service_mode"
cargo leptos --manifest-path integrations/axum/tests/service_mode/Cargo.toml build
