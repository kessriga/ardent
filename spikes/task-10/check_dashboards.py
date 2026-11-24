"""Run with uv run spikes/task-10/check_dashboards.py from the repository root."""

# /// script
# dependencies = ["wasmtime==49.0.0"]
# ///

from wasmtime import Engine, Store
from wasmtime.component import Component, Linker


# Only the exports exercised by this discovery check are included.
BASE = r"""
(component
  (core module $guest
    (memory (export "memory") 1)
    (data (i32.const 0) "comments0.1.0recentRecent comments")
    ;; Each string and list uses a little-endian (pointer, length) pair.
    ;; 64 holds metadata, 80 holds a descriptor, 96 holds the one-element list.
    (data (i32.const 64) "\00\00\00\00\08\00\00\00\08\00\00\00\05\00\00\00")
    (data (i32.const 80) "\0d\00\00\00\06\00\00\00\13\00\00\00\0f\00\00\00")
    (data (i32.const 96) "\50\00\00\00\01\00\00\00")
    (func (export "info") (result i32) i32.const 64)
    (func (export "list") (result i32) i32.const 96))
  (core instance $i (instantiate $guest))
  (alias core export $i "memory" (core memory $memory))
  (type $metadata (record (field "name" string) (field "version" string)))
  (func $info (result $metadata)
    (canon lift (core func $i "info") (memory $memory)))
  (instance $stored (export "plugin-info" (type $metadata)) (export "info" (func $info)))
  (export "ardent:plugin/stored@0.1.0" (instance $stored))
"""

DASHBOARDS = """
  (type $descriptor (record (field "id" string) (field "title" string)))
  (func $list (result (list $descriptor))
    (canon lift (core func $i "list") (memory $memory)))
  (instance $dashboards
    (export "descriptor" (type $descriptor))
    (export "list" (func $list)))
  (export "ardent:plugin/dashboards@0.1.0" (instance $dashboards))
"""


def call(instance, store, interface, name):
    index = instance.get_export_index(store, name, interface)
    assert index is not None, name
    function = instance.get_func(store, index)
    assert function is not None, name
    value = function(store)
    function.post_return(store)
    return value


def main():
    engine = Engine()
    linker = Linker(engine)
    for extra in ("", DASHBOARDS):
        store = Store(engine)
        instance = linker.instantiate(store, Component(engine, BASE + extra + ")"))
        stored = instance.get_export_index(store, "ardent:plugin/stored@0.1.0")
        assert stored is not None
        info = call(instance, store, stored, "info")
        assert (info.name, info.version) == ("comments", "0.1.0")
        dashboards = instance.get_export_index(store, "ardent:plugin/dashboards@0.1.0")
        assert (dashboards is not None) == bool(extra)
        if dashboards is not None:
            descriptors = call(instance, store, dashboards, "list")
            assert [(item.id, item.title) for item in descriptors] == [("recent", "Recent comments")]
    print("Both stored exports ran. Only the extended component listed Recent comments.")


if __name__ == "__main__":
    main()
