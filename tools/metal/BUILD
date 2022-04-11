package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "metal_proto",
    srcs = ["metal.proto"],
)

rust_grpc_library(
    name = "metal_grpc_rust",
    deps = [":metal_proto"],
)

rust_library(
    name = "core",
    srcs = ["core.rs"],
    edition = "2018",
    deps = [
        ":metal_grpc_rust",
    ]
)


rust_library(
    name = "config",
    srcs = ["config.rs"],
    data = ["metal.ccl"],
    edition = "2018",
    deps = [
        ":metal_grpc_rust",
        "//util/ccl",
        "//util/ggen",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "config_test",
    crate = ":config",
    # Not sure why this isn't inherited from the other target...
    data = ["metal.ccl"],
)

rust_library(
    name = "state",
    srcs = ["state.rs"],
    edition = "2018",
    deps = [
        ":metal_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "service",
    srcs = ["service.rs"],
    edition = "2018",
    deps = [
        ":metal_grpc_rust",
        ":core",
        ":state",
        ":diff",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_library(
    name = "diff",
    srcs = ["diff.rs"],
    edition = "2018",
    deps = [
        ":metal_grpc_rust",
    ],
)

rust_library(
    name = "monitor",
    srcs = [
        "monitor.rs",
        "process.rs"
    ],
    edition = "2018",
    deps = [
        ":core",
        ":metal_grpc_rust",
        "//third_party:libc",
    ],
)

rust_test(
    name = "service_test",
    crate = ":service",
)

rust_binary(
    name = "cli",
    srcs = [
        "cli.rs",
    ],
    edition = "2018",
    deps = [
        ":config",
        ":diff",
        ":metal_grpc_rust",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_binary(
    name = "metal",
    srcs = [ 
        "main.rs",
    ],
    edition = "2018",
    deps = [
        ":service",
        ":state",
        ":monitor",
        ":metal_grpc_rust",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ]
)

