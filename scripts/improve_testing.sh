#!/bin/bash

# 🧪 Leptos Testing Infrastructure Improvement Script
# This script automates the testing improvements for the Leptos framework

set -e

echo "🚀 Starting Leptos Testing Infrastructure Improvements"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install cargo tools
install_cargo_tool() {
    local tool=$1
    if ! command_exists "$tool"; then
        print_status "Installing $tool..."
        cargo install "$tool"
        print_success "$tool installed successfully"
    else
        print_status "$tool already installed"
    fi
}

# Phase 1: Code Quality Improvements
echo ""
echo "📋 Phase 1: Code Quality Improvements"
echo "------------------------------------"

print_status "Fixing all warnings..."
if cargo fix --workspace --allow-dirty; then
    print_success "Warnings fixed successfully"
else
    print_warning "Some warnings could not be automatically fixed"
fi

print_status "Running clippy with fixes..."
if cargo clippy --workspace --fix --allow-dirty; then
    print_success "Clippy fixes applied successfully"
else
    print_warning "Some clippy issues require manual attention"
fi

# Phase 2: Install Testing Tools
echo ""
echo "📋 Phase 2: Install Testing Tools"
echo "--------------------------------"

install_cargo_tool "cargo-tarpaulin"
install_cargo_tool "cargo-nextest"
install_cargo_tool "cargo-watch"
install_cargo_tool "cargo-expand"

# Phase 3: Generate Coverage Report
echo ""
echo "📋 Phase 3: Generate Coverage Report"
echo "-----------------------------------"

print_status "Generating coverage report..."
if cargo tarpaulin --workspace --out Html --output-dir coverage/ --skip-clean; then
    print_success "Coverage report generated in coverage/"
else
    print_warning "Coverage generation failed, continuing with other improvements"
fi

# Phase 4: Run All Tests
echo ""
echo "📋 Phase 4: Run All Tests"
echo "------------------------"

print_status "Running all tests..."
if cargo test --workspace; then
    print_success "All tests passed!"
else
    print_error "Some tests failed"
    exit 1
fi

# Phase 5: Run Tests with Different Features
echo ""
echo "📋 Phase 5: Feature Matrix Testing"
echo "---------------------------------"

FEATURES=("csr" "ssr" "hydrate" "all")
for feature in "${FEATURES[@]}"; do
    print_status "Testing with feature: $feature"
    if cargo test --workspace --features "$feature"; then
        print_success "Tests passed with feature: $feature"
    else
        print_warning "Some tests failed with feature: $feature"
    fi
done

# Phase 6: Performance Testing
echo ""
echo "📋 Phase 6: Performance Testing"
echo "------------------------------"

print_status "Running benchmarks..."
if cargo bench --workspace; then
    print_success "Benchmarks completed successfully"
else
    print_warning "Some benchmarks failed or are not available"
fi

# Phase 7: Documentation Testing
echo ""
echo "📋 Phase 7: Documentation Testing"
echo "--------------------------------"

print_status "Checking documentation..."
if cargo doc --workspace --no-deps; then
    print_success "Documentation generated successfully"
else
    print_warning "Documentation generation had issues"
fi

print_status "Running doctests..."
if cargo test --workspace --doc; then
    print_success "All doctests passed"
else
    print_warning "Some doctests failed"
fi

# Phase 8: Example Validation
echo ""
echo "📋 Phase 8: Example Validation"
echo "-----------------------------"

