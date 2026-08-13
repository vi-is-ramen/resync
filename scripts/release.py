import lib

CARGO_TOML = lib.Cargo.manifest()


def is_new_version() -> bool:
    for line in lib.rq.urlopen("https://index.crates.io/re/sy/resync"):
        ver = lib.json.loads(line)
        if ver["vers"] == CARGO_TOML.package.version:
            return False


def publish():
    lib.sp.run(["cargo", "publish"], check=True)


def tag():
    name = "v" + CARGO_TOML.package.version

    sha = (
        lib.sp.run(["git", "rev-parse", "HEAD"], check=True, stdout=lib.sp.PIPE)
        .stdout.decode("utf-8")
        .strip()
    )

    lib.sp.run(
        [
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
