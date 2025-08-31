#!/bin/bash
# Full Test Suite Execution Script for Leptos Hydration Fix
# This script runs comprehensive tests before and after the fix

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create logs directory
mkdir -p test_logs

echo -e "${BLUE}🧪 Leptos Hydration Fix - Full Test Suite${NC}"
echo "========================================"
echo "Started: $(date)"
echo ""

# Function to log with timestamp
log_with_time() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

# Function to run command with logging
run_with_log() {
    local cmd="$1"
    local logfile="$2"
    local description="$3"
    
    log_with_time "Starting: $description"
    if eval "$cmd" > "test_logs/$logfile" 2>&1; then
        echo -e "${GREEN}✅ $description - PASSED${NC}"
        return 0
    else
        echo -e "${RED}❌ $description - FAILED${NC}"
        echo "   Check test_logs/$logfile for details"
        return 1
    fi
}

# Phase 1: Baseline Compilation Status
echo -e "${BLUE}📋 Phase 1: Baseline Compilation Status${NC}"
echo "----------------------------------------"

run_with_log "cargo --version && rustc --version" "toolchain_info.log" "Toolchain Information"

# Check workspace compilation (expected to fail on 0.8.x before fix)
echo "Checking workspace compilation status..."
if cargo check --workspace --all-features > test_logs/baseline_compilation.log 2>&1; then
    echo -e "${GREEN}✅ Workspace compiles successfully${NC}"
    COMPILATION_STATUS="PASS"
else
    echo -e "${YELLOW}⚠️  Workspace has compilation issues (expected before fix)${NC}"
    COMPILATION_STATUS="FAIL"
    echo "   Check test_logs/baseline_compilation.log for details"
fi

# Phase 2: Feature Flag Testing
echo ""
echo -e "${BLUE}📋 Phase 2: Feature Flag Compilation Tests${NC}"
echo "-------------------------------------------"

# Test each feature flag individually
run_with_log "cargo check --workspace --features csr" "feature_csr_check.log" "CSR Feature Compilation"
run_with_log "cargo check --workspace --features ssr" "feature_ssr_check.log" "SSR Feature Compilation"
run_with_log "cargo check --workspace --features hydrate" "feature_hydrate_check.log" "Hydrate Feature Compilation"

# Phase 3: Unit Tests (where possible)
echo ""
echo -e "${BLUE}📋 Phase 3: Unit Test Execution${NC}"
echo "--------------------------------"

# Test individual crates that should compile
run_with_log "cargo test --package reactive_graph" "reactive_graph_tests.log" "Reactive Graph Tests"
run_with_log "cargo test --package oco" "oco_tests.log" "OCO Utility Tests"
run_with_log "cargo test --package any_spawner" "any_spawner_tests.log" "Any Spawner Tests"
run_with_log "cargo test --package either_of" "either_of_tests.log" "Either Of Tests"

# Try to test leptos_macro (might fail due to hydration issue)
echo "Testing leptos_macro (may fail due to hydration bug)..."
if cargo test --package leptos_macro > test_logs/leptos_macro_tests.log 2>&1; then
    echo -e "${GREEN}✅ Leptos Macro Tests - PASSED${NC}"
else
    echo -e "${YELLOW}⚠️  Leptos Macro Tests - FAILED (may be due to hydration bug)${NC}"
    echo "   Check test_logs/leptos_macro_tests.log for details"
fi

# Test our new hydration validation tests
echo "Testing hydration fix validation tests..."
if cargo test --test hydration_fix_validation > test_logs/hydration_validation_tests.log 2>&1; then
    echo -e "${GREEN}✅ Hydration Validation Tests - PASSED${NC}"
else
    echo -e "${RED}❌ Hydration Validation Tests - FAILED${NC}"
    echo "   This indicates the hydration bug is present"
    echo "   Check test_logs/hydration_validation_tests.log for details"
fi

# Phase 4: Example Compilation Tests
echo ""
echo -e "${BLUE}📋 Phase 4: Example Compilation Tests${NC}"
echo "------------------------------------"

# Test a few key examples
EXAMPLE_DIRS=("counter" "counters" "hackernews" "todo_app_sqlite")

for example in "${EXAMPLE_DIRS[@]}"; do
    if [ -d "examples/$example" ]; then
        echo "Testing examples/$example..."
        cd "examples/$example"
        if cargo check > "../../test_logs/example_${example}_check.log" 2>&1; then
            echo -e "${GREEN}✅ Example $example - PASSED${NC}"
        else
            echo -e "${YELLOW}⚠️  Example $example - FAILED${NC}"
            echo "   Check test_logs/example_${example}_check.log for details"
        fi
        cd "../.."
    else
        echo -e "${YELLOW}⚠️  Example $example not found${NC}"
    fi
