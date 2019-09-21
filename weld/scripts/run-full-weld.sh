#!/bin/bash

IP_ADDR=$1

sleep 1

RUST_LOG=debug RUST_BACKTRACE=1 bazel run -c opt //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/client/" \
  --port=50051 &

RUST_LOG=debug RUST_BACKTRACE=1 bazel run -c opt //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/server/" \
  --port=50052 &

sleep 1

RUST_BACKTRACE=1 bazel run -c opt //weld:weld_server -- \
  --root_cert="/home/colin/Documents/scratch/certs/root.crt" \
  --pkcs12="/home/colin/Documents/scratch/certs/server.p12" \
  --use_tls=false \
  --use_mock_largetable=false &

sleep 1

sudo umount -l $HOME/codefs

RUST_BACKTRACE=1 bazel run -c opt //weld:weld_client -- \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --largetable_port=50052 \
  --use_tls=false \
  --weld_hostname=127.0.0.1 \
  --mount_point=/home/colin/codefs &

sleep 1

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=false \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --server_hostname=127.0.0.1

jobs -p | xargs -I{} kill -- {}
