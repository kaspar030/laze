#!/bin/sh

: ${LAZERS:=lazers}

rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log

set -e

${LAZERS} generate
ninja -v

test "$(cat single_app.o)" = "$(cat single_app.c)"
test "$(cat single_app.elf)" = "$(cat single_app.c)"

rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log
