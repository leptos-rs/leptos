#!/bin/bash
# Hydration-Specific Fix Validation Script
# Focused testing for the tuple mismatch bug

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create hydration-specific logs directory
mkdir -p hydration_logs

echo -e "${BLUE}🔧 Leptos Hydration Fix Validation${NC}"
echo "=================================="
echo "Started: $(date)"
echo ""

# Function to run hydration-specific test
test_hydration_fix() {
    local test_name="$1"
    local description="$2"
    
    echo "Testing: $description"
    
    if cargo test "$test_name" --test hydration_fix_validation > "hydration_logs/${test_name}.log" 2>&1; then
        echo -e "${GREEN}✅ $description - PASSED${NC}"
        return 0
    else
        echo -e "${RED}❌ $description - FAILED${NC}"
        echo "   Check hydration_logs/${test_name}.log for details"
        
        # Show specific error if it's the tuple mismatch
        if grep -q "expected.*elements.*found" "hydration_logs/${test_name}.log"; then
            echo -e "${YELLOW}   → Tuple mismatch error detected (as expected before fix)${NC}"
        fi
        return 1
    fi
}

echo -e "${BLUE}📋 Testing Individual View Patterns${NC}"
echo "-----------------------------------"

# Test each specific case
test_hydration_fix "test_empty_view" "Empty View"
test_hydration_fix "test_single_element_view" "Single Element View"
test_hydration_fix "test_two_element_view" "Two Element View"
test_hydration_fix "test_three_element_view" "Three Element View (Target)"
test_hydration_fix "test_five_element_view" "Five Element View (CRITICAL - Current Failure)"
test_hydration_fix "test_large_view" "Large View (20+ elements)"
test_hydration_fix "test_mixed_content_view" "Mixed Content View"
test_hydration_fix "test_nested_components" "Nested Components"

echo ""
echo -e "${BLUE}📋 Testing Feature Flag Compatibility${NC}"
echo "------------------------------------"

# Test feature-specific scenarios
if command -v cargo &> /dev/null; then
    # Test CSR feature
    echo "Testing CSR feature hydration fix..."
    if cargo test test_csr_five_elements --test hydration_fix_validation --features csr > hydration_logs/csr_test.log 2>&1; then
        echo -e "${GREEN}✅ CSR Feature - PASSED${NC}"
    else
        echo -e "${RED}❌ CSR Feature - FAILED${NC}"
    fi
    
    # Test SSR feature  
    echo "Testing SSR feature hydration fix..."
    if cargo test test_ssr_five_elements --test hydration_fix_validation --features ssr > hydration_logs/ssr_test.log 2>&1; then
        echo -e "${GREEN}✅ SSR Feature - PASSED${NC}"
    else
        echo -e "${RED}❌ SSR Feature - FAILED${NC}"
    fi
    
    # Test Hydrate feature (most critical)
    echo "Testing Hydrate feature hydration fix..."
    if cargo test test_hydrate_five_elements --test hydration_fix_validation --features hydrate > hydration_logs/hydrate_test.log 2>&1; then
        echo -e "${GREEN}✅ Hydrate Feature - PASSED${NC}"
    else
        echo -e "${RED}❌ Hydrate Feature - FAILED (CRITICAL)${NC}"
    fi
fi

echo ""
echo -e "${BLUE}📋 Testing Integration Scenarios${NC}"
echo "-------------------------------"

test_hydration_fix "test_hydration_mod_scenario" "Hydration Module Scenario"
test_hydration_fix "test_leptos_state_compatibility" "Leptos-State Compatibility"

echo ""
echo -e "${BLUE}📋 Macro Expansion Analysis${NC}"
echo "---------------------------"

# Test macro expansion if cargo-expand is available
if command -v cargo-expand &> /dev/null; then
    echo "Analyzing macro expansion for 5-element view..."
    
    # Create a temporary test file for macro expansion
    cat > hydration_logs/macro_test.rs << 'EOF'
use leptos::prelude::*;

fn test_expansion() {
    let _view = view! {
        <link rel="modulepreload" href="test1.js" />
        <link rel="preload" href="test2.css" as="style" />
        <script type="module" src="test.js"></script>
        <style>/* styles */</style>
        <meta name="viewport" content="width=device-width" />
    };
}
EOF
    
    if cargo expand --bin macro_test > hydration_logs/macro_expansion.rs 2>&1; then
        echo -e "${GREEN}✅ Macro expansion captured${NC}"
        echo "   Check hydration_logs/macro_expansion.rs for details"
    else
        echo -e "${YELLOW}⚠️  Macro expansion failed (expected)${NC}"
    fi
