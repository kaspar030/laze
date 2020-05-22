#!/bin/sh

: ${LAZERS:=lazers}

rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log

set -e

${LAZERS} generate
ninja -v

test "$(cat single_app.o)" = "local0 local1 local1_0 global0 global1 global1_0 global1_1 single_app.c"
test "$(cat single_app.elf)" = "$(cat single_app.o)"

rm -f single_app.o single_app.elf build.ninja .ninja_deps .ninja_log

echo TEST_OK
