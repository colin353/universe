#!/bin/bash

IP_ADDR=$1

RUST_LOG=debug RUST_BACKTRACE=1 blaze run //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/" &

RUST_BACKTRACE=1 blaze run //weld:weld_server -- --use_mock_largetable=true &

sudo umount -l /home/colinmerkel/codefs

RUST_BACKTRACE=1 blaze run //weld:weld_client -- \
  --weld_hostname=127.0.0.1 \
  --mount_point=/home/colinmerkel/codefs

jobs -p | xargs -I{} kill -- {}
