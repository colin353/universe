package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")

proto_library(
    name = "primitive_proto",
    srcs = ["primitive.proto"],
)

rust_proto_library(
    name = "primitive_proto_rust",
    deps = [":primitive_proto"],
)

rust_library(
    name = "primitive",
    srcs = [
        "primitive.rs",
    ],
    deps = [
        ":primitive_proto_rust",
        "@rules_rust//proto/raze:protobuf",
    ],
)
