package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_library(
    name = "ccl",
    srcs = [ 
      "lib.rs",
      "ast.rs",
      "import_resolver.rs",
      "fmt.rs",
      "exec.rs",
      "eval.rs",
      "eval_tests.rs",
    ],
    deps = [ 
       "//util/ggen",
    ],
    edition = "2018",
)

rust_binary(
    name = "cclfmt",
    srcs = [
        "cclfmt.rs",
    ],
    deps = [
        ":ccl",
        "//util/flags",
    ],
    edition = "2018",
)

rust_binary(
    name = "cclc",
    srcs = [
        "cclc.rs",
    ],
    deps = [
        ":ccl",
        "//util/flags",
    ],
    edition = "2018",
)


rust_test(
    name = "ccl_test",
    crate = ":ccl",
)

