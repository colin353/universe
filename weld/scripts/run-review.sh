#!/bin/bash

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=false \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --server_hostname=127.0.0.1
