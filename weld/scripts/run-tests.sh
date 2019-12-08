#!/bin/bash
get_files () {
  bazel query $1 2>/dev/null
}

get_targets () {
  bazel query "attr('srcs', $1, //...)" 2>/dev/null
}

get_dependencies () {
  bazel query "rdeps(//..., $1)" 2>/dev/null
}
export -f get_dependencies
export -f get_targets
export -f get_files

FILES=$(files | xargs -n 1 bash -c 'get_files "$@"' _)
echo "found files: $FILES"

TARGETS=$(echo $FILES | xargs -n 1 bash -c 'get_targets "$@"' _)
echo "found targets: $TARGETS"

echo $TARGETS | xargs bazel build -c fastbuild

if [ $? -ne 0 ]; then
  echo "Build failed" >&2
  exit 1
fi

bazel test -c fastbuild --test_output=errors $(echo $TARGETS | tr '\n' ' ')
EXIT=$?
if [ $EXIT -ne 0 ] && [ $EXIT -ne 4 ]; then
  echo "Test failed with exit code $EXIT" >&2
  exit 1
fi

DEPENDENCIES=$(echo $TARGETS | xargs -n 1 bash -c 'get_dependencies "$@"' _)
echo $DEPENDENCIES | xargs bazel build
if [ $? -ne 0 ]; then
  echo "Build failed" >&2
  exit 1
fi
echo "found dependencies: $DEPENDENCIES"

bazel test -c fastbuild --test_output=errors $(echo $DEPENDENCIES | tr '\n' ' ')
EXIT=$?
if [ $EXIT -ne 0 ] && [ $EXIT -ne 4 ]; then
  echo "Test failed with exit code $EXIT" >&2
  exit 1
fi
