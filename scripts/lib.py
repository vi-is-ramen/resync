import json as j
import subprocess as sp
import urllib.request as rq

import tomllib as toml


class Cargo:
    class Manifest(dict):
        def __init__(self, inner: dict) -> None:
            self.update(inner)

        def __getattr__(self, name: str) -> object:
            if name[0] == '_':
                return super().__getattr__(self, name)
            else:
                val = super().__getitem__(self, name)

                if isinstance(val, dict):
                    return Cargo.Manifest(val)

                return val

    @staticmethod
    def manifest():
        with open("Cargo.toml", "rb") as f:
            return Cargo.Manifest(toml.load(f))
