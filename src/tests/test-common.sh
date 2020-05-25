cleanup() {
    rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log
}

build() {
    ${LAZERS} build
}

: "${LAZERS:=lazers}"

set -e
