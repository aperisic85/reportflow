# ReportFlow – multisektorska organizacija i administracija korisnika

## Model prava

| Uloga | Organizacija / korisnici | Planovi i izvješća |
|---|---|---|
| **Mega admin** (postojeća DB uloga `admin`) | Kreira sektore, odjele, korisnike, Podadmine i druge Mega admine; uređuje i deaktivira korisnike | **Nema pristup sadržaju, ne predaje niti zaključava** |
| **Podadmin** (`org_admin`) | Kreira sektore, odjele, korisnike i druge Podadmine; uređuje/deaktivira korisnike; ne može mijenjati Mega admine | **Nema pristup sadržaju, ne predaje niti zaključava** |
| **Direktor sektora** (`director`) | Nema administratorske ovlasti | Čita vlastite odjele, izrađuje/uređuje/zaključava objedinjeno sektorsko izvješće |
| **Voditelj odjela** (`manager`) | Nema administratorske ovlasti | Uređuje aktivnosti i planove svoga odjela, predaje odjelne dokumente |
| **Radnik** (`employee`) | Nema administratorske ovlasti | Unosi svoje mjesečne zapise i komunicira unutar odjela |

## Organizacijska struktura

```text
ReportFlow
  Mega admin
  Podadmin(i)
  Sektor 1
    Direktor sektora
    Odjel A
      Voditelj odjela
      Radnici
    Odjel B
      Voditelj odjela
      Radnici
  Sektor 2
    Direktor sektora
    Odjel ...
```

Sektor je jedinica razdvajanja izvješća i korisničkog sadržaja. Korisniku u direktorskoj ulozi dodjeljuje se jedan sektor; radniku i voditelju jedan odjel koji mora pripadati odabranom sektoru. Administrator nema sektor ni odjel jer upravlja cijelom organizacijom, ali ne izrađuje izvješća.

## Migracija

Nova migracija `backend/migrations/0004_multisector.sql`:
- stvara tablicu `sectors` i inicijalni sektor **Sigurnost plovidbe** (oznaka `SP`);
- povezuje postojeće odjele i sve stare sektorske revizije s novim sektorom uz očuvane UUID-eve i dokumente;
- omogućava oznake odjela jedinstvene unutar sektora, a ne globalno;
- korisnicima dodaje `sector_id`, uvodi ulogu `org_admin` i provjerava usklađenost uloge, sektora i odjela;
- postojeću DB ulogu `admin` **ne preimenuje** (na sučelju je prikazana kao Mega admin);
- starim administratorima uklanja odjelnu vezu, a postojećim direktorima zadržava sektor temeljem ranije dodijeljenog odjela ili inicijalnog sektora.

**Prije produkcijskog uvođenja obavezni su backup baze i testna migracija na kopiji stvarnih podataka.** Migraciju ne treba primjenjivati ručno izrezanim SQL naredbama: postojeći backend pokreće SQLx migracije.

## API i zaštita podataka

- Administracija: `GET/POST /api/sectors`, `GET/POST /api/departments`, `GET/POST /api/users`, `PATCH /api/users/{id}`.
- Uređivanje korisnika: ime, uloga, sektor/odjel, aktivnost računa, opcionalna zamjena lozinke; korisničko ime ostaje stalno.
- `/api/auth/me` i prijava vraćaju `sector_id` i ulogu.
- Svaki autorizirani zahtjev čita aktualno stanje korisnika iz baze: deaktivacija ili izmjena uloge vrijedi odmah, bez čekanja isteka JWT-a.
- Za sadržajne API-je provjerava se pripadnost odjelu/sektoru; direktor pristupa samo vlastitom sektoru.
- Sektorska dokumentacija, revizije i izvozi filtriraju se prema `sector_id`. Ni Mega admin ni Podadmin nisu ovlašteni za sadržajne API-je.
- Chat i Inbox provjeravaju sektor/odjel, a administratori nemaju pravo čitanja radnih rasprava.

## Provjere prije produkcije

1. Testna migracija na kopiji postojeće baze, potvrda da svi stari odjeli, aktivnosti i dokumenti pripadaju sektoru SP.
2. Mega admin: prijava, novi Podadmin, sektor, odjel, korisnik; uređivanje/deaktivacija i opcionalna promjena lozinke.
3. Podadmin: isto upravljanje sektorima, odjelima i korisnicima, ali bez mogućnosti mijenjanja ili kreiranja Mega admina.
4. Direktor sektora A: vidi samo vlastite odjele, izvješća i revizije; ne vidi sektor B ni kroz API.
5. Voditelj/radnik: samo svoj odjel za sadržaj; radnik ne može predavati izvješća.
6. Mega admin i Podadmin: ne mogu pristupati planovima, aktivnostima, Inboxu, HTML izvješćima niti njihovim API-jima.
7. Provjeriti izdavanje JWT-a, izmjenu uloge/deaktivaciju već prijavljenog korisnika i zabranu uporabe zastarjelih ovlasti.
8. Provjeriti postojeći PDF/Word i HTML izvoz za direktore, uključujući dvije revizije za isti mjesec u različitim sektorima.

**Build:** po zahtjevu vlasnika projekta Rust build nije provjeravan. Frontend build ne jamči funkcionalnost SQL migracije i svih API putanja, zato ovu nadogradnju prvo provesti na testnom sustavu.
