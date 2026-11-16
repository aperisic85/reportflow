# ReportFlow — interno elektroničko odobravanje i provjera PDF-a

## Što je implementirano

**Potpiši interno i arhiviraj** je interno odobravanje, **nije PAdES ni kvalificirani elektronički potpis**. Jedina uloga koja može odobriti sektorski dokument je **direktor vlastitog sektora**. Mega admin i Podadmin nemaju prava na sadržaj izvješća niti na odobravanje.

Proces:
1. Direktor u **Sektor / Uprava → Verzije dokumenta** odabire nacrt i klikne **Odobri i arhiviraj**.
2. Sustav traži ponovni unos **postojeće korisničke lozinke** i provjerava je prema trenutačnom korisničkom računu.
3. Pod transakcijskim zaključavanjem odabrane revizije stvara nepredvidivi UUID verifikacijski token te trajni identifikator oblika **RF-SP-2026-11-I-v2**. Kratice su oznaka sektora, godina, mjesec, **I** (izvješće) / **P** (plan) te revizija.
4. Dokument-servis generira **jedan konačni moderni PDF s originalnim ISO memorandumom**, blokom internog odobrenja (ime i vrijeme) i QR kodom koji vodi na adresu za provjeru.
5. Izračunava se SHA-256 **gotovih PDF bajtova** i nepromjenjivi PDF, hash, autor, datum/vrijeme, ID dokumenta i token spremaju se u PostgreSQL.
6. Revizija prelazi iz `draft` u `approved`; arhiva i status bilježe se u jednoj DB transakciji. Uređivanje i ponovno odobravanje te revizije nisu dopušteni.

**Važno:** ovo nije kriptografski potpis privatnim certifikatom ni dokaz statusa prema eIDAS-u. QR i SHA-256 osiguravaju usporedbu s PDF-om evidentiranim u bazi, a ne neovisno povjerenje u poslužitelja ili administratorsko vlasništvo nad bazom.

## Broj dokumenta, verzija i arhiviranje

- ID dokumenta jedinstven je po sektoru, razdoblju, vrsti i sektorskoj reviziji.
- Odjelne izvorne verzije (v1/v2…) ostaju zabilježene u postojećem snapshotu sektorske revizije.
- Prilikom arhiviranja zadržava se **binarni PDF u PostgreSQL-u** (`sector_document_approvals.pdf_bytes`), a ne samo mogućnost regeneriranja s novim predloškom.
- SQL trigger odbija UPDATE/DELETE odobrenja. U aplikaciji se odobreni dokument prikazuje s brojem i gumbom **Odobreni PDF (arhiva)**.
- Ranije revizije zaključane starim gumbom ostaju označene `locked`; **ne označavamo ih retroaktivno odobrenima**.

## QR provjera

QR vodi na `{VERIFY_BASE_URL}/verify/{uuid}`. Web-stranica, dostupna bez prijave uz poznavanje nasumičnog UUID tokena, prikazuje samo metapodatke o odobravanju i očekivani SHA-256.

Korisnik na toj stranici može odabrati svoj PDF za usporedbu. **Datoteka se ne šalje na server**: Web Crypto SHA-256 izračunava se u pregledniku; prvo se uspoređuje i veličina. Podudaranje znači da su PDF bajtovi istovjetni arhiviranima. QR token nije potpis, a javno se ne nudi preuzimanje originalnog dokumenta.

## Instalacija

1. Napravi **backup baze** i testiraj novu migraciju `backend/migrations/0006_sector_document_approvals.sql` na testnoj bazi.
2. U `.env` postavi **`VERIFY_BASE_URL=https://stvarna-domena-reportflowa.hr`** (bez `/verify` i bez završne kose crte); to je JAVNA HTTPS adresa koju će otvoriti QR, a ne interna Docker adresa.
3. Ponovno izgradi `api` i `documents` image: dokument-servis koristi Python biblioteku **qrcode[pil]**.
4. Provjeri da reverse proxy poslužuje SPA putanje `/verify/<token>` preko `index.html` te prosljeđuje `/api/verify/<token>` prema Rust API-ju. Postojeći Nginx konfiguracijski predložak podržava ove putanje.
5. Ulogiraj se kao direktor i provjeri: pogrešna lozinka ne odobrava; ispravna odobrava samo vlastiti nacrt. Korisnici drugih sektora i administratori ne smiju odobravati ni preuzeti tuđe arhivirane PDF-ove.
6. Preuzmi **Odobreni PDF (arhiva)**, skeniraj njegov QR i učitaj upravo tu datoteku u provjeru: mora se podudarati SHA-256. Zatim promijeni PDF i provjeri da se ne podudara.
7. Provjeri duži višestranični PDF sa stvarnim ISO predloškom kako zaglavlje, podnožje i QR ne bi bili presječeni.
8. Potvrdi da već zaključane revizije ostaju u starom statusu te da nijedan stari ISO/moderni izvoz nije promijenjen.

## Ograničenja prve faze

- Nema kvalificiranog potpisa, PAdES, potpisnog certifikata ni vremenskog žiga certificiranog pružatelja usluga povjerenja.
- Ponovna potvrda lozinkom nije MFA; za višu sigurnost treba dodati drugi faktor.
- Za strogo odvojenu revizijsku vjerodostojnost predviđeno je kasnije potpisivanje PAdES-om / vanjski timestamp i vanjska nepromjenjiva arhiva.
- SQLx migracija i end-to-end testiranje direktorskog API-ja zahtijevaju testnu bazu. **Rust build nije pokrenut**, prema korisnikovu zahtjevu.
