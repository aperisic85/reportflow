"""Stable hierarchical numbering shared by ISO and modern report exports.

Numbering resets for each department: 1, 2, 3; nested rows 3.1, 3.1.1.
Original order is preserved; missing parents and cycles remain visible as roots.
"""
from collections import defaultdict


def numbered_items(items):
    ids = {item.get("id") for item in items}
    children = defaultdict(list)
    for item in items:
        ident = item.get("id")
        parent = item.get("parent_id")
        if parent == ident or parent not in ids:
            parent = None
        children[parent].append(item)

    visited = set()
    output = []

    def visit(item, label, depth):
        ident = item.get("id")
        if ident in visited:
            return
        visited.add(ident)
        output.append((item, label, depth))
        number = 0
        for child in children.get(ident, ()):
            if child.get("id") in visited:
                continue
            number += 1
            visit(child, f"{label}.{number}", depth + 1)

    root_number = 0
    for root in children.get(None, ()):
        if root.get("id") in visited:
            continue
        root_number += 1
        visit(root, str(root_number), 0)
    for orphan in items:
        if orphan.get("id") in visited:
            continue
        root_number += 1
        visit(orphan, str(root_number), 0)
    return output
