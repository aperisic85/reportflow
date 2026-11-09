"""Run: python -m unittest discover -s tests -v"""
import importlib.util
import io
import unittest
from pathlib import Path
from docx import Document

APP=Path(__file__).resolve().parents[1]/'document-service'/'app.py'
spec=importlib.util.spec_from_file_location('reportflow_document_service',APP)
service=importlib.util.module_from_spec(spec)
spec.loader.exec_module(service)

class ReportDocumentTest(unittest.TestCase):
    def test_titles_descriptions_monthly_updates_and_nested_items(self):
        data={'title':'IZVJEŠĆE – LISTOPAD 2026.','introduction':'Uvod','conclusion':'Zaključak teksta',
              'snapshot':{'sections':[{'name':'Radio-elektrotehnički poslovi', 'items':[
                {'id':'a','parent_id':None,'title':'Projekt CYSCROMS','description':'Opis projekta', 'content':'Mjesečni napredak'},
                {'id':'b','parent_id':'a','title':'Mrežni usmjerivači','description':'Nabava usmjerivača','content':''},
                {'id':'c','parent_id':None,'title':'VPN mreža','description':'Praćenje mreže','content':'Praćenje mreže'}]}]}}
        content=service.populate(service.RenderRequest(**data))
        doc=Document(io.BytesIO(content))
        txt='\n'.join(p.text for p in doc.paragraphs)
        for expected in ['Projekt CYSCROMS','Opis projekta','Mjesečni napredak','Mrežni usmjerivači','Nabava usmjerivača','1.1.1.', 'VPN mreža','Zaključak teksta']:
            self.assertIn(expected,txt)
        self.assertEqual(txt.count('Praćenje mreže'),1,'Description and identical monthly content must not duplicate')

if __name__=='__main__': unittest.main()