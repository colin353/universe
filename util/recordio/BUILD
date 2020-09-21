package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")

rust_library(
    name = "recordio",
    srcs = [
        "recordio.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:byteorder",
        "//util/itertools",
        "//util/primitive",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "recordio_test",
    crate = ":recordio",
)
