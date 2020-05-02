package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "search_proto",
    srcs = ["search.proto"],
)

rust_proto_library(
    name = "search_proto_rust",
    deps = [":search_proto"],
)

rust_grpc_library(
    name = "search_grpc_rust",
    deps = [":search_proto"],
)

rust_binary(
    name = "server",
    srcs = [
        "render.rs",
        "server.rs",
        "webserver.rs",
    ],
    data = [
        "html/detail.html",
        "html/index.html",
        "html/results.html",
        "html/template.html",
    ],
    edition = "2018",
    deps = [
        "search_grpc_rust",
        ":search_lib",
        ":server_lib",
        "//auth:auth_client",
        "//third_party:base64",
        "//util/flags",
        "//util/tmpl",
        "//util/ws",
        "//util/ws:ws_utils",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_image(
    name = "search_server_img",
    base = "//weld/review:review_base_img",
    binary = ":server",
)

docker_push(
    name = "search_server_img_push",
    image = ":search_server_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/search_server",
    tag = "dev",
)

rust_library(
    name = "search_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":search_grpc_rust",
        "//third_party:tls_api_openssl",
        "//util/grpc_tls",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@raze__httpbis__0_7_0//:httpbis",
    ],
)

rust_library(
    name = "server_lib",
    srcs = ["server_lib.rs"],
    edition = "2018",
    deps = [
        ":search_grpc_rust",
        ":search_lib",
        "//auth:auth_client",
        "@io_bazel_rules_rust//proto/raze:grpc",
    ],
)

rust_binary(
    name = "search",
    srcs = ["search.rs"],
    edition = "2018",
    deps = [
        ":search_lib",
        ":search_proto_rust",
        "//util/flags",
    ],
)

rust_library(
    name = "search_lib",
    srcs = ["search_lib.rs"],
    edition = "2018",
    deps = [
        ":search_grpc_rust",
        ":search_utils",
        "//third_party:aho_corasick",
        "//third_party:lazy_static",
        "//third_party:regex",
        "//util/itertools",
        "//util/pool",
        "//util/sstable",
    ],
)

rust_library(
    name = "extract_lib",
    srcs = ["extract_lib.rs"],
    edition = "2018",
    deps = [
        ":search_proto_rust",
        "//util/recordio",
    ],
)

rust_binary(
    name = "indexer",
    srcs = ["indexer.rs"],
    edition = "2018",
    deps = [
        ":extract_lib",
        ":indexer_lib",
        ":search_proto_rust",
        "//plume",
        "//util/flags",
    ],
)

rust_library(
    name = "indexer_lib",
    srcs = ["indexer_lib.rs"],
    edition = "2018",
    deps = [
        ":search_proto_rust",
        ":search_utils",
        "//plume",
        "//tools/search/language_specific",
    ],
)

rust_library(
    name = "search_utils",
    srcs = ["utils.rs"],
)

rust_test(
    name = "indexer_test",
    crate = ":indexer_lib",
)

rust_binary(
    name = "extract",
    srcs = ["extract.rs"],
    edition = "2018",
    deps = [
        ":extract_lib",
        "//util/flags",
    ],
)
