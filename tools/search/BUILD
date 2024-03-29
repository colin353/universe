package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library", "rust_grpc_library")

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
        "html/opensearch.xml",
        "html/detail.html",
        "html/detail_folder.html",
        "html/detail_template.html",
        "html/index.html",
        "html/favicon.png",
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
        "//third_party:tokio",
        "//util/flags",
        "//util/markdown",
        "//util/tmpl",
        "//util/ws",
        "//util/ws:ws_utils",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:tls_api_stub",
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
        "html/opensearch.xml",
        "html/detail.html",
        "html/detail_folder.html",
        "html/detail_template.html",
        "html/index.html",
        "html/markdown.html",
        "html/favicon.png",
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
        "//third_party:tokio",
        "//util/flags",
        "//util/markdown",
        "//util/tmpl",
        "//util/ws",
        "//util/ws:ws_utils",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "search_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":search_grpc_rust",
        "//third_party:futures",
        "//util/grpc_tls",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:tls_api",
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
        "@rules_rust//proto/raze:grpc",
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
        "//third_party:byteorder",
        "//third_party:lazy_static",
        "//third_party:regex",
        "//util/itertools",
        "//util/pool",
        "//util/primitive",
        "//util/sstable",
        "@rules_rust//proto/raze:protobuf",
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
    name = "entity_lib",
    srcs = ["entity_lib.rs"],
    edition = "2018",
    deps = [
        ":extract_lib",
        ":search_utils",
        ":search_proto_rust",
        "//plume",
    ],
)

rust_test(
    name = "entity_test",
    crate = ":entity_lib",
)


rust_library(
    name = "indexer_lib",
    srcs = ["indexer_lib.rs"],
    edition = "2018",
    deps = [
        ":pagerank",
        ":entity_lib",
        ":search_proto_rust",
        ":search_utils",
        "//plume",
        "//third_party:byteorder",
        "//tools/search/language_specific",
        "@rules_rust//proto/raze:protobuf",
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
