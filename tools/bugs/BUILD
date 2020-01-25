package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "bugs_proto",
    srcs = ["bugs.proto"],
)

rust_grpc_library(
    name = "bugs_grpc_rust",
    deps = [":bugs_proto"],
)

rust_library(
    name = "bug_server_lib",
    srcs = ["server_lib.rs"],
    deps = [
        ":bugs_grpc_rust",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "bug_server_lib_test",
    crate = ":bug_server_lib",
    deps = [
        "//largetable:largetable_test",
    ],
)

rust_binary(
    name = "bug_server",
    srcs = ["server.rs"],
    deps = [
        ":bug_server_lib",
        ":bugs_grpc_rust",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "bug_client",
    srcs = ["client.rs"],
    deps = [":bugs_grpc_rust"],
)

rust_binary(
    name = "bug_util",
    srcs = ["util.rs"],
    edition = "2018",
    deps = [
        ":bug_client",
        ":bugs_grpc_rust",
        "//auth:auth_client",
        "//util/cli",
        "//util/flags",
    ],
)

rust_image(
    name = "bug_server_img",
    binary = ":bug_server",
)

docker_push(
    name = "bug_server_img_push",
    image = ":bug_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/bugs",
    tag = "dev",
)
