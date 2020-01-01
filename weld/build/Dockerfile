from ubuntu:19.04

RUN apt-get update
RUN apt-get -y install curl gnupg2
RUN curl -N -s https://bazel.build/bazel-release.pub.gpg | apt-key add -
RUN echo "deb [arch=amd64] https://storage.googleapis.com/bazel-apt stable jdk1.8" | tee /etc/apt/sources.list.d/bazel.list
RUN apt-get update
RUN apt-get -y install bazel libfuse-dev libfuse2 git python pkg-config
RUN echo "build --jobs=1 --ram_utilization_factor=50 --host_force_python=PY2 --remote_http_cache=https://storage.googleapis.com/colinmerkel-bazel-cache --google_credentials=/data/bazel-access.json" >> /etc/bazel.bazelrc
RUN echo "test --jobs=1 --ram_utilization_factor=50 --host_force_python=PY2 --remote_http_cache=https://storage.googleapis.com/colinmerkel-bazel-cache --google_credentials=/data/bazel-access.json" >> /etc/bazel.bazelrc
RUN echo "run --jobs=1 --ram_utilization_factor=50 --host_force_python=PY2 --remote_http_cache=https://storage.googleapis.com/colinmerkel-bazel-cache --google_credentials=/data/bazel-access.json" >>  /etc/bazel.bazelrc
RUN mkdir /data
RUN bazel
