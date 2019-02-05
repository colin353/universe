package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")

rust_library(
    name = "recordio",
    srcs = [
        "recordio.rs",
        ],
    deps = [
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "//third_party:byteorder",
        ]
    )


rust_test(
    name = "recordio_test",
    deps = [ ":recordio" ],
    )

