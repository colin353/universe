#!bin/bash

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=false \
  --port=60065 \
  --auth_hostname=127.0.0.1 \
  --auth_port=60066 \
  --queue_port=5553 \
  --queue_hostname=127.0.0.1 \
  --server_port=60063 \
  --static_files="$PWD/weld/review/static" \
  --base_url=http://review.local.colinmerkel.xyz \
  --server_hostname=127.0.0.1 \
  --disable_auth

jobs -p | xargs -I{} kill -- {}
