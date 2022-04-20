package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
   name = "sync",
   srcs = [ "sync.rs" ],
   edition = "2018",
   deps = [ 
         "//util/init",
         "//util/gfile",
         "//util/flags",
         "//util/cli",
         "//auth:auth_client",
   ],
)
