"""Stored view fixture shared by both components."""

from componentize_py_types import Err
from wit_world.imports.types import PluginInfo, Value_Text


class Stored:
    def info(self):
        return PluginInfo("comments", "0.1.0")

    def check(self, fields):
        if not self.summary(fields):
            raise Err("a comment needs text")
        return fields

    def summary(self, fields):
        for field in fields:
            if field.name == "text" and isinstance(field.value, Value_Text):
                return field.value.value
        return ""