done

# Phase 5: Performance Baseline
echo ""
echo -e "${BLUE}📋 Phase 5: Performance Baseline${NC}"
echo "--------------------------------"

# Capture build times for key examples
echo "Capturing build time baselines..."
cd examples/counter
time cargo leptos build --release > "../../test_logs/counter_build_time.log" 2>&1 &
BUILD_PID=$!
cd ../..

# Wait for build to complete
wait $BUILD_PID
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Counter build time captured${NC}"
else
    echo -e "${YELLOW}⚠️  Counter build failed${NC}"
fi

# Phase 6: Generate Summary Report
echo ""
echo -e "${BLUE}📋 Phase 6: Test Summary Report${NC}"
echo "-------------------------------"

# Create comprehensive report
cat > test_logs/TEST_SUMMARY_REPORT.md << EOF
# Leptos Hydration Fix - Test Execution Summary

**Execution Date**: $(date)
**Rust Version**: $(rustc --version)
**Cargo Version**: $(cargo --version)

## Compilation Status
- Workspace compilation: $COMPILATION_STATUS
- CSR feature: $([ -f test_logs/feature_csr_check.log ] && echo "TESTED" || echo "SKIPPED")
- SSR feature: $([ -f test_logs/feature_ssr_check.log ] && echo "TESTED" || echo "SKIPPED")  
- Hydrate feature: $([ -f test_logs/feature_hydrate_check.log ] && echo "TESTED" || echo "SKIPPED")

## Test Results Summary
- Reactive Graph: $(grep -q "test result: ok" test_logs/reactive_graph_tests.log 2>/dev/null && echo "PASS" || echo "FAIL")
- OCO Utility: $(grep -q "test result: ok" test_logs/oco_tests.log 2>/dev/null && echo "PASS" || echo "FAIL")
- Any Spawner: $(grep -q "test result: ok" test_logs/any_spawner_tests.log 2>/dev/null && echo "PASS" || echo "FAIL")
- Either Of: $(grep -q "test result: ok" test_logs/either_of_tests.log 2>/dev/null && echo "PASS" || echo "FAIL")
- Leptos Macro: $(grep -q "test result: ok" test_logs/leptos_macro_tests.log 2>/dev/null && echo "PASS" || echo "FAIL")
- Hydration Validation: $(grep -q "test result: ok" test_logs/hydration_validation_tests.log 2>/dev/null && echo "PASS" || echo "FAIL")

## Example Compilation
EOF

for example in "${EXAMPLE_DIRS[@]}"; do
    if [ -f "test_logs/example_${example}_check.log" ]; then
        STATUS=$(grep -q "error\|failed" test_logs/example_${example}_check.log && echo "FAIL" || echo "PASS")
        echo "- $example: $STATUS" >> test_logs/TEST_SUMMARY_REPORT.md
    fi
done

cat >> test_logs/TEST_SUMMARY_REPORT.md << EOF

## Key Findings
- The hydration bug impacts: $(grep -c "expected.*elements.*found" test_logs/*.log 2>/dev/null || echo "0") files
- Critical failing tests: $(grep -c "test result: FAILED" test_logs/hydration_validation_tests.log 2>/dev/null || echo "0")

## Next Steps
1. Implement the hydration fix in leptos_macro/src/view/mod.rs
2. Re-run this test suite to validate the fix
3. Proceed with integration and performance testing

---
**Test Suite Execution Complete**
EOF

echo "Test execution completed: $(date)"
echo ""
echo -e "${GREEN}📊 Summary Report Generated: test_logs/TEST_SUMMARY_REPORT.md${NC}"
echo ""
echo -e "${BLUE}🎯 Key Next Steps:${NC}"
echo "1. Review test logs in test_logs/ directory"
echo "2. Implement hydration fix if tests show compilation failures"
echo "3. Re-run this script after implementing the fix"
echo "4. Compare before/after results"

# Show critical errors if any
if grep -q "expected.*elements.*found" test_logs/*.log 2>/dev/null; then
    echo ""
    echo -e "${RED}🚨 Critical Hydration Errors Detected:${NC}"
    grep -n "expected.*elements.*found" test_logs/*.log 2>/dev/null | head -5
fi

echo ""
echo -e "${BLUE}Test suite execution complete!${NC}"