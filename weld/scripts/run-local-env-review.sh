#!bin/bash

RUST_BACKTRACE=1 bazel run //tasks -- \
  --grpc_port=60068 \
  --web_port=60069 \
  --weld_port=60064 \
  --weld_hostname=127.0.0.1 \
  --base_url=http://tasks.local.colinmerkel.xyz &

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
