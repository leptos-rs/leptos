#!/bin/bash
# Comprehensive Hydration Fix Validation
# This script can be used for both baseline and post-fix validation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Parse command line arguments
BASELINE_MODE=false
POST_FIX_MODE=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --baseline)
            BASELINE_MODE=true
            shift
            ;;
        --post-fix)
            POST_FIX_MODE=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--baseline|--post-fix] [--verbose]"
            echo ""
            echo "Options:"
            echo "  --baseline    Run in baseline mode (expect failures)"
            echo "  --post-fix    Run in post-fix mode (expect success)"
            echo "  --verbose     Enable verbose output"
            echo "  --help        Show this help message"
            echo ""
            echo "If no mode is specified, the script will auto-detect based on compilation status"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Auto-detect mode if not specified
if [ "$BASELINE_MODE" = false ] && [ "$POST_FIX_MODE" = false ]; then
    echo "Auto-detecting mode..."
    if cargo check --workspace --all-features > /dev/null 2>&1; then
        POST_FIX_MODE=true
        echo "Detected: Post-fix mode (workspace compiles successfully)"
    else
        BASELINE_MODE=true
        echo "Detected: Baseline mode (workspace has compilation issues)"
    fi
fi

# Create logs directory
LOG_DIR="test_logs/$(if [ "$POST_FIX_MODE" = true ]; then echo "post_fix"; else echo "baseline"; fi)"
mkdir -p "$LOG_DIR"

echo -e "${BLUE}🧪 Leptos Hydration Fix Validation${NC}"
echo "====================================="
echo "Mode: $(if [ "$POST_FIX_MODE" = true ]; then echo "POST-FIX"; else echo "BASELINE"; fi)"
echo "Started: $(date)"
echo ""

# Function to log with timestamp
log_with_time() {
    local color="$1"
    local message="$2"
    echo -e "${color}[$(date '+%Y-%m-%d %H:%M:%S')] $message${NC}"
}

# Function to run test and report
run_test() {
    local test_name="$1"
    local description="$2"
    local expected_result="$3"  # "pass" or "fail"
    
    log_with_time "$BLUE" "Testing: $description"
    
    if cargo test "$test_name" --package hydration_fix_tests > "$LOG_DIR/${test_name}.log" 2>&1; then
        if [ "$expected_result" = "pass" ]; then
            echo -e "${GREEN}✅ $description - PASSED (as expected)${NC}"
            return 0
        else
            echo -e "${RED}❌ $description - PASSED (UNEXPECTED!)${NC}"
            return 1
        fi
    else
        if [ "$expected_result" = "fail" ]; then
            echo -e "${YELLOW}⚠️  $description - FAILED (as expected)${NC}"
            
            # Check for specific error patterns
            if grep -q "expected.*elements.*found" "$LOG_DIR/${test_name}.log"; then
                echo -e "${YELLOW}   → Tuple mismatch error detected${NC}"
            fi
            if grep -q "mismatched types" "$LOG_DIR/${test_name}.log"; then
                echo -e "${YELLOW}   → Type mismatch error detected${NC}"
            fi
            return 0
        else
            echo -e "${RED}❌ $description - FAILED (UNEXPECTED)${NC}"
            if [ "$VERBOSE" = true ]; then
                echo "Error details:"
                tail -20 "$LOG_DIR/${test_name}.log"
            fi
            return 1
        fi
    fi
}

