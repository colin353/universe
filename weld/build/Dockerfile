from ubuntu:19.04

RUN apt-get update
RUN apt-get -y install curl gnupg2
RUN curl -N -s https://bazel.build/bazel-release.pub.gpg | apt-key add -
RUN echo "deb [arch=amd64] https://storage.googleapis.com/bazel-apt stable jdk1.8" | tee /etc/apt/sources.list.d/bazel.list
RUN apt-get update
RUN apt-get -y install bazel libfuse-dev libfuse2 git python pkg-config
RUN echo "build --jobs=2 --host_force_python=PY2 --remote_http_cache=https://storage.googleapis.com/colinmerkel-bazel-cache --google_credentials=/data/bazel-access.json" >> /etc/bazel.bazelrc
RUN echo "test --jobs=2 --host_force_python=PY2 --remote_http_cache=https://storage.googleapis.com/colinmerkel-bazel-cache --google_credentials=/data/bazel-access.json" >> /etc/bazel.bazelrc
RUN echo "run --jobs=2 --host_force_python=PY2 --remote_http_cache=https://storage.googleapis.com/colinmerkel-bazel-cache --google_credentials=/data/bazel-access.json" >>  /etc/bazel.bazelrc
RUN mkdir /root/.ssh
RUN echo "Host github.com" >> /root/.ssh/config
RUN echo "   StrictHostKeyChecking no" >> /root/.ssh/config
RUN git config --global user.email "weld@colinmerkel.xyz"
RUN git config --global user.name "weld-bot"
RUN mkdir /data
RUN bazel
