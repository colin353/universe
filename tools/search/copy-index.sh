#!/bin/bash

bazel run //tools/search:indexer -- \
  --input_dir=$PWD \
  --output_dir=~/Documents/code/index

scp ~/Documents/code/index/* colin@192.168.86.158:/home/colin/index
