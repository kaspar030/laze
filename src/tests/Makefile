TESTS = $(dir $(wildcard */test.sh))

.PHONY: test
.PHONY: rebuild

export LAZE=kcov --include-pattern=laze/src cov ../../../target/debug/laze

rebuild:
	cargo build

test: rebuild
	for testdir in $(TESTS); do \
		(cd $${testdir}; ./test.sh; ) || exit 1 ; \
		done
	kcov --merge ../../target/cov */cov $${COVERALLS_TOKEN:+--coveralls-id=${COVERALLS_TOKEN}}
	rm -Rf */cov
