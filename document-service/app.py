"""Renders final DOCX/PDF from an immutable sector snapshot."""
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel
from docx import Document
from docx.shared import Cm, Pt
from docx.enum.text import WD_ALIGN_PARAGRAPH
from pathlib import Path
import io, subprocess, tempfile
from numbering import numbered_items
from modern import populate_modern

app = FastAPI(title='ReportFlow document renderer')
TEMPLATE = Path('/app/templates/plovput-clean.docx')

class RenderRequest(BaseModel):
    title: str
    introduction: str = ''
    conclusion: str = ''
    snapshot: dict


def populate(payload: RenderRequest) -> bytes:
    doc = Document(str(TEMPLATE)) if TEMPLATE.exists() else Document()
    # Remove any placeholder body paragraphs while preserving original section headers/footers.
    body = doc._element.body
    for el in list(body):
        if not el.tag.endswith('}sectPr'):
            body.remove(el)
    section = doc.sections[0]
    section.top_margin = Cm(3.0)
    section.bottom_margin = Cm(2.5)
    section.left_margin = Cm(2.5)
    section.right_margin = Cm(2.5)

    title = doc.add_paragraph()
    title.style = 'Normal'
    title.add_run('Predmet:   ' + payload.title).bold = True
    title.paragraph_format.space_after = Pt(18)
    if payload.introduction.strip():
        doc.add_paragraph(payload.introduction)

    for section_num, section_data in enumerate(payload.snapshot.get('sections', []), 1):
        h = doc.add_paragraph()
        h.paragraph_format.space_before = Pt(12)
        h.paragraph_format.keep_with_next = True
        h.add_run(f'{section_num}.  {section_data.get("name", "ODJEL").upper()}').bold = True
        items = section_data.get('items', [])
        for item, number, depth in numbered_items(items):
            para = doc.add_paragraph()
            para.paragraph_format.left_indent = Cm(min(depth, 5) * 0.45)
            para.paragraph_format.first_line_indent = Cm(-0.25)
            para.paragraph_format.space_after = Pt(3)
            para.add_run(f'{number}.  ').bold = True
            para.add_run(item.get('title', '')).bold = True
            # Retain permanent description and monthly update separately.
            description = (item.get('description') or '').strip()
            content = (item.get('content') or '').strip()
            if description:
                para.add_run(' – ' + description)
            if content and content != description:
                para.add_run().add_break()
                label = 'Mjesečni plan: ' if payload.snapshot.get('kind') == 'plan' else 'Mjesečna realizacija: '
                para.add_run(label).bold = True
                para.add_run(content)

    if payload.conclusion.strip():
        p = doc.add_paragraph()
        p.paragraph_format.space_before = Pt(15)
        p.add_run('Zaključak').bold = True
        doc.add_paragraph(payload.conclusion)
    out = io.BytesIO()
    doc.save(out)
    return out.getvalue()

@app.get('/health')
def health():
    return {'status': 'ok'}

@app.post('/render/{format}')
def render(format: str, payload: RenderRequest):
    if format not in ('docx', 'pdf', 'modern-pdf', 'modern-letterhead-pdf'):
        raise HTTPException(400, 'Unsupported format')
    content = (populate_modern(payload, memorandum=(format == 'modern-letterhead-pdf'))
               if format in ('modern-pdf', 'modern-letterhead-pdf') else populate(payload))
    if format == 'docx':
        return Response(content=content, media_type='application/vnd.openxmlformats-officedocument.wordprocessingml.document')
    with tempfile.TemporaryDirectory() as tmp:
        source = Path(tmp) / 'report.docx'
        source.write_bytes(content)
        proc = subprocess.run(['libreoffice',f'-env:UserInstallation=file://{tmp}/lo-profile', '--headless','--convert-to','pdf','--outdir',tmp,str(source)], capture_output=True, text=True, timeout=90)
        pdf = Path(tmp) / 'report.pdf'
        if proc.returncode != 0 or not pdf.exists():
            raise HTTPException(500, f'PDF conversion failed: {proc.stderr[-300:]}')
        return Response(content=pdf.read_bytes(),media_type='application/pdf')
