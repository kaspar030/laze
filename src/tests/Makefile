TESTS = $(dir $(wildcard */test.sh))

all:
	for testdir in $(TESTS); do \
		(cd $${testdir}; ./test.sh; ) || exit 1 ; \
		done
