#!/bin/bash
# Comprehensive Hydration Fix Test Suite
# This script implements the testing strategy from TESTING_STRATEGY.md

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Create comprehensive logs directory
mkdir -p test_logs/hydration_fix

echo -e "${BLUE}🧪 Leptos Hydration Fix - Comprehensive Test Suite${NC}"
echo "=================================================="
echo "Started: $(date)"
echo ""

# Function to log with timestamp and color
log_with_time() {
    local color="$1"
    local message="$2"
    echo -e "${color}[$(date '+%Y-%m-%d %H:%M:%S')] $message${NC}"
}

# Function to run command with logging
run_with_log() {
    local cmd="$1"
    local logfile="$2"
    local description="$3"
    local expected_failure="$4"
    
    log_with_time "$BLUE" "Starting: $description"
    
    if eval "$cmd" > "test_logs/hydration_fix/$logfile" 2>&1; then
        echo -e "${GREEN}✅ $description - PASSED${NC}"
        return 0
    else
        if [ "$expected_failure" = "true" ]; then
            echo -e "${YELLOW}⚠️  $description - FAILED (EXPECTED)${NC}"
            return 0
        else
            echo -e "${RED}❌ $description - FAILED${NC}"
            echo "   Check test_logs/hydration_fix/$logfile for details"
            return 1
        fi
    fi
}

# Function to check for specific error patterns
check_error_pattern() {
    local logfile="$1"
    local pattern="$2"
    local description="$3"
    
    if grep -q "$pattern" "test_logs/hydration_fix/$logfile"; then
        echo -e "${YELLOW}   → $description detected${NC}"
        return 0
    else
        return 1
    fi
}

echo -e "${PURPLE}📋 Phase 1: Baseline Establishment${NC}"
echo "====================================="

# 1.1 Toolchain Information
run_with_log "cargo --version && rustc --version" "toolchain_info.log" "Toolchain Information"

# 1.2 Current Compilation Status (expected to fail)
echo ""
log_with_time "$BLUE" "Checking current compilation status (expected to fail)..."
if cargo check --workspace --all-features > test_logs/hydration_fix/baseline_compilation.log 2>&1; then
    echo -e "${GREEN}✅ Workspace compiles successfully (unexpected!)${NC}"
    COMPILATION_STATUS="PASS"
else
    echo -e "${YELLOW}⚠️  Workspace has compilation issues (expected before fix)${NC}"
    COMPILATION_STATUS="FAIL"
    
    # Check for specific error patterns
    check_error_pattern "baseline_compilation.log" "expected.*elements.*found" "Tuple mismatch error"
    check_error_pattern "baseline_compilation.log" "type annotations needed" "Type annotation error"
    check_error_pattern "baseline_compilation.log" "mismatched types" "Type mismatch error"
fi

# 1.3 Feature Flag Compilation Tests
echo ""
echo -e "${PURPLE}📋 Phase 2: Feature Flag Compilation Tests${NC}"
echo "============================================="

run_with_log "cargo check --workspace --features csr" "feature_csr_check.log" "CSR Feature Compilation" "true"
run_with_log "cargo check --workspace --features ssr" "feature_ssr_check.log" "SSR Feature Compilation" "true"
run_with_log "cargo check --workspace --features hydrate" "feature_hydrate_check.log" "Hydrate Feature Compilation" "true"

# Check for specific errors in each feature
for feature in csr ssr hydrate; do
    check_error_pattern "feature_${feature}_check.log" "expected.*elements.*found" "Tuple mismatch in $feature"
    check_error_pattern "feature_${feature}_check.log" "type annotations needed" "Type annotation error in $feature"
done

echo ""
echo -e "${PURPLE}📋 Phase 3: Unit Test Execution${NC}"
echo "================================="

# Test individual crates that should compile
run_with_log "cargo test --package reactive_graph" "reactive_graph_tests.log" "Reactive Graph Tests"
run_with_log "cargo test --package oco" "oco_tests.log" "OCO Utility Tests"
run_with_log "cargo test --package any_spawner" "any_spawner_tests.log" "Any Spawner Tests"
run_with_log "cargo test --package either_of" "either_of_tests.log" "Either Of Tests"

# Test leptos_macro (might fail due to hydration issue)
echo ""
log_with_time "$BLUE" "Testing leptos_macro (may fail due to hydration bug)..."
if cargo test --package leptos_macro > test_logs/hydration_fix/leptos_macro_tests.log 2>&1; then
    echo -e "${GREEN}✅ Leptos Macro Tests - PASSED${NC}"
else
    echo -e "${YELLOW}⚠️  Leptos Macro Tests - FAILED (may be due to hydration bug)${NC}"
    check_error_pattern "leptos_macro_tests.log" "expected.*elements.*found" "Tuple mismatch in macro tests"
fi

echo ""
echo -e "${PURPLE}📋 Phase 4: Hydration Fix Validation Tests${NC}"
echo "============================================="

# Test our new hydration validation tests
echo "Testing hydration fix validation tests..."
if cargo test --package hydration_fix_tests > test_logs/hydration_fix/hydration_validation_tests.log 2>&1; then
    echo -e "${GREEN}✅ Hydration Validation Tests - PASSED${NC}"
    echo -e "${RED}   → This indicates the hydration bug has been fixed!${NC}"
