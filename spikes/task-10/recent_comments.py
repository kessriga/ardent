"""Extended fixture with the optional dashboards interface."""

from componentize_py_types import Err
from wit_world.exports import dashboards

from comments import Stored


class Dashboards:
    def list(self):
        return [dashboards.Descriptor("recent", "Recent comments")]

    def render(self, id, now, settings, entries):
        if id != "recent":
            raise Err("unknown dashboard")
        items = []
        for entry in entries:
            comment = entry.value
            item = dashboards.Item(Stored().summary(comment.fields), dashboards.Target_StoredId(comment.id))
            items.append(dashboards.OutlineItem(item, None))
        return dashboards.Layout_Outline(items)
