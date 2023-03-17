package(default_visibility = [
  "//visibility:public",
])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
        name = "security_framework",
        srcs = glob(["**/*.rs"]),
        deps = [
                "//third_party/vendor/security-framework-sys:security_framework_sys",
                "//third_party/vendor/core-foundation-sys:core_foundation_sys",
                "//third_party/vendor/core-foundation:core_foundation",
                "//third_party:libc",
                "//third_party/vendor/bitflags-1.0.4:bitflags",
        ]
)
