#!/bin/sh

rm -Rf testrepo
git init testrepo
cp imported.yml testrepo/laze.yml
git -C testrepo add .
git -C testrepo commit -m ...

. ../test-common.sh

cleanup

${LAZE} build -g

clean_temp_files
rm -Rf testrepo
rm -Rf build/imports

diff_build_dir

echo TEST_OK

cleanup
