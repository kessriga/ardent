"""Run one release component sample in a fresh host process, from the repository root."""

import argparse
import json
from pathlib import Path
from statistics import median
from time import perf_counter_ns

from wasmtime import Config, Engine, Store, WasiConfig
from wasmtime.component import Component, Linker


def elapsed_ms(start):
    return (perf_counter_ns() - start) / 1_000_000


def call(store, function, *args):
    value = function(store, *args)
    function.post_return(store)
    return value


def instantiate(store, linker, component):
    store.set_wasi(WasiConfig())
    instance = linker.instantiate(store, component)
    interface = instance.get_export_index(store, "ardent:format/scanner@0.1.0")
    if interface is None:
        raise ValueError("component does not export ardent:format/scanner@0.1.0")

    def lookup(name):
        index = instance.get_export_index(store, name, interface)
        if index is None or (function := instance.get_func(store, index)) is None:
            raise ValueError(f"scanner is missing the {name} function")
        return function

    return lookup("describe"), lookup("scan")


def measure(component_path, cache_config):
    source = Path("crates/ardent-format/tests/rust/modules.rs").read_bytes()
    start = perf_counter_ns()
    config = Config()
    if cache_config:
        config.cache = cache_config
    engine = Engine(config)
    linker = Linker(engine)
    linker.add_wasip2()
    component = Component.from_file(engine, component_path)
    store = Store(engine)
    describe, scan = instantiate(store, linker, component)
    metadata = call(store, describe)
    assert (metadata.name, metadata.version, metadata.files) == ("rust", "0.1.0", ["*.rs"])
    load_ms = elapsed_ms(start)

    start = perf_counter_ns()
    nodes = call(store, scan, source)
    first_ms = elapsed_ms(start)
    assert len(nodes) == 11
    outer, inner = nodes[4:6]
    assert (outer.label, outer.span.start, outer.span.end) == ("fn run", 286, 457)
    assert (inner.label, inner.parent) == ("fn count", 4)

    repeated = []
    for _ in range(10):
        start = perf_counter_ns()
        result = call(store, scan, source)
        repeated.append(elapsed_ms(start))
        assert result == nodes

    broken = b"impl Engine {\n    pub fn run(&self) {}\n\n    fn helper(\n\n    fn after(&self) {}\n}\n"
    error = call(store, scan, broken)
    assert (error.line, error.column) == (6, 5)
    assert call(store, scan, b"fn other() {}")[0].label == "fn other"
    assert call(store, scan, source) == nodes
    return dict(load_ms=load_ms, first_scan_ms=first_ms, repeat_scan_ms=median(repeated))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("component")
    parser.add_argument("--cache-config")
    args = parser.parse_args()
    print(json.dumps(measure(args.component, args.cache_config)))
