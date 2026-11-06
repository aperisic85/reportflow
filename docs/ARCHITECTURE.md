# Arhitektura i razvojni plan

## Poslovni proces

Djelatnik → mjesečne aktivnosti → voditelj odjela → predano izvješće/plan → direktor sektora → objedinjeni dokument za Upravu.

## Koncepti

- **Activity** – dugovječni opis posla (projekt, trajni ili jednokratni), opcionalni `parent_id` za podstavke.
- **ActivityUpdate** – mjesečni zapis realizacije ili plana; jedinstvena kombinacija aktivnosti, godine, mjeseca i vrste.
- **DepartmentDocument** – trenutni status dokumenta odjela za određeni mjesec.
- **SectorDocument** – nepromjenjivi snapshot aktivnosti odjela pri generiranju, s revizijama i statusom zaključavanja.

## Smjer razvoja

1. Omogućiti uredništvo finalnih poglavlja direktoru bez diranja izvornog teksta.
2. Odobravanje na razini djelatnika i komentari voditelja.
3. Multi-department članstva, zamjene i delegiranje ovlasti.
4. Word import uz mapiranje postojeće hijerarhije.
5. Automatski prijenos plana u sljedeći mjesec i povezivanje planirano/realizirano.
6. Konfigurabilni metapodaci: datum, urudžbeni broj, potpisnik, evidencijska oznaka.
7. Puni audit log, SSO, MFA i pohrana revizija dokumenata.
8. Integracijski testovi, CI, restore drill, sigurnosni pregled.

## API pregled

- `POST /api/auth/login`, `GET /api/auth/me`
- `GET /api/departments`
- `GET /api/users`, `POST /api/users` (admin)
- `GET /api/activities?department_id=...`, `POST /api/activities`, `PATCH /api/activities/{id}`
- `GET /api/updates?department_id=...&year=...&month=...&kind=...`, `POST /api/updates`
- `GET /api/department-documents?...`, `POST /api/department-documents`
- `GET /api/sector-documents?year=...&month=...&kind=...`, `POST /api/sector-documents`
- `POST /api/sector-documents/{id}/lock`
- `GET /api/sector-documents/{id}/export/docx`
- `GET /api/sector-documents/{id}/export/pdf`

## Implementacijske napomene

API autorizacija koristi JWT s ulogom i odjelom; produkcijski treba učitavati prava iz baze pri svakoj promjeni uloge ili koristiti kratkoživuće tokene. Novi projekti i unosi trenutačno se mogu stvarati i unutar odjela čije je mjesečno izvješće već predano, ali predani mjesečni zapis nije moguće uređivati.
