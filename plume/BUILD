package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")

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
        "//util/itertools",
        "//util/pool",
        "//util/primitive",
        "//util/recordio",
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
    deps = [
        ":plume",
        "//util/recordio",
        "//util/sstable",
    ],
)
