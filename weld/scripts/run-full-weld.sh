#!/bin/bash

IP_ADDR=$1

RUST_LOG=debug RUST_BACKTRACE=1 bazel run //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/" &

RUST_BACKTRACE=1 bazel run //weld:weld_server -- \
  --root_cert="/home/colin/Documents/scratch/certs/root.crt" \
  --pkcs12="/home/colin/Documents/scratch/certs/server.p12" \
  --use_mock_largetable=true &

sudo umount -l /home/colinmerkel/codefs

RUST_BACKTRACE=1 bazel run //weld:weld_client -- \
  --use_tls=true \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --weld_hostname=127.0.0.1 \
  --mount_point=/home/colin/codefs &

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=true \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --server_hostname=127.0.0.1

jobs -p | xargs -I{} kill -- {}
