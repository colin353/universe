package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "ci",
    srcs = [ "main.rs" ],
    deps = [
        "//tools/queue:queue_client",
        "//tools/src:service",
        "//tools/lockserv:lockserv_client",
        "//tools/src:core",
        "//tools/src:src_lib",
        "//util/bus",
        "//third_party:tokio",
        "//third_party:futures",
    ],
)

rust_test(
    name = "ci_test",
    crate = ":ci",
)
