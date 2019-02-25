package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")

rust_binary(
    name = "read_recordio",
    srcs = [
        "read_recordio.rs",
    ],
    deps = [
        "//largetable:largetable_proto_rust",
        "//util/recordio",
    ],
)