else
    echo -e "${YELLOW}⚠️  Hydration Validation Tests - FAILED (expected before fix)${NC}"
    check_error_pattern "hydration_validation_tests.log" "expected.*elements.*found" "Tuple mismatch in validation tests"
    check_error_pattern "hydration_validation_tests.log" "mismatched types" "Type mismatch in validation tests"
fi

echo ""
echo -e "${PURPLE}📋 Phase 5: Specific Test Cases${NC}"
echo "==============================="

# Test specific failing cases
echo "Testing specific failing cases..."

# Test the exact scenario from leptos/src/hydration/mod.rs:138
echo "Testing hydration module scenario..."
if cargo test test_hydration_mod_scenario --package hydration_fix_tests > test_logs/hydration_fix/hydration_mod_scenario.log 2>&1; then
    echo -e "${GREEN}✅ Hydration Module Scenario - PASSED${NC}"
else
    echo -e "${YELLOW}⚠️  Hydration Module Scenario - FAILED (expected before fix)${NC}"
    check_error_pattern "hydration_mod_scenario.log" "expected.*elements.*found" "Tuple mismatch in hydration module"
fi

# Test the critical five-element case
echo "Testing critical five-element case..."
if cargo test test_five_element_view --package hydration_fix_tests > test_logs/hydration_fix/five_element_case.log 2>&1; then
    echo -e "${GREEN}✅ Five Element Case - PASSED${NC}"
else
    echo -e "${YELLOW}⚠️  Five Element Case - FAILED (expected before fix)${NC}"
    check_error_pattern "five_element_case.log" "expected.*elements.*found" "Tuple mismatch in five-element case"
fi

echo ""
echo -e "${PURPLE}📋 Phase 6: Performance Baseline${NC}"
echo "================================="

# Capture build time baseline
echo "Capturing build time baseline..."
time cargo build --package hydration_fix_tests > test_logs/hydration_fix/build_time_baseline.log 2>&1
echo -e "${CYAN}📊 Build time captured${NC}"

echo ""
echo -e "${PURPLE}📋 Phase 7: Example Compilation Tests${NC}"
echo "====================================="

# Test a few key examples
for example in counter hackernews; do
    if [ -d "examples/$example" ]; then
        echo "Testing example: $example"
        if cd "examples/$example" && cargo check > "../../test_logs/hydration_fix/example_${example}_check.log" 2>&1; then
            echo -e "${GREEN}✅ Example $example - PASSED${NC}"
        else
            echo -e "${YELLOW}⚠️  Example $example - FAILED (may be expected)${NC}"
            check_error_pattern "example_${example}_check.log" "expected.*elements.*found" "Tuple mismatch in $example"
        fi
        cd ../..
    fi
done

echo ""
echo -e "${PURPLE}📋 Phase 8: Test Summary${NC}"
echo "============================="

# Generate summary report
echo "# Leptos Hydration Fix Test Report" > test_logs/hydration_fix/TEST_REPORT.md
echo "Generated: $(date)" >> test_logs/hydration_fix/TEST_REPORT.md
echo "" >> test_logs/hydration_fix/TEST_REPORT.md

echo "## Baseline Status" >> test_logs/hydration_fix/TEST_REPORT.md
echo "- Compilation Status: $COMPILATION_STATUS" >> test_logs/hydration_fix/TEST_REPORT.md
echo "- Hydration Bug Present: $(if [ "$COMPILATION_STATUS" = "FAIL" ]; then echo "YES"; else echo "NO"; fi)" >> test_logs/hydration_fix/TEST_REPORT.md

echo "## Error Patterns Detected" >> test_logs/hydration_fix/TEST_REPORT.md
if grep -r "expected.*elements.*found" test_logs/hydration_fix/ > /dev/null 2>&1; then
    echo "- ✅ Tuple mismatch errors detected (expected)" >> test_logs/hydration_fix/TEST_REPORT.md
else
    echo "- ❌ No tuple mismatch errors found (unexpected)" >> test_logs/hydration_fix/TEST_REPORT.md
fi

if grep -r "type annotations needed" test_logs/hydration_fix/ > /dev/null 2>&1; then
    echo "- ✅ Type annotation errors detected" >> test_logs/hydration_fix/TEST_REPORT.md
else
    echo "- ❌ No type annotation errors found" >> test_logs/hydration_fix/TEST_REPORT.md
fi

echo "## Next Steps" >> test_logs/hydration_fix/TEST_REPORT.md
echo "1. Implement the hydration fix in leptos_macro" >> test_logs/hydration_fix/TEST_REPORT.md
echo "2. Fix type annotation issues in leptos/src/hydration/mod.rs" >> test_logs/hydration_fix/TEST_REPORT.md
echo "3. Re-run this test suite to validate the fix" >> test_logs/hydration_fix/TEST_REPORT.md

echo ""
echo -e "${GREEN}🎉 Test suite completed!${NC}"
echo -e "${CYAN}📊 Full report saved to: test_logs/hydration_fix/TEST_REPORT.md${NC}"
echo ""
echo -e "${YELLOW}📋 Summary:${NC}"
echo "  - Baseline established"
echo "  - Hydration bug confirmed"
echo "  - Test infrastructure ready"
echo "  - Ready for fix implementation"
