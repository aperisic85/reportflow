"""Modern, on-screen-reading-inspired PDF layout for ReportFlow.

Rendered as a purpose-designed DOCX with python-docx, then converted to PDF by
the existing LibreOffice renderer. The ISO DOCX/PDF template is not touched.
"""
from datetime import datetime, timezone
from io import BytesIO
from docx.shared import Inches
import qrcode
from pathlib import Path

from docx import Document
from docx.enum.table import WD_CELL_VERTICAL_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Cm, Pt, RGBColor

from numbering import numbered_items

INK = RGBColor(37, 35, 32)
MUTED = RGBColor(112, 104, 94)
AMBER = RGBColor(210, 93, 15)
GOLD = RGBColor(244, 185, 66)
PAPER = RGBColor(255, 255, 255)
BORDER = "E6DFD6"
SOFT = "FFF5E5"
TEMPLATE = Path("/app/templates/plovput-clean.docx")


def shade(cell, fill):
    tc_pr = cell._tc.get_or_add_tcPr()
    shd = OxmlElement("w:shd")
    shd.set(qn("w:fill"), fill)
    tc_pr.append(shd)


def line(paragraph, color=BORDER, size=5):
    p_pr = paragraph._p.get_or_add_pPr()
    borders = OxmlElement("w:pBdr")
    bottom = OxmlElement("w:bottom")
    bottom.set(qn("w:val"), "single")
    bottom.set(qn("w:sz"), str(size))
    bottom.set(qn("w:color"), color)
    borders.append(bottom)
    p_pr.append(borders)


def write_run(p, text, *, bold=False, color=INK, size=None):
    run = p.add_run(str(text))
    run.bold = bold
    run.font.name = "DejaVu Sans"
    run.font.color.rgb = color
    if size:
        run.font.size = Pt(size)
    return run


def add_field(p, name):
    fld = OxmlElement("w:fldSimple")
    fld.set(qn("w:instr"), name)
    p._p.append(fld)


def paragraph(doc, text="", *, size=10, color=INK, after=7, bold=False):
    p = doc.add_paragraph()
    p.paragraph_format.space_after = Pt(after)
    p.paragraph_format.line_spacing = 1.3
    write_run(p, text, bold=bold, color=color, size=size)
    return p


