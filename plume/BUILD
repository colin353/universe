package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")

proto_library(
    name = "plume_proto",
    srcs = ["plume.proto"],
)

rust_proto_library(
    name = "plume_proto_rust",
    deps = [":plume_proto"],
)

rust_library(
    name = "plume",
    srcs = [
        "lib.rs",
    ],
    deps = [
        ":plume_proto_rust",
        "//third_party:lazy_static",
        "//util/primitive",
        "//util/shard_lib",
        "//util/sstable",
    ],
)

rust_binary(
    name = "example",
    srcs = [
        "example.rs",
    ],
    deps = [
        ":plume",
    ],
)

rust_test(
    name = "plume_test",
    crate = ":plume",
)

rust_test(
    name = "tests",
    srcs = ["tests.rs"],
    deps = [":plume"],
)
