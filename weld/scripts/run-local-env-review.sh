#!bin/bash

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=false \
  --port=60065 \
  --auth_hostname=127.0.0.1 \
  --auth_port=60066 \
  --task_hostname=localhost \
  --task_port=60068 \
  --server_port=60063 \
  --static_files="$PWD/weld/review/static" \
  --base_url=http://review.local.colinmerkel.xyz \
  --server_hostname=127.0.0.1

jobs -p | xargs -I{} kill -- {}