fi

# Generate detailed hydration report
echo ""
echo -e "${BLUE}📊 Generating Hydration Fix Report${NC}"
echo "--------------------------------"

cat > hydration_logs/HYDRATION_FIX_REPORT.md << EOF
# Hydration Fix Validation Report

**Generated**: $(date)
**Test Suite**: Hydration-specific validation tests

## Test Results Summary

### Core View Pattern Tests
EOF

# Count passed/failed tests
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

for logfile in hydration_logs/*.log; do
    if [ -f "$logfile" ]; then
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        if grep -q "test result: ok" "$logfile" 2>/dev/null; then
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
    fi
done

cat >> hydration_logs/HYDRATION_FIX_REPORT.md << EOF
- **Total Tests**: $TOTAL_TESTS
- **Passed**: $PASSED_TESTS  
- **Failed**: $FAILED_TESTS
- **Success Rate**: $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%

### Critical Findings

EOF

# Check for the specific tuple mismatch error
TUPLE_ERRORS=$(grep -c "expected.*elements.*found" hydration_logs/*.log 2>/dev/null || echo "0")
cat >> hydration_logs/HYDRATION_FIX_REPORT.md << EOF
- **Tuple Mismatch Errors**: $TUPLE_ERRORS instances detected
- **Critical Test Status**: $(grep -q "test result: ok" hydration_logs/test_five_element_view.log 2>/dev/null && echo "PASSING" || echo "FAILING - Fix Required")

### Feature Flag Compatibility
- **CSR Mode**: $(grep -q "test result: ok" hydration_logs/csr_test.log 2>/dev/null && echo "COMPATIBLE" || echo "ISSUES DETECTED")
- **SSR Mode**: $(grep -q "test result: ok" hydration_logs/ssr_test.log 2>/dev/null && echo "COMPATIBLE" || echo "ISSUES DETECTED")  
- **Hydrate Mode**: $(grep -q "test result: ok" hydration_logs/hydrate_test.log 2>/dev/null && echo "COMPATIBLE" || echo "ISSUES DETECTED")

## Detailed Error Analysis

### Most Common Error Pattern
EOF

# Find the most common error
if [ -f hydration_logs/test_five_element_view.log ]; then
    echo "#### Five Element View Test Output:" >> hydration_logs/HYDRATION_FIX_REPORT.md
    echo '```' >> hydration_logs/HYDRATION_FIX_REPORT.md
    grep -A 5 -B 5 "error\|expected.*found" hydration_logs/test_five_element_view.log | head -10 >> hydration_logs/HYDRATION_FIX_REPORT.md
    echo '```' >> hydration_logs/HYDRATION_FIX_REPORT.md
fi

cat >> hydration_logs/HYDRATION_FIX_REPORT.md << EOF

## Fix Implementation Status
$(if [ $FAILED_TESTS -gt 0 ]; then
    echo "❌ **Fix Required**: $FAILED_TESTS test(s) are failing"
    echo ""
    echo "**Recommended Action**: Implement the hydration fix in:"
    echo "- \`leptos_macro/src/view/mod.rs\` (fragment_to_tokens function)"
    echo "- Lines 614-617: Modify tuple generation logic"
else
    echo "✅ **Fix Complete**: All hydration tests are passing"
fi)

## Next Steps
1. $([ $FAILED_TESTS -gt 0 ] && echo "Implement the tuple generation fix" || echo "Proceed to integration testing")
2. $([ $FAILED_TESTS -gt 0 ] && echo "Re-run this validation script" || echo "Run full regression test suite")
3. $([ $FAILED_TESTS -gt 0 ] && echo "Validate fix across all feature flags" || echo "Prepare for production deployment")

---
**Report Generated**: $(date)
EOF

echo ""
echo -e "${GREEN}📊 Hydration validation complete!${NC}"
echo "Report generated: hydration_logs/HYDRATION_FIX_REPORT.md"

# Summary
if [ $FAILED_TESTS -gt 0 ]; then
    echo ""
    echo -e "${RED}🚨 HYDRATION FIX REQUIRED${NC}"
    echo "- Failed tests: $FAILED_TESTS/$TOTAL_TESTS"
    echo "- Tuple mismatch errors: $TUPLE_ERRORS"
    echo "- Next step: Implement the fix in leptos_macro/src/view/mod.rs"
else
    echo ""
    echo -e "${GREEN}🎉 HYDRATION FIX VALIDATED${NC}"
    echo "- All tests passing: $PASSED_TESTS/$TOTAL_TESTS"
    echo "- Ready for integration testing"
fi

echo ""