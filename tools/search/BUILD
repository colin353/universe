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
        "html/detail_folder.html",
        "html/detail_template.html",
        "html/index.html",
        "html/markdown.html",
        "html/results.html",
        "html/sidebar.html",
        "html/template.html",
        "//homepage/js:code_pad",
        "//tools/search/js:flow_view",
        "//tools/search/js:search_box",
    ],
    edition = "2018",
    deps = [
        "search_grpc_rust",
        ":search_lib",
        ":server_lib",
        "//auth:auth_client",
        "//third_party:base64",
        "//third_party:json",
        "//util/flags",
        "//util/markdown",
        "//util/tmpl",
        "//util/ws",
        "//util/ws:ws_utils",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_binary(
    name = "index_and_search",
    srcs = [
        "index_and_search.rs",
        "render.rs",
        "webserver.rs",
    ],
    data = [
        "html/detail.html",
        "html/detail_folder.html",
        "html/detail_template.html",
        "html/index.html",
        "html/markdown.html",
        "html/results.html",
        "html/sidebar.html",
        "html/template.html",
        "//homepage/js:code_pad",
        "//tools/search/js:flow_view",
        "//tools/search/js:search_box",
    ],
    edition = "2018",
    deps = [
        "search_grpc_rust",
        ":extract_lib",
        ":indexer_lib",
        ":search_lib",
        ":search_utils",
        ":server_lib",
        "//auth:auth_client",
        "//third_party:base64",
        "//third_party:json",
        "//util/flags",
        "//util/markdown",
        "//util/tmpl",
        "//util/ws",
        "//util/ws:ws_utils",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_image(
    name = "index_and_search_img",
    base = "@glibc_base//image",
    binary = ":index_and_search",
)

docker_push(
    name = "index_and_search_img_push",
    image = ":index_and_search_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/codesearch_combined",
    tag = "dev",
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
    rustc_flags = [
        #"--cfg=debug_scoring",
    ],
    deps = [
        ":search_grpc_rust",
        ":search_utils",
        "//third_party:aho_corasick",
        "//third_party:lazy_static",
        "//third_party:regex",
        "//util/itertools",
        "//util/pool",
        "//util/sstable/v2:sstable2",
        "@io_bazel_rules_rust//proto/raze:protobuf",
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

rust_image(
    name = "indexer_img",
    base = "@glibc_base//image",
    binary = ":indexer",
)

docker_push(
    name = "indexer_img_push",
    image = ":indexer_img",
    registry = "us.gcr.io",
    repository = "mushu-194218/search_indexer",
    tag = "dev",
)

rust_library(
    name = "indexer_lib",
    srcs = ["indexer_lib.rs"],
    edition = "2018",
    deps = [
        ":pagerank",
        ":search_proto_rust",
        ":search_utils",
        "//plume",
        "//tools/search/language_specific",
    ],
)

rust_library(
    name = "pagerank",
    srcs = ["pagerank.rs"],
    edition = "2018",
    deps = [
        ":search_proto_rust",
        ":search_utils",
        "//plume",
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
