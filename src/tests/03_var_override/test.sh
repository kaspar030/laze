#!/bin/sh

: ${LAZERS:=lazers}

rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log

set -e

${LAZERS} generate
ninja -v

test "$(cat single_app.o)" = "local_var global_var single_app.c"
test "$(cat single_app.elf)" = "$(cat single_app.o)"

rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log

echo TEST_OK
