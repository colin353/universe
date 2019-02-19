package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_library(
    name = "ws",
    srcs = [
        "ws.rs",
    ],
    deps = [
        "//third_party:hyper",
    ],
)

rust_binary(
    name = "ws_example",
    srcs = [
        "ws_example.rs",
    ],
    data = ["template.html"],
    deps = [
        ":ws",
        "//util/tmpl",
    ],
)
