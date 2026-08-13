import json as j
import subprocess as sp
import urllib.request as rq

import tomllib as toml


class Cargo:
    class Manifest(dict[object, object]):
        def __init__(self, inner: dict[object, object]) -> None:
            super().__init__()
            self.update(inner)

        def __getattr__(self, name: str) -> object:
            if name[0] == "_":
                return object.__getattr__(self, name)  # pyright: ignore[reportUnknownVariableType, reportUnknownMemberType, reportAttributeAccessIssue]
            else:
                val = super().__getitem__(name)

                if isinstance(val, dict):
                    return Cargo.Manifest(val)  # pyright: ignore[reportUnknownArgumentType]

                return val

    @staticmethod
    def manifest():
        with open("Cargo.toml", "rb") as f:
            return Cargo.Manifest(toml.load(f))  # pyright: ignore[reportArgumentType]


__all__ = ["Cargo", "j", "rq", "sp", "toml"]
