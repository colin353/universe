load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# Add a comment into the WORKSPACE
http_archive(
    name = "io_bazel_rules_rust",
    sha256 = "ea90c021a9cbd45a0e37b46907a69e1650c49579df86e3e3f1c98a117eec0b42",
    strip_prefix = "rules_rust-3075d2bbd0800cc1ea7afefa12431261959b3811",
    urls = [
        "https://github.com/colin353/rules_rust/archive/3075d2bbd0800cc1ea7afefa12431261959b3811.tar.gz",
    ],
)

http_archive(
    name = "bazel_skylib",
    sha256 = "9a737999532daca978a158f94e77e9af6a6a169709c0cee274f0a4c3359519bd",
    strip_prefix = "bazel-skylib-1.0.0",
    url = "https://github.com/bazelbuild/bazel-skylib/archive/1.0.0.tar.gz",
)

http_archive(
    name = "io_bazel_rules_docker",
    sha256 = "dc97fccceacd4c6be14e800b2a00693d5e8d07f69ee187babfd04a80a9f8e250",
    strip_prefix = "rules_docker-0.14.1",
    urls = ["https://github.com/bazelbuild/rules_docker/releases/download/v0.14.1/rules_docker-v0.14.1.tar.gz"],
)

http_archive(
    name = "vendored_node",
    build_file_content = """exports_files(["node-v14.8.0-linux-x64/bin/node"])""",
    urls = ["https://nodejs.org/dist/v14.8.0/node-v14.8.0-linux-x64.tar.xz"],
)

load(
    "@io_bazel_rules_docker//repositories:repositories.bzl",
    container_repositories = "repositories",
)

container_repositories()

# This is NOT needed when going through the language lang_image
# "repositories" function(s).
load("@io_bazel_rules_docker//repositories:deps.bzl", container_deps = "deps")

container_deps()

load("@io_bazel_rules_rust//rust:repositories.bzl", "rust_repository_set")

rust_repository_set(
    name = "rust_linux_x86_64",
    exec_triple = "x86_64-unknown-linux-gnu",
    extra_target_triples = [],
    iso_date = "2020-02-16",
    version = "nightly",
)

load("@io_bazel_rules_rust//:workspace.bzl", "bazel_version")

bazel_version(name = "bazel_version")

load("@io_bazel_rules_rust//proto:repositories.bzl", "rust_proto_repositories")

rust_proto_repositories()

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
load(
    "@io_bazel_rules_docker//rust:image.bzl",
    _rust_image_repos = "repositories",
)

_rust_image_repos()

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")
load("@io_bazel_rules_docker//container:pull.bzl", "container_pull")

container_pull(
    name = "glibc_base",
    digest = "sha256:32c93ea6e867f4deee92912656c77f78f50e6e3d031dbfd85270dd30d75ed1ff",
    registry = "gcr.io",
    repository = "distroless/cc-debian10",
)

container_pull(
    name = "build_base",
    digest = "sha256:1eeb62970ebe6377b347450653ad8036f4706a4a89db12fadb10352d7de1809f",
    registry = "registry.hub.docker.com",
    repository = "colinmerkel/build",
)

http_archive(
    name = "rules_pkg",
    sha256 = "4ba8f4ab0ff85f2484287ab06c0d871dcb31cc54d439457d28fd4ae14b18450a",
    url = "https://github.com/bazelbuild/rules_pkg/releases/download/0.2.4/rules_pkg-0.2.4.tar.gz",
)
