import sys

import lib

FEATURES = "--no-default-features", *sys.argv[1:]


def unit_test():
    print("\033[1m=== CARGO TEST ===\033[0m")
    lib.sp.run(["cargo", "test", FEATURES], check=True)


def main():
    unit_test()
