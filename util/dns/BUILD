package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_test", "rust_library", "rust_binary")

rust_library(
    name = "dns",
    srcs = [
        "lib.rs",
        "packet.rs",
    ],
)

rust_test(
    name = "dns_test",
    crate = ":dns",
)

rust_binary(
    name = "example",
    srcs = [ "example.rs" ],
    deps = [ ":dns" ],
    edition = "2018",
)