def populate_modern(payload, memorandum=False, approval=None):
    if memorandum:
        # Copy the *actual ISO template* into this document and only replace
        # the main body; section headers/footers, logo and page setup survive.
        # Never silently produce an unbranded PDF when the template is missing.
        if not TEMPLATE.is_file():
            raise FileNotFoundError(f"ISO memorandum template missing: {TEMPLATE}")
        doc = Document(str(TEMPLATE))
        body = doc._element.body
        for element in list(body):
            if not element.tag.endswith("}sectPr"):
                body.remove(element)
    else:
        doc = Document()
    section = doc.sections[0]
    if not memorandum:
        section.top_margin = Cm(2.55)
        section.bottom_margin = Cm(2.1)
        section.left_margin = Cm(2.5)
        section.right_margin = Cm(2.5)
        section.header_distance = Cm(1.1)
        section.footer_distance = Cm(1.1)

    normal = doc.styles["Normal"]
    normal.font.name = "DejaVu Sans"
    normal.font.size = Pt(10)
    normal.font.color.rgb = INK
    normal.paragraph_format.space_after = Pt(7)

    if not memorandum:
        header = section.header.paragraphs[0]
        header.alignment = WD_ALIGN_PARAGRAPH.RIGHT
        write_run(header, "REPORTFLOW", bold=True, color=AMBER, size=8.5)
        write_run(header, "   /   DIGITALNO IZVJEŠĆE", color=MUTED, size=8.5)
        line(header, color="E2D9CD", size=6)

        footer = section.footer.paragraphs[0]
        footer.alignment = WD_ALIGN_PARAGRAPH.RIGHT
        line(footer, color="E2D9CD", size=5)
        write_run(footer, "ReportFlow  •  ", color=MUTED, size=8)
        write_run(footer, "Stranica ", color=MUTED, size=8)
        add_field(footer, "PAGE")

    kind = (payload.snapshot.get("kind") or "report").lower()
    is_plan = kind == "plan"
    year = payload.snapshot.get("year", "")
    month = payload.snapshot.get("month", "")
    period = f"{int(month):02d}/{year}" if str(month).isdigit() else ""
    paragraph(doc, "MJESEČNI PLAN" if is_plan else "MJESEČNO IZVJEŠĆE",
              size=9, color=AMBER, after=10, bold=True)
    paragraph(doc, payload.title, size=22, color=INK, after=11, bold=True)

    info = doc.add_table(rows=1, cols=2)
    info.autofit = False
    info.columns[0].width = Cm(8)
    info.columns[1].width = Cm(7)
    for cell in info.rows[0].cells:
        shade(cell, SOFT)
        cell.vertical_alignment = WD_CELL_VERTICAL_ALIGNMENT.CENTER
        cell.text = ""
    p = info.cell(0, 0).paragraphs[0]
    p.paragraph_format.space_before = Pt(5)
    p.paragraph_format.space_after = Pt(5)
    write_run(p, "RAZDOBLJE   ", bold=True, color=MUTED, size=8)
    write_run(p, period, bold=True, color=INK, size=10)
    p = info.cell(0, 1).paragraphs[0]
    p.paragraph_format.space_before = Pt(5)
    p.paragraph_format.space_after = Pt(5)
    write_run(p, "ODJELA   ", bold=True, color=MUTED, size=8)
    write_run(p, len(payload.snapshot.get("sections", [])), bold=True, color=INK, size=10)
    doc.add_paragraph().paragraph_format.space_after = Pt(3)

    if payload.introduction.strip():
        p = paragraph(doc, "UVOD", size=11, bold=True, color=AMBER, after=8)
        p.paragraph_format.space_before = Pt(16)
        paragraph(doc, payload.introduction.strip(), size=10, after=18)

    for section_index, department in enumerate(payload.snapshot.get("sections", []), 1):
        heading = doc.add_paragraph()
        heading.paragraph_format.space_before = Pt(18)
        heading.paragraph_format.space_after = Pt(11)
        heading.paragraph_format.keep_with_next = True
        write_run(heading, f"{section_index:02d}  ", bold=True, color=AMBER, size=12)
        write_run(heading, department.get("name", "Odjel"), bold=True, color=INK, size=14)
        line(heading, color="D8CAB8", size=8)

        source_version = department.get("source_version")
        if source_version is not None:
            p = paragraph(doc, f"Izvorna verzija odjela: v{source_version}", size=8.5, color=MUTED, after=7)

        for item, number, depth in numbered_items(department.get("items", [])):
            title = doc.add_paragraph()
            title.paragraph_format.left_indent = Cm(min(depth, 5) * 0.56)
            title.paragraph_format.space_before = Pt(10 if depth == 0 else 5)
            title.paragraph_format.space_after = Pt(4)
            title.paragraph_format.keep_with_next = True
            write_run(title, f"{number}  ", bold=True, color=AMBER, size=10)
            write_run(title, (item.get("title") or "").strip(), bold=True, color=INK, size=10.5)

            description = (item.get("description") or "").strip()
            content = (item.get("content") or "").strip()
            if description:
                p = paragraph(doc, description, color=INK, size=9.6, after=4)
                p.paragraph_format.left_indent = Cm(min(depth, 5) * 0.56 + .15)
            if content and content != description:
                p = doc.add_paragraph()
                p.paragraph_format.left_indent = Cm(min(depth, 5) * 0.56 + .15)
                p.paragraph_format.space_after = Pt(9)
                p.paragraph_format.line_spacing = 1.3
                write_run(p, "Mjesečni plan: " if is_plan else "Mjesečna realizacija: ",
                          bold=True, color=AMBER, size=9.6)
                write_run(p, content, size=9.6)

    if payload.conclusion.strip():
        heading = paragraph(doc, "ZAKLJUČAK", size=11, color=AMBER, bold=True, after=8)
        heading.paragraph_format.space_before = Pt(18)
        line(heading, color="D8CAB8", size=6)
        paragraph(doc, payload.conclusion.strip(), size=10, after=13)

    if approval is not None:
        # This is explicitly an INTERNAL approval record, NOT a PAdES signature.
        if not isinstance(approval, dict) or not all(approval.get(k) for k in
                ("document_number", "approved_by", "approved_at", "verification_url")):
            raise ValueError("Incomplete approval metadata")
        panel = doc.add_paragraph()
        panel.paragraph_format.space_before = Pt(18)
        panel.paragraph_format.keep_with_next = True
        write_run(panel, "INTERNO ELEKTRONIČKI ODOBRENO", bold=True, color=AMBER, size=11)
        line(panel, color="D8CAB8", size=6)
        paragraph(doc, f"Dokument: {approval['document_number']}", size=9, after=3)
        paragraph(doc, f"Odobrio/la: {approval['approved_by']}", size=9, after=3)
        paragraph(doc, f"Vrijeme odobrenja (UTC): {approval['approved_at']}", size=9, after=5)
        paragraph(doc, "Ovo je interno odobravanje u sustavu ReportFlow, a ne kvalificirani elektronički potpis.",
                  size=8, color=MUTED, after=7)
        qr = qrcode.QRCode(error_correction=qrcode.constants.ERROR_CORRECT_M,
                           box_size=6, border=2)
        qr.add_data(str(approval["verification_url"]))
        qr.make(fit=True)
        image = BytesIO()
        qr.make_image(fill_color="black", back_color="white").save(image, format="PNG")
        image.seek(0)
        img_paragraph = doc.add_paragraph()
        img_paragraph.paragraph_format.keep_together = True
        img_paragraph.add_run().add_picture(image, width=Cm(2.8))
        paragraph(doc, "Skenirajte QR kod za provjeru izvornog arhiviranog PDF-a.", size=8,
                  color=MUTED, after=10)

    p = doc.add_paragraph()
    p.paragraph_format.space_before = Pt(17)
    write_run(p, "Digitalni prikaz izvješća  •  Izvoz iz ReportFlowa", color=MUTED, size=8)

    out = BytesIO()
    doc.save(out)
    return out.getvalue()
