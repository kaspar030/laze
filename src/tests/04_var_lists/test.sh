#!/bin/sh

. ../test-common.sh
cleanup
build

test "$(cat single_app.o)" = "local0 local1 local1_0 global0 global1 global1_0 global1_1 single_app.c"
test "$(cat single_app.elf)" = "$(cat single_app.o)"

echo TEST_OK

cleanup
