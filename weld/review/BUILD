package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

rust_binary(
    name = "review",
    srcs = ["main.rs"],
    deps = [
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
        "homepage.html",
        "style.css",
        "template.html",
    ],
    deps = [
        "//util/flags",
        "//util/tmpl",
        "//util/ws",
        "//weld",
    ],
)