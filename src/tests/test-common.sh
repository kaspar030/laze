cleanup() {
    rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log stderr stdout
    rm -rf build
}

build() {
    if [ -f EXPECTED_EXIT_CODE ]; then
        # ignore actual exit code
        set +e
    fi

    ${LAZE} build -g "$@" > stdout 2> stderr
    EXIT_CODE=$?

    if [ -f EXPECTED_EXIT_CODE ]; then
        set -e
        test "$EXIT_CODE" = "$(cat EXPECTED_EXIT_CODE)"
    fi

    if [ -f EXPECTED_STDOUT ]; then
        echo testing stdout
        diff -q EXPECTED_STDOUT stdout
    fi

    if [ -f EXPECTED_STDERR ]; then
        echo testing stderr
        diff -q EXPECTED_STDERR stderr
    fi

    if [ -f EXPECTED_STDOUT_PATTERNS ]; then
        echo testing stdout patterns
        grep --silent -f EXPECTED_STDOUT_PATTERNS stdout
    fi

    if [ -f EXPECTED_STDERR_PATTERNS ]; then
        echo testing stderr patterns
        grep --silent -f EXPECTED_STDERR_PATTERNS stderr
    fi
}

clean_temp_files() {
    rm -rf \
        build/.ninja_log build/.ninja_deps \
        build/laze-cache-local.bincode \
        build/laze-cache-global.bincode \
        compile_commands.json \
        stdout stderr
}

: "${LAZE:=laze}"

set -e