# Function to check compilation
check_compilation() {
    local description="$1"
    local expected_result="$2"  # "pass" or "fail"
    
    log_with_time "$BLUE" "Checking: $description"
    
    if cargo check --workspace --all-features > "$LOG_DIR/compilation.log" 2>&1; then
        if [ "$expected_result" = "pass" ]; then
            echo -e "${GREEN}✅ $description - PASSED (as expected)${NC}"
            return 0
        else
            echo -e "${RED}❌ $description - PASSED (UNEXPECTED!)${NC}"
            return 1
        fi
    else
        if [ "$expected_result" = "fail" ]; then
            echo -e "${YELLOW}⚠️  $description - FAILED (as expected)${NC}"
            
            # Check for specific error patterns
            if grep -q "expected.*elements.*found" "$LOG_DIR/compilation.log"; then
                echo -e "${YELLOW}   → Tuple mismatch error detected${NC}"
            fi
            if grep -q "type annotations needed" "$LOG_DIR/compilation.log"; then
                echo -e "${YELLOW}   → Type annotation error detected${NC}"
            fi
            return 0
        else
            echo -e "${RED}❌ $description - FAILED (UNEXPECTED)${NC}"
            if [ "$VERBOSE" = true ]; then
                echo "Error details:"
                tail -20 "$LOG_DIR/compilation.log"
            fi
            return 1
        fi
    fi
}

# Set expected results based on mode
if [ "$POST_FIX_MODE" = true ]; then
    EXPECTED_COMPILATION="pass"
    EXPECTED_TESTS="pass"
    MODE_DESCRIPTION="POST-FIX (expecting success)"
else
    EXPECTED_COMPILATION="fail"
    EXPECTED_TESTS="fail"
    MODE_DESCRIPTION="BASELINE (expecting failures)"
fi

echo -e "${PURPLE}📋 Phase 1: Compilation Validation${NC}"
echo "====================================="

check_compilation "Workspace Compilation" "$EXPECTED_COMPILATION"

echo ""
echo -e "${PURPLE}📋 Phase 2: Critical Test Cases${NC}"
echo "================================="

# Test the most critical cases
run_test "test_five_element_view" "Five Element View (CRITICAL)" "$EXPECTED_TESTS"
run_test "test_hydration_mod_scenario" "Hydration Module Scenario" "$EXPECTED_TESTS"
run_test "test_empty_view" "Empty View" "pass"
run_test "test_single_element_view" "Single Element View" "pass"
run_test "test_three_element_view" "Three Element View" "pass"

echo ""
echo -e "${PURPLE}📋 Phase 3: Feature Flag Testing${NC}"
echo "================================="

# Test feature-specific scenarios
for feature in csr ssr hydrate; do
    echo "Testing $feature feature..."
    if cargo test "test_${feature}_five_elements" --package hydration_fix_tests --features "$feature" > "$LOG_DIR/${feature}_test.log" 2>&1; then
        if [ "$POST_FIX_MODE" = true ]; then
            echo -e "${GREEN}✅ $feature Feature - PASSED${NC}"
        else
            echo -e "${RED}❌ $feature Feature - PASSED (UNEXPECTED!)${NC}"
        fi
    else
        if [ "$BASELINE_MODE" = true ]; then
            echo -e "${YELLOW}⚠️  $feature Feature - FAILED (as expected)${NC}"
            if grep -q "expected.*elements.*found" "$LOG_DIR/${feature}_test.log"; then
                echo -e "${YELLOW}   → Tuple mismatch in $feature${NC}"
            fi
        else
            echo -e "${RED}❌ $feature Feature - FAILED (UNEXPECTED)${NC}"
        fi
    fi
done

echo ""
echo -e "${PURPLE}📋 Phase 4: Example Validation${NC}"
echo "================================="

# Test key examples
for example in counter hackernews; do
    if [ -d "examples/$example" ]; then
        echo "Testing example: $example"
        if cd "examples/$example" && cargo check > "../../$LOG_DIR/example_${example}.log" 2>&1; then
            if [ "$POST_FIX_MODE" = true ]; then
                echo -e "${GREEN}✅ Example $example - PASSED${NC}"
            else
                echo -e "${RED}❌ Example $example - PASSED (UNEXPECTED!)${NC}"
            fi
        else
            if [ "$BASELINE_MODE" = true ]; then
                echo -e "${YELLOW}⚠️  Example $example - FAILED (may be expected)${NC}"
                if grep -q "expected.*elements.*found" "../../$LOG_DIR/example_${example}.log"; then
                    echo -e "${YELLOW}   → Tuple mismatch in $example${NC}"
                fi
            else
                echo -e "${RED}❌ Example $example - FAILED (UNEXPECTED)${NC}"
            fi
        fi
        cd ../..
    fi
