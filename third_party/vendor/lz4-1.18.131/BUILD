package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_library(
    name = "lz4",
    srcs = [ 
      "lib.rs",
      "liblz4.rs",
      "encoder.rs",
      "decoder.rs",
      "block/mod.rs",
      "lz4_sys.rs",
    ],
)
