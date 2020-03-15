package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "tui",
    srcs = ["tui.rs"],
    edition = "2018",
    deps = [
        "//tools/search:search_client",
        "//tools/search:search_grpc_rust",
        "//util/tui",
    ],
)
