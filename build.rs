//!

extern crate version_check as vc;

fn main()
{
    println!("cargo:rerun-if-changed=build.rs");

    let channel = vc::Channel::read()
        .or(vc::Channel::parse("1.0.0")) // default to stable
        .unwrap();

    if channel.is_nightly()
    {
        println!("cargo:rustc-cfg=nightly");
    }
    if channel.is_beta()
    {
        println!("cargo:rustc-cfg=beta");
    }
    if channel.is_stable()
    {
        println!("cargo:rustc-cfg=stable");
    }
}
