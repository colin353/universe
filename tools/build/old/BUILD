package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_binary")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "build_proto",
    srcs = ["build.proto"],
)

rust_grpc_library(
    name = "build_grpc_rust",
    deps = [":build_proto"],
)

rust_library(
    name = "build", 
    srcs = [
        "lib.rs",
        "exec.rs",
        "file_resolver.rs",
        "target_resolver.rs",
    ],
    deps = [ ":build_grpc_rust" ],
    edition = "2018",
)

rust_test(
    name = "build_test", 
    crate = ":build",
)
