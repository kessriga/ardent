"""Run with uv run spikes/task-10/check_dashboards.py from the repository root."""

# /// script
# dependencies = ["componentize-py==0.25.1", "wasmtime==49.0.0"]
# ///

from pathlib import Path
import subprocess
from shutil import copyfile
from tempfile import TemporaryDirectory
from types import SimpleNamespace

from wasmtime import Engine, Store
from wasmtime.component import Component, Linker, Variant


FIXTURES = Path(__file__).resolve().parent
PACKAGE = "ardent:plugin"


def call(instance, store, interface, name, *args):
    index = instance.get_export_index(store, name, interface)
    assert index is not None, name
    function = instance.get_func(store, index)
    assert function is not None, name
    value = function(store, *args)
    function.post_return(store)
    return value


def check_view(instance, store, expect_dashboards):
    stored = instance.get_export_index(store, f"{PACKAGE}/stored@0.1.0")
    assert stored is not None
    info = call(instance, store, stored, "info")
    assert (info.name, info.version) == ("comments", "0.1.0")
    assert call(instance, store, stored, "check", []) == "a comment needs text"
    fields = [SimpleNamespace(name="text", value="Review this change")]
    checked = call(instance, store, stored, "check", fields)
    assert [(field.name, field.value) for field in checked] == [("text", "Review this change")]
    assert call(instance, store, stored, "summary", checked) == "Review this change"

    dashboards = instance.get_export_index(store, f"{PACKAGE}/dashboards@0.1.0")
    assert (dashboards is not None) == expect_dashboards
    if dashboards is None:
        return
    descriptors = call(instance, store, dashboards, "list")
    assert [(item.id, item.title) for item in descriptors] == [("recent", "Recent comments")]

    now = "2026-11-20T08:20:00Z"
    anchor = SimpleNamespace(
        path="README.md", fingerprint="174add0298423e83", status="confirmed", was=None, quote=None, lines=None,
    )
    entry = SimpleNamespace(id="4k2m9qx7tb", created=now, anchor=anchor, fields=checked)
    settings = [SimpleNamespace(name="limit", value=20.0)]
    layout = call(instance, store, dashboards, "render", "recent", now, settings, [Variant("stored", entry)])
    assert layout.tag == "outline"
    (row,) = layout.payload
    assert (row.item.label, row.item.target, row.parent) == ("Review this change", Variant("stored-id", entry.id), None)


def main():
    engine = Engine()
    linker = Linker(engine)
    with linker.root() as root, root.add_instance(f"{PACKAGE}/host@0.1.0") as host:
        host.add_func("outline-of", lambda store, file: None)
        host.add_func("backlinks", lambda store, target: [])
    with TemporaryDirectory() as directory:
        build = Path(directory)
        # Compile against decision-3's WIT so the base contract has a single source.
        decision = next((FIXTURES.parents[1] / "backlog/decisions").glob("decision-3 - *.md"))
        base = decision.read_text().split("```wit\n", 1)[1].split("```", 1)[0]
        (build / "view.wit").write_text(base)
        copyfile(FIXTURES / "dashboards.wit", build / "dashboards.wit")
        for world, module in [("stored-view", "comments"), ("dashboard-view", "recent_comments")]:
            output = build / f"{world}.wasm"
            copyfile(FIXTURES / f"{module}.py", build / f"{module}.py")
            subprocess.run(
                ["componentize-py", "-d", ".", "-w", world, "componentize", "--stub-wasi", module, "-o", str(output)],
                cwd=build, check=True,
            )
            store = Store(engine)
            instance = linker.instantiate(store, Component.from_file(engine, output))
            check_view(instance, store, world == "dashboard-view")
    print("Both components passed info, check and summary. Only the extended component listed and rendered Recent comments.")


if __name__ == "__main__":
    main()
