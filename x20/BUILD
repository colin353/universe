package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "x20_proto",
    srcs = ["x20.proto"],
)

rust_grpc_library(
    name = "x20_grpc_rust",
    deps = [":x20_proto"],
)

rust_library(
    name = "x20_server_lib",
    srcs = ["server_lib.rs"],
    deps = [
        ":x20_grpc_rust",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "x20_server_lib_test",
    crate = ":x20_server_lib",
    deps = [
        "//largetable:largetable_test",
    ],
)

rust_binary(
    name = "x20_server",
    srcs = [
        "server.rs",
    ],
    deps = [
        ":x20_grpc_rust",
        ":x20_server_lib",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "x20_client",
    srcs = ["client.rs"],
    deps = [
        ":x20_grpc_rust",
        "//util/grpc_tls",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_binary(
    name = "x20",
    srcs = [
        "config.rs",
        "subprocess.rs",
        "util.rs",
        "x20.rs",
    ],
    deps = [
        ":x20_client",
        ":x20_grpc_rust",
        "//third_party:json",
        "//third_party:rand",
        "//util/cli",
        "//util/flags",
        "//util/recordio",
    ],
)

rust_test(
    name = "x20_util_test",
    crate = ":x20",
)

rust_image(
    name = "x20_server_img",
    base = "@glibc_base//image",
    binary = ":x20_server",
)

docker_push(
    name = "x20_server_img_push",
    image = ":x20_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/x20",
    tag = "dev",
)

rust_binary(
    name = "x20_webserver",
    srcs = ["webserver.rs"],
    data = [
        "index.html",
        "x20.sh",
    ],
    deps = [
        ":x20_client",
        "//util/flags",
        "//util/ws",
    ],
)

rust_image(
    name = "x20_webserver_img",
    binary = ":x20_webserver",
)

docker_push(
    name = "x20_webserver_img_push",
    image = ":x20_webserver_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/x20_web",
    tag = "dev",
)
