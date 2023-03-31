package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")

rust_library(
    name = "rainbow",
    srcs = ["lib.rs"],
    deps = [
        "//util/gfile",
        "//auth:auth_client",
        "//util/cli",
        "//third_party:sha256",
        "//third_party:hyper",
        "//third_party:hyper_tls",
    ],
)

rust_binary(
    name = "cli",
    srcs = ["cli.rs"],
    deps = [
        "//auth:auth_client",
        "//util/cli",
        ":rainbow",
        "//util/flags",
    ],
)

rust_test(
    name = "rainbow_test",
    crate = ":rainbow",
)
