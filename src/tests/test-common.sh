cleanup() {
    rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log
    rm -rf build
}

build() {
    ${LAZERS} build -g
}

: "${LAZERS:=lazers}"

set -e
