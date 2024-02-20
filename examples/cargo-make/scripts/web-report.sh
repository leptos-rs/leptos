#!/bin/bash

set -emu

BOLD="\e[1m"
ITALIC="\e[3m"
YELLOW="\e[1;33m"
BLUE="\e[1;36m"
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
    echo -e "${ITALIC}Show how crates are configured to run and test web examples with cargo-make${RESET}"
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

        # Add cargo tags
        while read -r line; do
            case $line in
            *"cucumber"*)
                crate_tags=$crate_tags"C"
                ;;
            *"fantoccini"*)
                crate_tags=$crate_tags"F"
                ;;
            *"package.metadata.leptos"*)
                crate_tags=$crate_tags"M"
                ;;
            esac
        done <"./Cargo.toml"

        #Add makefile tags

        local pw_count
        pw_count=$(find . -name playwright.config.ts | wc -l)

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
            *"cargo-make/cargo-leptos.toml"*)
                crate_tags=$crate_tags"L"
                ;;
            *"cargo-make/deno-build.toml"*)
                crate_tags=$crate_tags"D"
                ;;
            esac
        done <"./Makefile.toml"

        # Sort tags
        local keys
        keys=$(echo "$crate_tags" | grep -o . | sort | tr -d "\n")

        # Find leptos projects that are not configured to build with cargo-leptos
        keys=${keys//"LM"/"L"}

        # Find leptos projects that are not configured to build with deno
        keys=${keys//"DM"/"D"}

        # Maybe print line
        local crate_line=$path

        if [ -n "$crate_tags" ]; then
            local color=$YELLOW
            case $keys in
            *"M"*)
                color=$BLUE
                ;;
            esac

            crate_line="$crate_line ➤ ${color}$keys${RESET}"
            echo -e "$crate_line"
        elif [ "$#" -gt 0 ]; then
            crate_line="${BOLD}$crate_line${RESET}"
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
    c="${BOLD}${YELLOW}C${RESET} = Cucumber Test Runner"
    d="${BOLD}${YELLOW}D${RESET} = Deno"
    f="${BOLD}${YELLOW}F${RESET} = Fantoccini WebDriver"
    l="${BOLD}${YELLOW}L${RESET} = Cargo Leptos"
    m="${BOLD}${BLUE}M${RESET} = Cargo Leptos Metadata Only (${ITALIC}ci is not configured to build with cargo-leptos or deno${RESET})"
    n="${BOLD}${YELLOW}N${RESET} = Node"
    p="${BOLD}${YELLOW}P${RESET} = Playwright Test"
    t="${BOLD}${YELLOW}T${RESET} = Trunk"
    w="${BOLD}${YELLOW}W${RESET} = WASM Test"

    echo
    echo -e "${ITALIC}Report Keys:${RESET}\n $c\n $d\n $f\n $l\n $m\n $n\n $p\n $t\n $w"
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
time "${@:-all}" # Show the report by default
