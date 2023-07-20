package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "server",
    srcs = [
        "server.rs"
    ],
    deps = [
        ":server_lib",
        "//util/ws",
        "//util/flags",
        "//third_party:tokio",
        "//auth:auth_client",
        "//tools/queue:queue_bus",
        "//util/bus:bus_rpc",
    ]
)

rust_library(
    name = "server_lib",
    srcs = [
        "lib.rs",
        "render.rs",
    ],
    data = [
        "change.html",
        "diff_view.html",
        "homepage.html",
        "modified_files.html",
        "template.html",
    ],
    deps = [
        "//third_party:base64",
        "//util/tmpl",
        "//util/tmpl:escape",
        "//util/ws",
        "//auth:auth_client",
        "//tools/src:service",
        "//tools/src:core",
        "//tools/queue:queue_bus",
        "//util/bus:bus_rpc",
        "//tools/search/language_specific",
    ],
)


rust_test(
    name = "server_lib_test",
    data = [
        "change.html",
        "diff_view.html",
        "homepage.html",
        "modified_files.html",
        "template.html",
    ],
    crate = "server_lib",
)

