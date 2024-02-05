#!/bin/bash

set -emu

BOLD="\e[1m"
ITALIC="\e[3m"
YELLOW="\e[0;33m"
RESET="\e[0m"

echo
echo -e "${YELLOW}Web Test Technology${RESET}"
echo

makefile_paths=$(find . -name Makefile.toml -not -path '*/target/*' -not -path '*/node_modules/*' |
    sed 's%./%%' |
    sed 's%/Makefile.toml%%' |
    grep -v Makefile.toml |
    sort -u)

start_path=$(pwd)

for path in $makefile_paths; do
    cd "$path"

    crate_symbols=

    pw_count=$(find . -name playwright.config.ts | wc -l)

    while read -r line; do
        case $line in
        *"cucumber"*)
            crate_symbols=$crate_symbols"C"
            ;;
        *"fantoccini"*)
            crate_symbols=$crate_symbols"D"
            ;;
        esac
    done <"./Cargo.toml"

    while read -r line; do
        case $line in
        *"cargo-make/wasm-test.toml"*)
            crate_symbols=$crate_symbols"W"
            ;;
        *"cargo-make/playwright-test.toml"*)
            crate_symbols=$crate_symbols"P"
            crate_symbols=$crate_symbols"N"
            ;;
        *"cargo-make/playwright-trunk-test.toml"*)
            crate_symbols=$crate_symbols"P"
            crate_symbols=$crate_symbols"T"
            ;;
        *"cargo-make/trunk_server.toml"*)
            crate_symbols=$crate_symbols"T"
            ;;
        *"cargo-make/cargo-leptos-webdriver-test.toml"*)
            crate_symbols=$crate_symbols"L"
            ;;
        *"cargo-make/cargo-leptos-test.toml"*)
            crate_symbols=$crate_symbols"L"
            if [ "$pw_count" -gt 0 ]; then
                crate_symbols=$crate_symbols"P"
            fi
            ;;
        esac
    done <"./Makefile.toml"

    # Sort list of tools
    sorted_crate_symbols=$(echo ${crate_symbols} | grep -o . | sort | tr -d "\n")

    formatted_crate_symbols=" ➤ ${BOLD}${YELLOW}${sorted_crate_symbols}${RESET}"
    crate_line=$path
    if [ ! -z ${1+x} ]; then
        # Show all examples
        if [ ! -z $crate_symbols ]; then
            crate_line=$crate_line$formatted_crate_symbols
        fi
        echo -e "$crate_line"
    elif [ ! -z $crate_symbols ]; then
        # Filter out examples that do not run tests in `ci`
        crate_line=$crate_line$formatted_crate_symbols
        echo -e "$crate_line"
    fi

    cd "$start_path"
done

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
