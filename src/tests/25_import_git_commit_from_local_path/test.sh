#!/bin/sh

rm -Rf testrepo
git init testrepo
cp foo.c testrepo/foo_imported.c
cp imported.yml testrepo/laze.yml
git -C testrepo add .
git -C testrepo commit -m ...

. ../test-common.sh

cleanup

${LAZE} build -g

clean_temp_files
rm -Rf testrepo
rm -Rf build/imports/testrepo-11985653632366690788

diff -r build build_expected

echo TEST_OK

cleanup
