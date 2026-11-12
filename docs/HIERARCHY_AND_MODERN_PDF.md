# Hijerarhija aktivnosti i moderni PDF u ReportFlowu

## Uvlačenje podaktivnosti

Aktivnosti su složene prema stvarnim odnosima `parent_id`, a ne samo označene strelicom. Hijerarhijski pomak vidljiv je u Registru, Mjesečnom unosu, pregledu dokumenta odjela, uređivanju sektorskog nacrta i HTML pregledu svih izvješća.

Primjer unutar odjela:

```text
1  Pregled opreme
2  Planiranje nabave
3  Modernizacija komunikacija
   3.1  Zamjena antene
   3.2  Ugradnja VHF uređaja
        3.2.1  Konfiguracija kanala
4  Preventivni pregledi
```

Brojevi aktivnosti počinju od 1 u svakom odjelu; naslovi odjela ostaju zasebna poglavlja (odjel 01, 02 itd.). Podaktivnosti imaju vlastite brojeve izvedene iz nadređene aktivnosti. Više razina podržano je rekurzivno, s ograničenjem samo vizualne uvlake nakon pete razine.

Za neispravne stare poveznice roditelj–dijete primjenjuje se siguran prikaz: izgubljeni ili ciklički povezani zapisi neće nestati iz izvješća.

## Numeracija u svim izlazima

Jedinstvena logika prikaza koristi se u:
- registru i mjesečnom unosu,
- HTML izvješćima odjela i sektora,
- postojećem Word predlošku i standardnom ISO PDF-u,
- novom modernom PDF-u.

Za dokumente već predane u prošlosti prikazuju se nepromjenjivi snapshotovi aktivnosti; postojeći dokument se ne prepisuje.

## Dva PDF formata

U **Sektor / Uprava → Verzije dokumenta** dodana su tri izbora:
- **Word** – postojeći službeni ISO predložak;
- **PDF (ISO)** – postojeći službeni PDF, ali s usklađenom numeracijom;
- **Moderni PDF** – zaseban stil primjeren digitalnom čitanju, s toplom svijetlom podlogom, grafitno-narančastim naglascima, tipografskom hijerarhijom, zasebnim poglavljima odjela, jasno odvojenom mjesečnom realizacijom i brojevima stranica.

Moderni PDF nastaje u postojećem `document-service` servisu: `/render/modern-pdf`. Izrađuje se zaseban formatirani DOCX u memoriji te se, kao i standardni PDF, konvertira LibreOfficeom. Ne sprema se na poslužitelj i ne mijenja arhivirane revizije.

Backend izvoz: `GET /api/sector-documents/{id}/export/modern-pdf` uz iste direktorske ovlasti kao ostali izvozi.

## Provjera

Dodani su Python smoke testovi za zajedničko brojanje i oba tipa dokumenta, uključujući primjer **3 → 3.1** te obradu neispravne povijesne hijerarhije. CI provjerava Python module i dokumente, uz postojeći React build.

Funkcionalni test modernog PDF-a na stvarnom dokument-servisu i vizualni pregled više-straničnog PDF-a preporučeni su prije primjene na produkciju.

**Napomena:** Rust build se ne provjerava prema dogovoru. U ovoj izmjeni nema migracija baze.
