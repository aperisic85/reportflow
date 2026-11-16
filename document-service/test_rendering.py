"""Document-renderer tests that do not require building the Rust backend."""
import io
import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch

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

    def test_memo_export_retains_iso_header_and_footer(self):
        # Build a representative ISO template without sharing any company assets.
        with tempfile.TemporaryDirectory() as directory:
            template = Path(directory) / "iso-memorandum.docx"
            original = Document()
            original.add_paragraph("PLACEHOLDER BODY SHOULD BE REPLACED")
            original.sections[0].header.paragraphs[0].text = "ISO MEMORANDUM - ZAGLAVLJE"
            original.sections[0].footer.paragraphs[0].text = "ISO MEMORANDUM - PODNOZJE"
            original.save(template)
            with patch("modern.TEMPLATE", template):
                output = populate_modern(fixture(), memorandum=True)
        result = Document(io.BytesIO(output))
        self.assertIn("ISO MEMORANDUM - ZAGLAVLJE", result.sections[0].header.paragraphs[0].text)
        self.assertIn("ISO MEMORANDUM - PODNOZJE", result.sections[0].footer.paragraphs[0].text)
        body = "\\n".join(p.text for p in result.paragraphs)
        self.assertNotIn("PLACEHOLDER BODY SHOULD BE REPLACED", body)
        self.assertIn("3.1", body)
        self.assertIn("Mjesečna realizacija:", body)

    def test_missing_iso_template_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch("modern.TEMPLATE", Path(directory) / "nonexistent.docx"):
                with self.assertRaises(FileNotFoundError):
                    populate_modern(fixture(), memorandum=True)

    def test_approved_modern_document_has_qr_and_notice(self):
        data = fixture()
        approval = {
            "document_number": "RF-SP-2026-11-I-v2",
            "approved_by": "Direktor sektora",
            "approved_at": "2026-11-12T13:00:00+00:00",
            "verification_url": "https://reportflow.example.hr/verify/00000000-0000-4000-8000-000000000001",
        }
        result = Document(io.BytesIO(populate_modern(data, approval=approval)))
        text = "\\n".join(p.text for p in result.paragraphs)
        self.assertIn("RF-SP-2026-11-I-v2", text)
        self.assertIn("INTERNO ELEKTRONIČKI ODOBRENO", text)
        self.assertIn("ne kvalificirani elektronički potpis", text)
        self.assertTrue(result.inline_shapes, "Verification QR image is missing")

    def test_approval_requires_complete_metadata(self):
        with self.assertRaises(ValueError):
            populate_modern(fixture(), approval={"document_number": "RF-TEST"})

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
