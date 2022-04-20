package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "tui",
    srcs = ["tui.rs"],
    edition = "2018",
    deps = [
        "//auth:auth_client",
        "//third_party/raw_tty",
        "//tools/search:search_client",
        "//tools/search:search_grpc_rust",
        "//util/cli",
        "//util/flags",
        "//util/init",
        "//util/tui",
    ],
)
