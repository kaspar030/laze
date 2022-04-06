#!/bin/sh

. ../test-common.sh

cleanup

${LAZE} task echo foo | grep -s "^foo$"
${LAZE} task foobar | grep -s "^foobar$"
${LAZE} task --global --app subdir --builder default vars \
         |grep -s "^relpath=\. relroot=\.\. out=build/default/subdir/subdir.elf builder=default$"

echo TEST_OK

cleanup
