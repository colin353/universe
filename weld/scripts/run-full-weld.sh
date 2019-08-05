#!/bin/bash

IP_ADDR=$1

sudo umount -f $HOME/codefs

sleep 0.5

RUST_LOG=debug RUST_BACKTRACE=1 bazel run //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/" &

sleep 0.5

RUST_BACKTRACE=1 bazel run //weld:weld_server -- \
  --root_cert="/home/colin/Documents/scratch/certs/root.crt" \
  --pkcs12="/home/colin/Documents/scratch/certs/server.p12" \
  --use_mock_largetable=true &

sleep 0.5

sudo umount -l /home/colinmerkel/codefs

RUST_BACKTRACE=1 bazel run //weld:weld_client -- \
  --use_tls=true \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --weld_hostname=127.0.0.1 \
  --mount_point=/home/colin/codefs &

sleep 0.5

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=true \
  --root_ca="/home/colin/Documents/scratch/certs/root.der" \
  --cert="/home/colin/Documents/scratch/certs/client.p12" \
  --server_hostname=127.0.0.1

jobs -p | xargs -I{} kill -- {}
