#!/bin/bash

bazel run //tools/search:indexer -- \
  --input_dir=$PWD \
  --output_dir=~/Documents/code/index

gcloud beta compute --project "mushu-194218"\
  scp --zone "us-central1-a" \
  ~/Documents/code/index/* "e2-standard-2":/mnt/stateful_partition/index

