#!/bin/bash

bazel build //util/bus/example

ret=0

while [ $ret -eq 0 ]; do
  cat /dev/urandom | head -c 15 | ./bazel-bin/util/bus/example/example --read
  ret=$?
done
