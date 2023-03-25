package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_library(
    name = "bus",
    srcs = [
        "lib.rs",
        "serializable.rs",
        "varint.rs",
        "pack.rs",
        "repeated_field.rs",
        "encoded_struct.rs",
    ],
)

rust_library(
    name = "fmt",
    srcs = [
        "fmt.rs",
    ],
    deps = [
        ":parser",
        "//util/ggen",
    ]
)

rust_binary(
    name = "busfmt",
    srcs = [ "busfmt.rs" ],
    deps = [
        ":parser",
        ":fmt",
        "//util/flags",
    ]
)

rust_test(
    name = "fmt_test",
    crate = ":fmt",
)

rust_test(
    name = "bus_test",
    crate = ":bus",
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

rust_library(
    name = "bus_rpc",
    srcs = [
        "rpc.rs",
    ],
    deps = [
        ":bus",
        "//third_party:futures",
        "//third_party:hyper",
        "//third_party:hyper_tls",
        "//third_party:tokio",
    ]
)
