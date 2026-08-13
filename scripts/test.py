import os
import sys

import lib  # pyright: ignore[reportImplicitRelativeImport]

FEATURES = "--no-default-features", *sys.argv[1:]


def unit_test():
    print("\033[1m=== CARGO TEST ===\033[0m")
    _ = lib.sp.run(["cargo", "test", *FEATURES], check=True)


def integration_and_regression_test():
    print("\033[1m=== INTEG TEST ===\033[0m")
    os.chdir("testing/util")

    with open("../Cargo.toml", "rb") as f:
        cargo_toml = lib.toml.load(f)

    for bin in cargo_toml["bin"]:  # pyright: ignore[reportAny]
        _ = lib.sp.run(["cargo", "run", "-p", "resync-tests", "--bin", bin["name"]], check = True)


def main():
    unit_test()
    integration_and_regression_test()


main()
