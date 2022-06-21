package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_library(
    name = "car",
    srcs = [
        "lib.rs",
        "serializable.rs",
        "varint.rs",
        "pack.rs",
        "repeated_field.rs",
        "struct_test.rs",
    ],
)

rust_test(
    name = "car_test",
    crate = ":car",
)

rust_library(
    name = "parser",
    srcs = [
        "parser.rs",
        "ast.rs",
    ],
    deps = [
        "//util/ggen"
    ]
)

rust_test(
    name = "parser_test",
    crate = ":parser",
)
