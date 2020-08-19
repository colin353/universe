package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//container:container.bzl", "container_image")
load("@bazel_tools//tools/build_defs/pkg:pkg.bzl", "pkg_tar")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

rust_binary(
    name = "review",
    srcs = ["main.rs"],
    deps = [
        "//auth:auth_client",
        "//tasks:task_client",
        "//tools/queue:queue_client",
        "//util/flags",
        "//util/tmpl",
        "//util/ws",
        "//weld",
        "//weld/review:server",
    ],
)

rust_library(
    name = "server",
    srcs = [
        "render.rs",
        "server.rs",
    ],
    data = [
        "change.html",
        "diff_view.html",
        "homepage.html",
        "modified_files.html",
        "template.html",
    ],
    deps = [
        "//auth:auth_client",
        "//tasks:task_client",
        "//third_party:base64",
        "//tools/queue:queue_client",
        "//util/flags",
        "//util/tmpl",
        "//util/ws",
        "//weld",
    ],
)

pkg_tar(
    name = "static_files",
    srcs = glob([
        "static/**",
    ]),
    strip_prefix = ".",
)

pkg_tar(
    name = "codepad",
    srcs = ["//homepage/js:code_pad"],
    package_dir = "/static",
)

container_image(
    name = "review_base_img",
    base = "@glibc_base//image",
    tars = [
        ":static_files",
        ":codepad",
    ],
)

rust_image(
    name = "review_img",
    srcs = ["main.rs"],
    base = ":review_base_img",
    deps = [
        "//auth:auth_client",
        "//tasks:task_client",
        "//tools/queue:queue_client",
        "//util/flags",
        "//util/tmpl",
        "//util/ws",
        "//weld",
        "//weld/review:server",
    ],
)

docker_push(
    name = "review_img_push",
    image = ":review_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/review",
    tag = "dev",
)