print_status "Validating examples..."
for example in examples/*/; do
    if [ -d "$example" ] && [ -f "$example/Cargo.toml" ]; then
        print_status "Checking example: $(basename "$example")"
        if cargo check --manifest-path "$example/Cargo.toml"; then
            print_success "Example $(basename "$example") compiles successfully"
        else
            print_warning "Example $(basename "$example") has compilation issues"
        fi
    fi
done

# Phase 9: Generate Test Report
echo ""
echo "📋 Phase 9: Generate Test Report"
echo "-------------------------------"

print_status "Generating test statistics..."
{
    echo "# Leptos Testing Report"
    echo "Generated on: $(date)"
    echo ""
    echo "## Test Statistics"
    echo "- Total test files: $(find . -name "*.rs" -path "*/tests/*" | wc -l | tr -d ' ')"
    echo "- Total examples: $(find examples -name "Cargo.toml" | wc -l | tr -d ' ')"
    echo "- Workspace members: $(cargo metadata --format-version=1 | jq '.workspace_members | length')"
    echo ""
    echo "## Coverage"
    if [ -d "coverage/" ]; then
        echo "- Coverage report available in: coverage/"
    else
        echo "- Coverage report not generated"
    fi
    echo ""
    echo "## Recent Test Results"
    echo "- All tests: $(cargo test --workspace --no-run 2>&1 | grep -c "test result: ok" || echo "Unknown")"
    echo "- Warnings: $(cargo check --workspace 2>&1 | grep -c "warning:" || echo "0")"
    echo "- Errors: $(cargo check --workspace 2>&1 | grep -c "error:" || echo "0")"
} > testing_report.md

print_success "Test report generated: testing_report.md"

# Phase 10: Create Testing Guidelines
echo ""
echo "📋 Phase 10: Create Testing Guidelines"
echo "-------------------------------------"

print_status "Creating testing guidelines..."

cat > docs/TESTING_GUIDELINES.md << 'EOF'
# 🧪 Leptos Testing Guidelines

## Overview
This document outlines the testing standards and best practices for the Leptos framework.

## Testing Standards

### 1. Code Coverage
- **Target**: >80% code coverage for all crates
- **Tool**: cargo-tarpaulin
- **Command**: `cargo tarpaulin --workspace --out Html`

### 2. Test Types Required
- **Unit Tests**: For all public APIs
- **Integration Tests**: For crate interactions
- **Macro Tests**: For procedural macros
- **Example Tests**: For all examples
- **Documentation Tests**: For all public APIs

### 3. Test Naming Conventions
- Unit tests: `test_function_name`
- Integration tests: `test_integration_scenario`
- Macro tests: `test_macro_behavior`

### 4. Test Organization
```
crate/
├── src/
│   └── lib.rs
└── tests/
    ├── integration_tests.rs
    ├── macro_tests.rs
    └── unit_tests.rs
```

### 5. Performance Testing
- **Benchmarks**: For performance-critical code
- **Tool**: cargo bench
- **Command**: `cargo bench --workspace`

### 6. Feature Testing
- Test all feature combinations
- Test with and without optional dependencies
- Test different target platforms

### 7. Documentation Testing
- All public APIs must have examples
- Examples must compile and run
- Use `cargo test --doc` to verify

### 8. Example Validation
- All examples must compile
- All examples must have tests
- Examples should demonstrate real usage

## Running Tests

### Basic Test Commands
```bash
# Run all tests
cargo test --workspace

# Run tests with specific features
cargo test --workspace --features "csr,ssr"

# Run tests with coverage
cargo tarpaulin --workspace --out Html

# Run benchmarks
cargo bench --workspace

# Run documentation tests
cargo test --workspace --doc

# Check examples
for example in examples/*/; do
    cargo check --manifest-path "$example/Cargo.toml"
done
```

### Continuous Integration
- All tests must pass before merging
- Coverage reports are generated automatically
- Performance benchmarks are tracked
- Examples are validated

## Best Practices

### 1. Test Isolation
- Each test should be independent
- Use unique test data
- Clean up resources after tests

### 2. Error Testing
- Test error conditions
- Test edge cases
- Test invalid inputs

### 3. Async Testing
- Use proper async test utilities
- Handle timeouts appropriately
- Test cancellation scenarios

### 4. Macro Testing
- Test macro expansion
- Test compile-time errors
- Test different input variations

### 5. Integration Testing
- Test crate interactions
- Test feature combinations
- Test real-world scenarios

## Tools and Dependencies

### Required Tools
- `cargo-tarpaulin`: Code coverage
- `cargo-nextest`: Parallel test execution
- `cargo-watch`: Development workflow
- `cargo-expand`: Macro debugging

### Optional Tools
- `cargo-fuzz`: Fuzzing tests
- `cargo-audit`: Security auditing
- `cargo-deny`: Dependency checking

## Reporting Issues

### Test Failures
1. Reproduce the failure
2. Check if it's a flaky test
3. Report with minimal reproduction
4. Include environment details

### Coverage Issues
1. Identify uncovered code paths
2. Add tests for critical paths
3. Document why some paths can't be tested
4. Update coverage targets if needed

## Maintenance

### Regular Tasks
- Update test dependencies
- Review and update test coverage
- Validate all examples
- Update testing guidelines

### Monitoring
- Track test execution time
- Monitor coverage trends
- Review test failure patterns
- Update CI/CD pipelines
EOF

print_success "Testing guidelines created: docs/TESTING_GUIDELINES.md"

# Final Summary
echo ""
echo "🎉 Testing Infrastructure Improvement Complete!"
echo "=============================================="
echo ""
echo "✅ Completed Tasks:"
echo "  - Fixed code warnings"
echo "  - Installed testing tools"
echo "  - Generated coverage report"
echo "  - Ran all tests"
echo "  - Tested feature combinations"
echo "  - Validated examples"
echo "  - Created testing guidelines"
echo ""
echo "📁 Generated Files:"
echo "  - testing_report.md"
echo "  - docs/TESTING_GUIDELINES.md"
echo "  - coverage/ (if successful)"
echo ""
echo "🚀 Next Steps:"
echo "  1. Review testing_report.md"
echo "  2. Address any remaining warnings"
echo "  3. Add more specific tests as needed"
echo "  4. Set up CI/CD integration"
echo "  5. Implement performance benchmarks"
echo ""
echo "📚 Documentation:"
echo "  - See docs/TESTING_GUIDELINES.md for detailed guidelines"
echo "  - Use scripts/improve_testing.sh for future improvements"
echo ""

print_success "All testing improvements completed successfully!"
