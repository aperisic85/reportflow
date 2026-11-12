# Moderni PDF s ISO memorandumom

Uz tri postojeća izvoza (**Word**, **PDF (ISO)** i **Moderni PDF**) postoji četvrta opcija: **Moderni PDF + memorandum**.

## Kako se sastavlja

1. Aplikacija šalje istu nepromjenjivu reviziju sektorskog izvješća.
2. Dokument-servis učitava **postojeći** `document-service/templates/plovput-clean.docx`.
3. Zadržava originalna Word zaglavlja/podnožja, logotipe, tablice, numeraciju stranica, postavke stranica i reference na predložak, a zamjenjuje samo glavni tekstualni sadržaj modernim stilom.
4. LibreOffice generira PDF, koji se korisniku preuzima kroz isti autorizirani API.

Ovo **nije rekonstrukcija** memoranduma: koristi se izvorni ISO predložak. Ako predložak nedostaje, servis vraća grešku umjesto PDF-a bez memoranduma.

## Sučelje

**Sektor / Uprava → Verzije dokumenta → Moderni PDF + memorandum**

Rute:
- `GET /api/sector-documents/{id}/export/modern-letterhead-pdf`
- `POST /render/modern-letterhead-pdf` (interni dokument-servis)

Uklapa se u postojeće provjere direktorskih prava, bez promjene baze i starih PDF-ova.

## Provjera prije produkcije

- Python test provjerava da ostaju izvorni header i footer i da zamjenski placeholder iz tijela nestaje.
- Provjeriti stvarni višestranični PDF na testnoj instalaciji, naročito visinu ISO zaglavlja/podnožja, logotip, prijelome stranica i mjesto početka teksta.
- Ako je ISO predložak izmijenjen, izlaz automatski nasljeđuje njegove aktualne header/footer elemente.
- Rust build se ne provjerava prema zahtjevu korisnika.
