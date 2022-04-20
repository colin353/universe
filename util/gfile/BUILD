package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "gfile",
    srcs = ["lib.rs"],
    edition = "2018",
    deps = [
        "//auth:auth_client",
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:hyper_tls",
        "//third_party:json",
        "//third_party:tokio",
        "//util/requests",
        "//util/ws:ws_utils",
    ],
)

rust_test(
    name = "gfile_test",
    crate = ":gfile",
    deps = [
        "//util/primitive",
        "//util/recordio",
    ],
)
