# ReportFlow – Navigacija, vrijeme nastanka i izvorne verzije

## Izbornik

Redoslijed radnog izbornika za direktora, voditelja i radnika:

1. Početna
2. **Unos aktivnosti** (bivši Registar aktivnosti)
3. **Izrada mjesečnog plana/izvješća** (bivši Mjesečni unos)
4. **Izvješće/Plan odjela**
5. **Povijest/Pretraživanje**
6. **Sektor / Uprava** – samo direktor sektora

**Inbox / Obavijesti** je vizualno izdvojen ispod glavnog izbornika, sa zadržanim brojem nepročitanih poruka.

Na stranici **Unos aktivnosti** uklonjen je gornji blok za mjesec, godinu i vrstu dokumenta. Direktor sektora i dalje ima samo odabir **odjela**, jer upravlja pregledom različitih odjela svojeg sektora; voditelj i radnik koriste vlastiti odjel.

## Evidencija vremena i korisnika

- **Aktivnost iz registra**: zadržava izvorni `created_at` i `updated_at`; uz kreiranje se prikazuje ime autora kada postoji.
- **Mjesečni zapis plana/izvješća**: dobiva zaseban `created_at` i `created_by`, koji se ne mijenjaju tijekom sljedećih izmjena. `updated_at` i `updated_by` i dalje pokazuju posljednje spremanje.
- **Dokument odjela**: dobiva `created_at` i `created_by`; `submitted_at` i `submitted_by` bilježe posljednju predaju, a tablica verzija čuva svaku pojedinu predaju s točnim vremenom i autorom.
- **Sektorski dokument**: već ima `created_at` i `created_by`. U popisu revizija prikazani su datum, sat, minuta, sekunda i ime autora.
- Povijest aktivnosti prikazuje izvorno kreiranje, kasnije izmjene i njihove autore. Datumi se prikazuju u hrvatskom lokalnom formatu, a PostgreSQL bilježi vremena kao `TIMESTAMPTZ`.

### Povijesni podaci

Ranija shema nije čuvala zasebno izvorno vrijeme i autora nastanka svih mjesečnih zapisa i dokumenata odjela. Nova migracija za te zapise koristi najraniji još dostupni timestamp te `creation_time_estimated=true`. Sučelje takve datume izričito označava kao **povijesnu procjenu**, bez lažne tvrdnje o preciznosti. Novi zapisi imaju stvarno vrijeme kreiranja.

## Verzije odjela u sektorskom izvješću

Sektorska revizija već je pohranjivala `source_version` (v1, v2…) i **nastavlja čuvati izvorne snapshotove** sadržaja odjela. Nova implementacija pri stvaranju dodatno pohranjuje:

- `source_submitted_at`: vrijeme predaje baš te odjelne verzije;
- `source_submitted_by`: korisnik koji ju je predao.

Prije objedinjavanja odjeli pokazuju broj svoje posljednje predane verzije. Nakon objedinjavanja popis sektorskih revizija i HTML pregled prikazuju **koja je verzija svakog odjela stvarno korištena**. Naknadna predaja v3 ne mijenja već izrađeni sektorski snapshot s v2. Pri uređivanju nacrta sektorske revizije backend također čuva izvorne metapodatke sekcija.

## Uvođenje i testiranje

1. Napraviti backup PostgreSQL baze i provjeriti migracije na kopiji.
2. Primijeniti **novu** `backend/migrations/0005_document_creation_audit.sql` (prethodne se migracije ne uređuju).
3. Napraviti aktivnost kao radnik, mjesečni zapis kao radnik, urediti kao voditelj; provjeriti različita vremena i autore kreiranja i zadnje izmjene.
4. Predati plan/izvješće odjela v1 i v2; provjeriti autora i vrijeme svake predaje te dokumenta odjela.
5. Izraditi sektorsku reviziju s odjelima koji imaju različite brojeve verzija; provjeriti oznake v1/v2, imena korisnika i vremenske oznake u HTML prikazu.
6. Naknadno predati novu verziju odjela; stara sektorska revizija mora zadržati originalnu oznaku izvorne verzije.
7. U Unosu aktivnosti provjeriti da nema kontrola mjesec/godina/plan-izvješće, a da direktor može odabrati odjel.
8. Provjeriti Inbox i raspored izbornika na mobilnom prikazu.

**Napomena:** Rust build se prema zahtjevu korisnika ne provjerava. Uspješan frontend build nije potvrda da migracija i backend rade u produkciji.
