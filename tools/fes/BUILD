package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

rust_binary(
    name = "fes",
    srcs = ["fes.rs"],
    edition = "2018",
    deps = [
        "//util/flags",
        "//util/ws",
    ],
)

rust_binary(
    name = "fes_with_js",
    srcs = ["fes.rs"],
    data = [
        "//homepage/js:code_pad",
        "//tools/search/js:search_box",
        "//tools/search/js:flow_card",
        "//tools/search/js:flow_view",
    ],
    edition = "2018",
    deps = [
        "//util/flags",
        "//util/ws",
    ],
)

rust_image(
    name = "fes_image",
    base = "@glibc_base//image",
    binary = ":fes_with_js",
)

docker_push(
    name = "fes_image_push",
    image = ":fes_image",
    registry = "us.gcr.io",
    repository = "mushu-194218/fes",
    tag = "dev",
)
