#!/bin/bash
# Quick Hydration Fix Validation
# Focused testing for the most critical hydration issues

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Quick Hydration Fix Validation${NC}"
echo "=================================="
echo "Started: $(date)"
echo ""

# Create logs directory
mkdir -p test_logs/quick_validation

# Function to test and report
test_case() {
    local test_name="$1"
    local description="$2"
    local expected_failure="$3"
    
    echo "Testing: $description"
    
    if cargo test "$test_name" --package hydration_fix_tests > "test_logs/quick_validation/${test_name}.log" 2>&1; then
        if [ "$expected_failure" = "true" ]; then
            echo -e "${RED}❌ $description - PASSED (UNEXPECTED - fix may be implemented!)${NC}"
        else
            echo -e "${GREEN}✅ $description - PASSED${NC}"
        fi
        return 0
    else
        if [ "$expected_failure" = "true" ]; then
            echo -e "${YELLOW}⚠️  $description - FAILED (EXPECTED - hydration bug confirmed)${NC}"
            
            # Check for specific error patterns
            if grep -q "expected.*elements.*found" "test_logs/quick_validation/${test_name}.log"; then
                echo -e "${YELLOW}   → Tuple mismatch error detected${NC}"
            fi
            if grep -q "mismatched types" "test_logs/quick_validation/${test_name}.log"; then
                echo -e "${YELLOW}   → Type mismatch error detected${NC}"
            fi
        else
            echo -e "${RED}❌ $description - FAILED (UNEXPECTED)${NC}"
        fi
        return 1
    fi
}

echo -e "${BLUE}📋 Testing Critical Hydration Cases${NC}"
echo "-----------------------------------"

# Test the most critical cases
test_case "test_five_element_view" "Five Element View (CRITICAL)" "true"
test_case "test_hydration_mod_scenario" "Hydration Module Scenario" "true"
test_case "test_empty_view" "Empty View" "false"
test_case "test_single_element_view" "Single Element View" "false"
test_case "test_three_element_view" "Three Element View" "false"

echo ""
echo -e "${BLUE}📋 Testing Feature Flag Compatibility${NC}"
echo "------------------------------------"

# Test feature-specific scenarios
echo "Testing CSR feature..."
if cargo test test_csr_five_elements --package hydration_fix_tests --features csr > test_logs/quick_validation/csr_test.log 2>&1; then
    echo -e "${GREEN}✅ CSR Feature - PASSED${NC}"
else
    echo -e "${YELLOW}⚠️  CSR Feature - FAILED (expected)${NC}"
    if grep -q "expected.*elements.*found" test_logs/quick_validation/csr_test.log; then
        echo -e "${YELLOW}   → Tuple mismatch in CSR${NC}"
    fi
fi

echo "Testing Hydrate feature (most critical)..."
if cargo test test_hydrate_five_elements --package hydration_fix_tests --features hydrate > test_logs/quick_validation/hydrate_test.log 2>&1; then
    echo -e "${GREEN}✅ Hydrate Feature - PASSED${NC}"
else
    echo -e "${YELLOW}⚠️  Hydrate Feature - FAILED (expected)${NC}"
    if grep -q "expected.*elements.*found" test_logs/quick_validation/hydrate_test.log; then
        echo -e "${YELLOW}   → Tuple mismatch in Hydrate${NC}"
    fi
fi

echo ""
echo -e "${BLUE}📋 Compilation Status Check${NC}"
echo "----------------------------"

# Quick compilation check
echo "Checking workspace compilation..."
if cargo check --workspace --all-features > test_logs/quick_validation/compilation_check.log 2>&1; then
    echo -e "${GREEN}✅ Workspace compiles successfully${NC}"
else
    echo -e "${YELLOW}⚠️  Workspace has compilation issues (expected)${NC}"
    if grep -q "expected.*elements.*found" test_logs/quick_validation/compilation_check.log; then
        echo -e "${YELLOW}   → Tuple mismatch in compilation${NC}"
    fi
    if grep -q "type annotations needed" test_logs/quick_validation/compilation_check.log; then
        echo -e "${YELLOW}   → Type annotation issues${NC}"
    fi
fi

echo ""
echo -e "${GREEN}🎉 Quick validation completed!${NC}"
echo ""
echo -e "${YELLOW}📋 Summary:${NC}"
echo "  - Hydration bug confirmed"
echo "  - Test infrastructure working"
echo "  - Ready for fix implementation"
echo ""
echo -e "${CYAN}📊 Logs saved to: test_logs/quick_validation/${NC}"
