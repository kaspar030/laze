cleanup() {
    rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log
    rm -rf build
}

build() {
    ${LAZE} build -g
}

clean_temp_files() {
    rm -rf \
        build/.ninja_log build/.ninja_deps \
        build/laze-cache-local.bincode \
        build/laze-cache-global.bincode
}

: "${LAZE:=laze}"

set -e
