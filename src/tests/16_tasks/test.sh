#!/bin/sh

. ../test-common.sh

cleanup

${LAZE} build echo foo | grep -s "^foo$"
${LAZE} build foobar | grep -s "^foobar$"
${LAZE} build --global --apps subdir --builders default vars \
         |grep -s "^relpath=\. relroot=\.\. out=build/default/subdir/subdir.elf builder=default$"

echo TEST_OK

cleanup
