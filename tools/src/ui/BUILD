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
                "//third_party:tokio",
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
        "//util/ws",
        "//tools/src:service",
        "//tools/src:core",
        "//util/bus:bus_rpc",
    ],
    edition="2018",
)

