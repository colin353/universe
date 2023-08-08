package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@io_bazel_rules_docker//container:container.bzl", "container_image")
load("@bazel_tools//tools/build_defs/pkg:pkg.bzl", "pkg_tar")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")
load("//homepage/sync:sync.bzl", "bucket_sync")

pkg_tar(
    name = "homepage_tar",
    srcs = glob([
        "static/**",
    ]) + [
        ":homepage"
    ],
    strip_prefix = ".",
)

rust_binary(
    name = "homepage",
    srcs = ["main.rs"],
    deps = [
        "//auth:auth_client",
        "//util/flags",
        "//util/ws",
        "//third_party:tokio",
    ],
)

bucket_sync(
    name = "sync_homepage",
    index = "index.html",
    srcs =
        glob([
        "static/**",
        ]) + 
        [
          "//experimental/gitique:index.html",
        ],
    deps = [
        "//experimental/gitique:gitique_ui",
        "//experimental/gitique:github",
    ],
)
