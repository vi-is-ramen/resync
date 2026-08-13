//! Custom build script for detecting compiler channel.

extern crate rustversion as rv;

#[rv::nightly]
fn a()
{
    println!("cargo:rustc-cfg=nightly")
}

#[rv::not(nightly)]
fn a() {}

#[rv::beta]
fn b()
{
    println!("cargo:rustc-cfg=beta")
}

#[rv::not(beta)]
fn b() {}

#[rv::stable]
fn c()
{
    println!("cargo:rustc-cfg=stable")
}

#[rv::not(stable)]
fn c() {}

fn main()
{
    println!("cargo:rerun-if-changed=build.rs");

    a();
    b();
    c();
}
