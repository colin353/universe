#!/bin/bash

IP_ADDR=$1

blaze build //largetable:largetable_server
RUST_LOG=debug RUST_BACKTRACE=1 blaze run //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/" &

sudo umount -l /home/colinmerkel/codefs

RUST_BACKTRACE=1 blaze run //weld:weld_client -- \
  --weld_hostname=$IP_ADDR \
  --mount_point=/home/colinmerkel/codefs

jobs -p | xargs -I{} kill -- {}
