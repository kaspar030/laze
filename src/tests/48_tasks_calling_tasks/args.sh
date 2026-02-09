#!/bin/sh

i=1
for arg in "$@"; do
  echo "${i}: ${arg}"
  i=$(( i +1))
done
