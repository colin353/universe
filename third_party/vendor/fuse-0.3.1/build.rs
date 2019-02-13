extern crate pkg_config;

#[cfg(not(target_os = "macos"))]
static LIBFUSE_NAME: &str = "fuse";

#[cfg(target_os = "macos")]
static LIBFUSE_NAME: &str = "osxfuse";

fn main() {
    match pkg_config::Config::new()
        .atleast_version("2.6.0")
        .probe(LIBFUSE_NAME)
    {
        Ok(x) => (),
        Err(x) => panic!("Bad luck! {:?}", x),
    };
}
