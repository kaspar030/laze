cleanup() {
    rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log
    rm -rf build
}

build() {
    if [ -f EXPECTED_EXIT_CODE ]; then
        # ignore actual exit code
        set +e
    fi

    ${LAZE} build -g "$@"
    EXIT_CODE=$?

    if [ -f EXPECTED_EXIT_CODE ]; then
        set -e
        test "$EXIT_CODE" = "$(cat EXPECTED_EXIT_CODE)"
    fi
}

clean_temp_files() {
    rm -rf \
        build/.ninja_log build/.ninja_deps \
        build/laze-cache-local.bincode \
        build/laze-cache-global.bincode \
        compile_commands.json
}

: "${LAZE:=laze}"

set -e
