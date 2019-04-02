#!/bin/bash

IP_ADDR=$1

RUST_LOG=debug RUST_BACKTRACE=1 blaze run //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/" &

RUST_BACKTRACE=1 blaze run //weld:weld_server -- \
  --pkcs12="/home/colinmerkel/scratch/certstrap/out4/cert.p12" \
  --use_mock_largetable=true &

sudo umount -l /home/colinmerkel/codefs

RUST_BACKTRACE=1 blaze run //weld:weld_client -- \
  --use_tls=true \
  --root_ca="/home/colinmerkel/scratch/certstrap/out4/cert.der" \
  --weld_hostname=127.0.0.1 \
  --mount_point=/home/colinmerkel/codefs

jobs -p | xargs -I{} kill -- {}
