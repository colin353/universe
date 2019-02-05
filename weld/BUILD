package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "weld_proto",
    srcs = ["weld.proto"],
)

rust_proto_library(
    name = "weld_proto_rust",
    deps = [":weld_proto"],
)

rust_grpc_library(
    name = "weld_grpc_rust",
    deps = [":weld_proto"],
)

rust_library(
    name = "weld",
    srcs = [
        "weld.rs",
    ],
    deps = [
        ":weld_grpc_rust",
        ":weld_proto_rust",
        "//third_party:futures",
        "//third_party:time",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "weld_test",
    deps = [":weld"],
)
