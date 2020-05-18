package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//container:container.bzl", "container_image")
load("@bazel_tools//tools/build_defs/pkg:pkg.bzl", "pkg_tar")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

pkg_tar(
    name = "static_files",
    srcs = glob([
        "static/**",
    ]),
    strip_prefix = ".",
)

container_image(
    name = "homepage_base_img",
    base = "@glibc_base//image",
    tars = [":static_files"],
)

rust_image(
    name = "homepage_img",
    srcs = ["main.rs"],
    base = ":homepage_base_img",
    deps = [
        "//auth:auth_client",
        "//util/flags",
        "//util/ws",
    ],
)

docker_push(
    name = "homepage_img_push",
    image = ":homepage_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/homepage",
    tag = "dev",
)

rust_binary(
    name = "homepage",
    srcs = ["main.rs"],
    deps = [
        "//auth:auth_client",
        "//util/flags",
        "//util/ws",
    ],
)
