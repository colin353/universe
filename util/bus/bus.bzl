load("@rules_rust//rust/private:rustc.bzl", "rustc_compile_action")
load("@rules_rust//rust:defs.bzl", "rust_common")
load("@rules_rust//rust/private:utils.bzl", "compute_crate_name", "determine_output_hash", "find_toolchain", "transform_deps")

def register_bus_toolchain():
  native.register_toolchains(str(Label("@rules_rust//proto:default-proto-toolchain")))

def _bus_library_rust_impl(ctx):
  output_dir = "%s.rust" % ctx.attr.name
  lib_rs = ctx.actions.declare_file("%s/lib.rs" % output_dir)
  ctx.actions.run_shell(
      inputs = ctx.files.srcs,
      tools = [ ctx.file._compiler ],
      command = "%s %s > %s" % (
          ctx.file._compiler.path,
          " ".join([x.path for x in ctx.files.srcs]),
          lib_rs.path,
      ),
      progress_message = "bus: generating code...",
      outputs = [lib_rs],
  )

  output_hash = determine_output_hash(lib_rs, ctx.label)
  rust_lib = ctx.actions.declare_file("%s/lib%s-%s.rlib" % (
      output_dir,
      ctx.attr.name,
      output_hash,
  ))

  toolchain = find_toolchain(ctx)
  proto_toolchain = ctx.toolchains[Label("@rules_rust//proto:toolchain")]

  return rustc_compile_action(
      ctx = ctx,
      attr = ctx.attr,
      toolchain = toolchain,
      crate_info = rust_common.create_crate_info(
          name = ctx.attr.name,
          type = "rlib",
          root = lib_rs,
          srcs = depset([lib_rs]),
          deps = depset([ctx.attr._compiler, ctx.attr._bus_lib]),
          proc_macro_deps = depset([]),
          aliases = {},
          output = rust_lib,
          edition = proto_toolchain.edition,
          rustc_env = {},
          is_test = False,
          compile_data = depset([]),
          wrapped_crate_type = None,
          owner = ctx.label,
      ),
      output_hash = output_hash,
  )

rust_bus_library = rule(
    implementation = _bus_library_rust_impl,
    attrs = {
        "srcs": attr.label_list(allow_files = True),
        "_compiler": attr.label(
            allow_single_file = True,
            default = Label("//util/bus/codegen"),
        ),
        "_bus_lib" : attr.label(
            allow_single_file = True,
            default = Label("//util/bus"),
        ),
        "_cc_toolchain": attr.label(
            default = Label("@bazel_tools//tools/cpp:current_cc_toolchain"),
        ),
        "_process_wrapper": attr.label(
            default = Label("@rules_rust//util/process_wrapper"),
            executable = True,
            allow_single_file = True,
            cfg = "exec",
        )
    },
    fragments = ["cpp"],
    host_fragments = ["cpp"],
    incompatible_use_toolchain_transition = True,
    toolchains=[
        "@rules_rust//rust:toolchain",
        "@rules_rust//proto:toolchain",
        "@bazel_tools//tools/cpp:toolchain_type",
    ],
)
