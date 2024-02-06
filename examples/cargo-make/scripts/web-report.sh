#!/bin/bash

set -emu

BOLD="\e[1m"
ITALIC="\e[3m"
YELLOW="\e[0;33m"
RESET="\e[0m"

function web { #task: only include examples with web cargo-make configuration
    print_header
    print_crate_tags "$@"
    print_footer
}

function all { #task: includes all examples
    print_header
    print_crate_tags "all"
    print_footer
}

function print_header {
    echo -e "${YELLOW}Cargo Make Web Report${RESET}"
    echo
    echo -e "${ITALIC}Show which crates are configured to run and test web examples with cargo-make${RESET}"
    echo
}

function print_crate_tags {
    local makefile_paths
    makefile_paths=$(find_makefile_lines)

    local start_path
    start_path=$(pwd)

    for path in $makefile_paths; do
        cd "$path"

        local crate_tags=

        local pw_count
        pw_count=$(find . -name playwright.config.ts | wc -l)

        while read -r line; do
            case $line in
            *"cucumber"*)
                crate_tags=$crate_tags"C"
                ;;
            *"fantoccini"*)
                crate_tags=$crate_tags"D"
                ;;
            esac
        done <"./Cargo.toml"

        while read -r line; do
            case $line in
            *"cargo-make/wasm-test.toml"*)
                crate_tags=$crate_tags"W"
                ;;
            *"cargo-make/playwright-test.toml"*)
                crate_tags=$crate_tags"P"
                crate_tags=$crate_tags"N"
                ;;
            *"cargo-make/playwright-trunk-test.toml"*)
                crate_tags=$crate_tags"P"
                crate_tags=$crate_tags"T"
                ;;
            *"cargo-make/trunk_server.toml"*)
                crate_tags=$crate_tags"T"
                ;;
            *"cargo-make/cargo-leptos-webdriver-test.toml"*)
                crate_tags=$crate_tags"L"
                ;;
            *"cargo-make/cargo-leptos-test.toml"*)
                crate_tags=$crate_tags"L"
                if [ "$pw_count" -gt 0 ]; then
                    crate_tags=$crate_tags"P"
                fi
                ;;
            esac
        done <"./Makefile.toml"

        # Sort list of tools
        local sorted_crate_symbols
        sorted_crate_symbols=$(echo "$crate_tags" | grep -o . | sort | tr -d "\n")

        local formatted_crate_symbols
        formatted_crate_symbols=" ➤ ${BOLD}${YELLOW}$sorted_crate_symbols${RESET}"

        local crate_line=$path

        if [ "$#" -gt 0 ]; then
            # Show all examples
            if [ -n "$crate_tags" ]; then
                crate_line=$crate_line$formatted_crate_symbols
            fi
            echo -e "$crate_line"
        elif [ -n "$crate_tags" ]; then
            # Show configured examples
            crate_line=$crate_line$formatted_crate_symbols
            echo -e "$crate_line"
        fi

        cd "$start_path"
    done
}

function find_makefile_lines {
    find . -name Makefile.toml -not -path '*/target/*' -not -path '*/node_modules/*' |
        sed 's%./%%' |
        sed 's%/Makefile.toml%%' |
        grep -v Makefile.toml |
        sort -u
}

function print_footer {
    c="${BOLD}${YELLOW}C${RESET} = Cucumber"
    d="${BOLD}${YELLOW}D${RESET} = WebDriver"
    l="${BOLD}${YELLOW}L${RESET} = Cargo Leptos"
    n="${BOLD}${YELLOW}N${RESET} = Node"
    p="${BOLD}${YELLOW}P${RESET} = Playwright"
    t="${BOLD}${YELLOW}T${RESET} = Trunk"
    w="${BOLD}${YELLOW}W${RESET} = WASM"

    echo
    echo -e "${ITALIC}Keys:${RESET} $c, $d, $l, $n, $p, $t, $w"
    echo
}

###################
# HELP
###################

function list_help_for {
    local task=$1
    grep -E "^function.+ #$task" "$0" |
        sed 's/function/ /' |
        sed -e "s| { #$task: |~|g" |
        column -s"~" -t |
        sort
}

function help { #help: show task descriptions
    echo -e "${BOLD}Usage:${RESET} ./$(basename "$0") <task> [options]"
    echo
    echo "Show the cargo-make configuration for web examples"
    echo
    echo -e "${BOLD}Tasks:${RESET}"
    list_help_for task
    echo
}

TIMEFORMAT="./web-report.sh completed in %3lR"
time "${@:-web}" # Show the report by default
