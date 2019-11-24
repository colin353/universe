5bin/bash

bazel run //homepage &

bazel run //auth:server -- \
  --grpc_port=60066 \
  --web_port=60067 \
  --oauth_client_id=63851891952-etukpotrduoaa2ch9rfvbheirndlthmr.apps.googleusercontent.com \
  --hostname=http://auth.local.colinmerkel.xyz/ \
  --allowed_emails=colin353@gmail.com \
  --oauth_client_secret=7dBfl21dYrSySjnWXZAgZFOz &

bazel build //largetable:largetable_server

RUST_LOG=debug RUST_BACKTRACE=1 bazel run -c opt //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/client/" \
  --port=60061 &

RUST_LOG=debug RUST_BACKTRACE=1 bazel run -c opt //largetable:largetable_server -- \
  --data_directory="/usr/local/largetable/server/" \
  --port=60062 &

sleep 5

RUST_BACKTRACE=1 bazel run -c opt //weld:weld_server -- \
  --use_tls=false \
  --port=60063 \
  --largetable_port=60062 \
  --use_mock_largetable=false &

sleep 1

sudo umount -l $HOME/codefs-local

RUST_BACKTRACE=1 bazel run -c opt //weld:weld_client -- \
  --port=60064 \
  --largetable_port=60062 \
  --use_tls=false \
  --weld_hostname=127.0.0.1 \
  --server_port=60063 \
  --mount_point=/home/colin/codefs-local &

#DOCKER_HOST=$(ip -4 addr show docker0 | grep -Po 'inet \K[\d.]+'
docker stop nginx
docker rm nginx
mkdir -p /tmp/nginx
cp $PWD/weld/scripts/nginx.conf /tmp/nginx/nginx.conf
docker run -p 80:80 -d --name nginx -v /tmp/nginx/nginx.conf:/etc/nginx/nginx.conf:ro nginx

RUST_BACKTRACE=1 bazel run //weld/review -- \
  --use_tls=false \
  --port=60065 \
  --auth_hostname=127.0.0.1 \
  --auth_port=60066 \
  --server_port=60063 \
  --static_files="$PWD/weld/review/static" \
  --server_hostname=127.0.0.1


jobs -p | xargs -I{} kill -- {}
