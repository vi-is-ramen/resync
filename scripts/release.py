import lib  # pyright: ignore[reportImplicitRelativeImport]

CARGO_TOML = lib.Cargo.manifest()


def is_new_version() -> bool:
    if "dev" in CARGO_TOML.package.version:  # pyright: ignore[reportUnknownMemberType, reportAttributeAccessIssue]
        return False
    for line in lib.rq.urlopen("https://index.crates.io/re/sy/resync"):  # pyright: ignore[reportAny]
        ver = lib.j.loads(line)  # pyright: ignore[reportAny]
        if ver["vers"] == CARGO_TOML.package.version:  # pyright: ignore[reportUnknownMemberType, reportAttributeAccessIssue]
            return False
    return True


def publish():
    _ = lib.sp.run(["cargo", "publish"], check=True)


def tag():
    name = "v" + CARGO_TOML.package.version  # pyright: ignore[reportUnknownMemberType, reportAttributeAccessIssue, reportUnknownVariableType]

    sha = (
        lib.sp.run(["git", "rev-parse", "HEAD"], check=True, stdout=lib.sp.PIPE)
        .stdout.decode("utf-8")
        .strip()
    )

    _ = lib.sp.run(
        [  # pyright: ignore[reportUnknownArgumentType]
            "gh",
            "api",
            "/repos/vi-is-ramen/resync/git/refs",
            "-X",
            "POST",
            "-H",
            "Accept: application/vnd.github.v3+json",
            "-F",
            "ref=refs/tags/" + name,
            "-F",
            "sha=" + sha,
        ]
    )


def main():
    if is_new_version():
        publish()
        tag()


main()
