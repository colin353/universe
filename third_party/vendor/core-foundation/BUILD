package(default_visibility = [
  "//visibility:public",
])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
        name = "core_foundation",
        edition = "2015",
        srcs = glob(["*.rs"]),
        deps = [
                "//third_party:libc",
                "//third_party/vendor/core-foundation-sys:core_foundation_sys",
        ]
)
