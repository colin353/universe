#!/bin/bash

IP_ADDR=$1

bazel build //largetable:largetable_server
RUST_LOG=debug RUST_BACKTRACE=1 bazel run //largetable:largetable_server -- \
  --data_directory="~/x20/data/largetable/" &

sudo umount -l $HOME/codefs

sleep 1

RUST_BACKTRACE=1 bazel run //weld:weld_client -- \
  --weld_hostname=$IP_ADDR \
  --mount_point=$HOME/codefs \
  --use_tls=false

jobs -p | xargs -I{} kill -- {}
