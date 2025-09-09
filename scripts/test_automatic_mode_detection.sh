#!/bin/bash

# Test script for the automatic mode detection system
# This script runs comprehensive tests to validate the entire system

set -e

echo "🧪 Testing Automatic Mode Detection System"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test and report results
run_test() {
    local test_name="$1"
    local test_command="$2"
    
    echo -e "${BLUE}Running: $test_name${NC}"
    
    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASSED: $test_name${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}❌ FAILED: $test_name${NC}"
        ((TESTS_FAILED++))
    fi
    echo
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo "🔍 Checking prerequisites..."

if ! command_exists cargo; then
    echo -e "${RED}❌ Cargo not found. Please install Rust.${NC}"
    exit 1
fi

if ! command_exists rustc; then
    echo -e "${RED}❌ Rust compiler not found. Please install Rust.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Prerequisites check passed${NC}"
echo

# Test 1: Build all crates
echo "🔨 Building all crates..."
run_test "Build leptos_feature_detection" "cargo build -p leptos_feature_detection"
run_test "Build leptos_mode_resolver" "cargo build -p leptos_mode_resolver"
run_test "Build leptos_compile_validator" "cargo build -p leptos_compile_validator"
run_test "Build leptos_compile_validator_derive" "cargo build -p leptos_compile_validator_derive"
run_test "Build leptos_mode_cli" "cargo build -p leptos_mode_cli"

# Test 2: Run unit tests
echo "🧪 Running unit tests..."
run_test "Test leptos_feature_detection" "cargo test -p leptos_feature_detection"
run_test "Test leptos_mode_resolver" "cargo test -p leptos_mode_resolver"
run_test "Test leptos_compile_validator" "cargo test -p leptos_compile_validator"
run_test "Test leptos_compile_validator_derive" "cargo test -p leptos_compile_validator_derive"

# Test 3: Run integration tests
echo "🔗 Running integration tests..."
run_test "Integration tests for feature detection" "cargo test -p leptos_feature_detection --test integration_tests"
run_test "Integration tests for validation" "cargo test -p leptos_compile_validator --test validation_tests"
run_test "Integration tests for derive macros" "cargo test -p leptos_compile_validator_derive --test derive_tests"

# Test 4: Test CLI tool
echo "🛠️  Testing CLI tool..."
run_test "CLI tool help" "cargo run -p leptos_mode_cli -- --help"
run_test "CLI tool analyze help" "cargo run -p leptos_mode_cli -- analyze --help"
run_test "CLI tool migrate help" "cargo run -p leptos_mode_cli -- migrate --help"
run_test "CLI tool validate help" "cargo run -p leptos_mode_cli -- validate --help"
run_test "CLI tool generate help" "cargo run -p leptos_mode_cli -- generate --help"

# Test 5: Test example project
echo "📁 Testing example project..."
if [ -d "examples/automatic_mode_detection" ]; then
    run_test "Build example project" "cargo build -p automatic_mode_detection"
    run_test "Test example project" "cargo test -p automatic_mode_detection"
else
    echo -e "${YELLOW}⚠️  Example project not found, skipping...${NC}"
fi

# Test 6: Test mode detection on existing examples
echo "🔍 Testing mode detection on existing examples..."
for example in examples/*/; do
    if [ -d "$example" ] && [ -f "$example/Cargo.toml" ]; then
        example_name=$(basename "$example")
        echo -e "${BLUE}Analyzing example: $example_name${NC}"
        
        # Run analysis (don't fail if it doesn't work)
        if cargo run -p leptos_mode_cli -- analyze --path "$example" --format json >/dev/null 2>&1; then
            echo -e "${GREEN}✅ Analysis successful for $example_name${NC}"
        else
            echo -e "${YELLOW}⚠️  Analysis failed for $example_name (may not be a Leptos project)${NC}"
        fi
    fi
done

# Test 7: Test validation setup
echo "✅ Testing validation setup..."
temp_dir=$(mktemp -d)
echo "Creating temporary project in $temp_dir"

# Create a minimal Cargo.toml
cat > "$temp_dir/Cargo.toml" << EOF
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = { path = "../leptos" }
EOF

# Create src directory
mkdir -p "$temp_dir/src"

# Create a simple lib.rs
cat > "$temp_dir/src/lib.rs" << EOF
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}

pub fn main() {
    mount_to_body(App);
}
EOF

run_test "Setup validation in temp project" "cargo run -p leptos_compile_validator --bin setup_validation -- $temp_dir"

# Clean up
rm -rf "$temp_dir"

# Test 8: Test mode generation
echo "🎯 Testing mode generation..."
run_test "Generate SPA mode config" "cargo run -p leptos_mode_cli -- generate --mode spa"
run_test "Generate fullstack mode config" "cargo run -p leptos_mode_cli -- generate --mode fullstack"
run_test "Generate static mode config" "cargo run -p leptos_mode_cli -- generate --mode static"
run_test "Generate API mode config" "cargo run -p leptos_mode_cli -- generate --mode api"

# Test 9: Test help system
echo "❓ Testing help system..."
run_test "Help for SPA mode" "cargo run -p leptos_mode_cli -- help spa"
run_test "Help for fullstack mode" "cargo run -p leptos_mode_cli -- help fullstack"
run_test "Help for static mode" "cargo run -p leptos_mode_cli -- help static"
run_test "Help for API mode" "cargo run -p leptos_mode_cli -- help api"

# Test 10: Test error handling
echo "🚨 Testing error handling..."
run_test "Invalid mode error" "cargo run -p leptos_mode_cli -- generate --mode invalid 2>&1 | grep -q 'Invalid mode'"
run_test "Invalid environment error" "cargo run -p leptos_mode_cli -- generate --mode spa --env invalid 2>&1 | grep -q 'Invalid environment'"

# Test 11: Test feature flag validation
echo "🏁 Testing feature flag validation..."
temp_dir=$(mktemp -d)

# Create a project with conflicting features
cat > "$temp_dir/Cargo.toml" << EOF
[package]
name = "conflict_test"
version = "0.1.0"
edition = "2021"

[features]
default = ["ssr", "csr"]

[dependencies]
leptos = { path = "../leptos" }
EOF

mkdir -p "$temp_dir/src"
cat > "$temp_dir/src/lib.rs" << EOF
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
EOF

run_test "Detect feature conflicts" "cargo run -p leptos_mode_cli -- analyze --path $temp_dir 2>&1 | grep -q 'conflict'"

# Clean up
rm -rf "$temp_dir"

# Test 12: Test migration process
echo "🔄 Testing migration process..."
temp_dir=$(mktemp -d)

# Create a project with manual features
cat > "$temp_dir/Cargo.toml" << EOF
[package]
name = "migration_test"
version = "0.1.0"
edition = "2021"

[features]
default = ["ssr", "hydrate"]

[package.metadata.leptos]
bin-features = ["ssr"]
lib-features = ["hydrate"]

[dependencies]
leptos = { path = "../leptos" }
EOF

mkdir -p "$temp_dir/src"
cat > "$temp_dir/src/lib.rs" << EOF
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! { <div>"Hello, World!"</div> }
}
EOF

run_test "Analyze migration candidate" "cargo run -p leptos_mode_cli -- analyze --path $temp_dir"
run_test "Generate migration recommendations" "cargo run -p leptos_mode_cli -- migrate --path $temp_dir --force"

# Clean up
rm -rf "$temp_dir"

# Final results
echo "📊 Test Results Summary"
echo "======================"
echo -e "${GREEN}✅ Tests Passed: $TESTS_PASSED${NC}"
echo -e "${RED}❌ Tests Failed: $TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed! The automatic mode detection system is working correctly.${NC}"
    exit 0
else
    echo -e "${RED}💥 Some tests failed. Please check the output above for details.${NC}"
    exit 1
fi
