"""Document-renderer tests that do not require building the Rust backend."""
import io
import unittest

from docx import Document
from app import RenderRequest, populate
from modern import populate_modern
from numbering import numbered_items


def fixture():
    return RenderRequest(
        title="Izvješće za 11/2026.",
        introduction="Uvodni tekst.",
        conclusion="Zaključak.",
        snapshot={
            "year": 2026, "month": 11, "kind": "report",
            "sections": [{
                "name": "Radioelektronički odjel",
                "source_version": 2,
                "items": [
                    {"id": "a", "parent_id": None, "title": "Aktivnost jedan", "description": "", "content": ""},
                    {"id": "b", "parent_id": None, "title": "Aktivnost dva", "description": "", "content": ""},
                    {"id": "c", "parent_id": None, "title": "Glavna aktivnost tri", "description": "", "content": ""},
                    {"id": "d", "parent_id": "c", "title": "Prva podaktivnost", "description": "", "content": "Izvršeno"},
                    {"id": "e", "parent_id": "c", "title": "Druga podaktivnost", "description": "", "content": ""},
                ]
            }]
        })


class NumberingTests(unittest.TestCase):
    def test_nested_numbers(self):
        rows = numbered_items(fixture().snapshot["sections"][0]["items"])
        self.assertEqual([number for _, number, _ in rows], ["1", "2", "3", "3.1", "3.2"])
        self.assertEqual([depth for _, _, depth in rows], [0, 0, 0, 1, 1])

    def test_odd_historical_links_remain_visible(self):
        rows = [
            {"id": "one", "parent_id": "missing"},
            {"id": "two", "parent_id": "three"},
            {"id": "three", "parent_id": "two"},
        ]
        result = numbered_items(rows)
        self.assertEqual({item["id"] for item, _, _ in result}, {"one", "two", "three"})

    def test_both_export_styles_preserve_subactivity_number(self):
        data = fixture()
        for render in (populate, populate_modern):
            with self.subTest(renderer=render.__name__):
                output = render(data)
                document = Document(io.BytesIO(output))
                joined = "\n".join(p.text for p in document.paragraphs)
                self.assertIn("3.1", joined)
                self.assertIn("Prva podaktivnost", joined)
                self.assertIn("Mjesečna realizacija", joined)


if __name__ == "__main__":
    unittest.main()
