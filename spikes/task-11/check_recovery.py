"""Check that a scan after a fuel trap succeeds in a fresh store and instance."""

import argparse

from wasmtime import Config, Engine, Store, WasmtimeError
from wasmtime.component import Component, Linker

from measure import call, instantiate


def check(component_path):
    config = Config()
    config.consume_fuel = True
    engine = Engine(config)
    linker = Linker(engine)
    linker.add_wasip2()
    component = Component.from_file(engine, component_path)

    store = Store(engine)
    store.set_fuel(10**15)
    _, scan = instantiate(store, linker, component)
    store.set_fuel(0)
    try:
        call(store, scan, b"fn limited() {}")
    except WasmtimeError as error:
        if "all fuel consumed" not in str(error):
            raise
    else:
        raise AssertionError("expected the first scan to exhaust its budget")

    store = Store(engine)
    store.set_fuel(10**15)
    _, scan = instantiate(store, linker, component)
    nodes = call(store, scan, b"fn next() {}")
    assert [node.label for node in nodes] == ["fn next"]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("component")
    check(parser.parse_args().component)
