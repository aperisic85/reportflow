"""Renders final DOCX/PDF from an immutable sector snapshot."""
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel
from docx import Document
from docx.shared import Cm, Pt
from docx.enum.text import WD_ALIGN_PARAGRAPH
from pathlib import Path
import io, subprocess, tempfile
from collections import defaultdict

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
        children = defaultdict(list)
        ids = {i.get('id') for i in items}
        for item in items:
            parent = item.get('parent_id')
            children[parent if parent in ids else None].append(item)

        visited = set()
        def write(items_here, prefix, depth=0):
            for index, item in enumerate(items_here, 1):
                ident = item.get('id')
                if ident in visited:
                    continue
                visited.add(ident)
                number = f'{prefix}.{index}'
                para = doc.add_paragraph()
                para.paragraph_format.left_indent = Cm(min(depth, 5) * 0.45)
                para.paragraph_format.first_line_indent = Cm(-0.25)
                para.paragraph_format.space_after = Pt(3)
                para.add_run(f'{number}.  ').bold = True
                para.add_run(item.get('title', '')).bold = True
                content = item.get('content', '').strip()
                if content:
                    para.add_run(' – ' + content)
                write(children.get(ident, []), number, depth + 1)
        write(children.get(None, []), str(section_num))
        # Defensive fallback for malformed historical parent relationships.
        write([item for item in items if item.get('id') not in visited], str(section_num))

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
    if format not in ('docx', 'pdf'):
        raise HTTPException(400, 'Unsupported format')
    content = populate(payload)
    if format == 'docx':
        return Response(content=content, media_type='application/vnd.openxmlformats-officedocument.wordprocessingml.document')
    with tempfile.TemporaryDirectory() as tmp:
        source = Path(tmp) / 'report.docx'
        source.write_bytes(content)
        proc = subprocess.run(['libreoffice','-env:UserInstallation=file:///tmp/reportflow-lo', '--headless','--convert-to','pdf','--outdir',tmp,str(source)], capture_output=True, text=True, timeout=90)
        pdf = Path(tmp) / 'report.pdf'
        if proc.returncode != 0 or not pdf.exists():
            raise HTTPException(500, f'PDF conversion failed: {proc.stderr[-300:]}')
        return Response(content=pdf.read_bytes(),media_type='application/pdf')
