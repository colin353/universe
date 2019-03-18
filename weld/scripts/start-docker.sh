docker rm largetable
docker run  --net=skynet --name=largetable -p 50051:50051 --mount type=bind,source=/mnt/stateful_partition/data,target=/data us.gcr.io/mushu-194218/largetable@$LARGETABLE --data_directory=/data

docker stop weld
docker rm weld
docker run -d -e RUST_BACKTRACE=1 --net=skynet --name=weld \
  -p 8001:8001 \
  us.gcr.io/mushu-194218/weld@$WELD \
  --largetable_hostname=largetable

