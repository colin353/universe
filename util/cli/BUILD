package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "example",
    srcs = [ "example.rs" ],
    edition = "2018",
    deps = [
        ":cli",
    ],
)

rust_library(
    name = "cli",
    srcs = [
        "cli.rs",
    ],
    edition = "2018",
    deps = [
        "//auth:auth_client",
        "//third_party:rand",
    ],
)

rust_test(
    name = "cli_test",
    crate = ":cli",
)

rust_binary(
    name = "prodaccess",
    srcs = ["prodaccess.rs"],
    edition = "2018",
    deps = [
        ":cli",
        "//auth:auth_client",
        "//util/flags",
        "//util/init",
    ],
)
