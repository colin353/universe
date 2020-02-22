load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# Add a comment into the WORKSPACE
http_archive(
    name = "io_bazel_rules_rust",
    sha256 = "521595e99aabc944fb3b3f40d07884efabed0303d2590645a77a96ebeee6aa11",
    strip_prefix = "rules_rust-2e9ef3dd34245337fb90d973454c98c36f651cd4",
    urls = [
        "https://github.com/bazelbuild/rules_rust/archive/2e9ef3dd34245337fb90d973454c98c36f651cd4.tar.gz",
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
    digest = "sha256:4b4c82caa1f48d149e200d39fa1751e17283bfc2e6caea0acab0cc040934fbf4",
    registry = "registry.hub.docker.com",
    repository = "colinmerkel/build",
)

http_archive(
    name = "rules_pkg",
    sha256 = "4ba8f4ab0ff85f2484287ab06c0d871dcb31cc54d439457d28fd4ae14b18450a",
    url = "https://github.com/bazelbuild/rules_pkg/releases/download/0.2.4/rules_pkg-0.2.4.tar.gz",
)
