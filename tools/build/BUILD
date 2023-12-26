package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_binary")

rust_binary(
    name = "build", 
    srcs = [
        "main.rs",
        "core.rs",
        "cargo.rs",
        "context.rs",
        "plugins.rs",
        "actions.rs",
        "exec.rs",
    ],
    deps = [
      "//third_party:hyper",
      "//third_party:hyper_tls",
      "//third_party:futures",
      "//third_party:tokio",
    ],
)

rust_test(
    name = "build_test", 
    crate = ":build",
)
