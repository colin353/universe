#!/bin/bash

docker container prune --all --force

# Need to ignore the status here, since it may exist
docker network create server || true