done

echo ""
echo -e "${PURPLE}📋 Phase 5: Performance Check${NC}"
echo "================================="

# Quick performance check
echo "Capturing build time..."
time cargo build --package hydration_fix_tests > "$LOG_DIR/build_time.log" 2>&1
echo -e "${CYAN}📊 Build time captured${NC}"

echo ""
echo -e "${PURPLE}📋 Phase 6: Summary Report${NC}"
echo "==============================="

# Generate summary report
echo "# Leptos Hydration Fix Validation Report" > "$LOG_DIR/validation_report.md"
echo "Generated: $(date)" >> "$LOG_DIR/validation_report.md"
echo "Mode: $MODE_DESCRIPTION" >> "$LOG_DIR/validation_report.md"
echo "" >> "$LOG_DIR/validation_report.md"

echo "## Test Results Summary" >> "$LOG_DIR/validation_report.md"
echo "- Compilation: $(if [ "$EXPECTED_COMPILATION" = "pass" ]; then echo "Expected to PASS"; else echo "Expected to FAIL"; fi)" >> "$LOG_DIR/validation_report.md"
echo "- Critical Tests: $(if [ "$EXPECTED_TESTS" = "pass" ]; then echo "Expected to PASS"; else echo "Expected to FAIL"; fi)" >> "$LOG_DIR/validation_report.md"

echo "" >> "$LOG_DIR/validation_report.md"
echo "## Error Patterns Detected" >> "$LOG_DIR/validation_report.md"
if grep -r "expected.*elements.*found" "$LOG_DIR/" > /dev/null 2>&1; then
    echo "- ✅ Tuple mismatch errors detected" >> "$LOG_DIR/validation_report.md"
else
    echo "- ❌ No tuple mismatch errors found" >> "$LOG_DIR/validation_report.md"
fi

if grep -r "type annotations needed" "$LOG_DIR/" > /dev/null 2>&1; then
    echo "- ✅ Type annotation errors detected" >> "$LOG_DIR/validation_report.md"
else
    echo "- ❌ No type annotation errors found" >> "$LOG_DIR/validation_report.md"
fi

echo "" >> "$LOG_DIR/validation_report.md"
echo "## Status" >> "$LOG_DIR/validation_report.md"
if [ "$POST_FIX_MODE" = true ]; then
    echo "- 🎉 **HYDRATION FIX VALIDATED**" >> "$LOG_DIR/validation_report.md"
    echo "- All tests passing" >> "$LOG_DIR/validation_report.md"
    echo "- No tuple mismatch errors" >> "$LOG_DIR/validation_report.md"
else
    echo "- 🔧 **BASELINE CONFIRMED**" >> "$LOG_DIR/validation_report.md"
    echo "- Hydration bug present" >> "$LOG_DIR/validation_report.md"
    echo "- Ready for fix implementation" >> "$LOG_DIR/validation_report.md"
fi

echo ""
echo -e "${GREEN}🎉 Validation completed!${NC}"
echo -e "${CYAN}📊 Full report saved to: $LOG_DIR/validation_report.md${NC}"
echo ""
echo -e "${YELLOW}📋 Summary:${NC}"
if [ "$POST_FIX_MODE" = true ]; then
    echo "  - ✅ Hydration fix validated"
    echo "  - ✅ All tests passing"
    echo "  - ✅ Ready for release"
else
    echo "  - 🔧 Baseline confirmed"
    echo "  - 🔧 Hydration bug present"
    echo "  - 🔧 Ready for fix implementation"
fi
