# ReportFlow – mjesečni planovi i izvješća

Interna aplikacija za **Sektor sigurnosti plovidbe**: četiri odjela izrađuju mjesečne planove i izvješća, a direktor sektora objedinjuje dostavljene dokumente u službeni Word ili PDF.

## Što je uključeno

- Rust (Axum + Tokio + SQLx) API, PostgreSQL migracije, JWT prijava i Argon2 lozinke.
- React + TypeScript + Vite frontend, responzivan na mobitelima.
- Četiri odjela: radio-elektrotehnički, hidro-građevinski, brodostrojarski i nautički.
- Registar trajnih, projektnih i jednokratnih aktivnosti s podstavkama.
- Mjesečni plan/izvješće: tekst, status i uključivanje stavke u dokument.
- Voditeljeva predaja dokumenta odjela i povratak u nacrt.
- Direktorovo objedinjavanje predanih odjelnih dokumenata, odvojene revizije i zaključavanje.
- Generator DOCX/PDF sa službenim zaglavljem i podnožjem sačuvanim iz priloženog Word predloška.

> **Status:** početni funkcionalni MVP / projektni kostur. Prije produkcijskog korištenja potrebni su integracijski testovi, sigurnosna provjera, prilagodba dokumenta i stvarna korisnička testiranja. Repo nije provjeren kompilacijom Rust backenda u ovom okruženju.

## Brzi početak – Docker Compose

1. Instalirajte Docker Engine i Compose plugin.
2. Kopirajte `.env.example` u `.env`.
3. Promijenite sve lozinke i `JWT_SECRET`. Ne objavljujte `.env` na GitHubu.
4. Pokrenite:

```bash
cp .env.example .env
# uređivanje .env: nano .env
docker compose up -d --build
docker compose ps
docker compose logs -f api
```

5. Otvorite `http://localhost:8088` (ili port iz `APP_PORT`).
6. Prijavite se s korisničkim imenom i lozinkom iz `.env` (`BOOTSTRAP_ADMIN` / `BOOTSTRAP_PASSWORD`).
7. Administrator dodaje voditelje i djelatnike kroz karticu **Korisnici**.

**Napomena:** aplikaciju ne izlagati javnom internetu bez HTTPS reverse proxyja, zaštite pristupa i sigurnosne provjere. Bootstrap administrator se kreira samo ako korisničko ime već ne postoji; izmjena lozinke u `.env` ne mijenja postojeću lozinku u bazi.

## Predloženi redoslijed korištenja

1. Administrator kreira korisnike i dodijeli im uloge i odjele.
2. Voditelj ili djelatnik dodaje projektne i trajne aktivnosti u registru.
3. Djelatnik upisuje mjesečni napredak u kartici **Mjesečni unos**.
4. Voditelj provjerava unos i predaje **Izvješće odjela**.
5. Direktor odabire predane odjele, izrađuje novu verziju **Sektor / Uprava** i preuzima Word/PDF.
6. Kada je dokument usuglašen, direktor ga zaključava.

## Uloge

| Uloga | Prava |
|---|---|
| `employee` | Dodavanje aktivnosti u vlastiti odjel; uređivanje vlastitih mjesečnih zapisa i vlastitih aktivnosti |
| `manager` | Pregled i uređivanje svih aktivnosti odjela, predaja i vraćanje izvješća |
| `director` | Pregled svih odjela, izrada i zaključavanje izvješća sektora |
| `admin` | Sve navedeno i upravljanje korisnicima |

Korisnik u MVP-u pripada najviše jednom odjelu. U budućoj verziji moguće je uvesti članstvo u više odjela.

## Arhitektura

```text
Browser → nginx / React UI (:8088)
               └─ /api/* → Rust Axum (:8095, interni Docker port)
                                ├─ PostgreSQL (:5432, interni Docker port)
                                └─ Python Document Service (:8081, interni port)
                                                   └─ DOCX + LibreOffice PDF
```

## Pohrana i sigurnost

- Baza ostaje u Docker volumenu `pgdata`. Napravite redoviti backup `pg_dump` i test povrata.
- JWT živi 12 sati. Frontend čuva token u **sessionStorage**; za produkciju preporučuje se HttpOnly cookie + CSRF zaštita, SSO/AD i MFA.
- Audit tablica bilježi dio promjena, ali ne predstavlja potpunu forenzičku revizijsku evidenciju.
- Promjene izvješća odjela nakon predaje nisu dozvoljene dok ga voditelj ne vrati u nacrt.
- Sektorski dokument je snapshot: naknadna promjena aktivnosti ne mijenja postojeću reviziju.
- U MVP-u nema ugrađenog sučelja za detaljnu redakciju sektorskog snapshota; redakcija se može napraviti u preuzetom Wordu. Konačno strukturirano uređivanje treba dodati u sljedećoj iteraciji.

## Dokumenti i obrazac

`templates/plovput-clean.docx` je očišćeni obrazac izrađen iz dostavljenog Word dokumenta (zaglavlje/podnožje zadržani). Druga kopija smještena je u `document-service/templates/` i koristi se u Docker imageu. Ako mijenjate predložak, zamijenite obje kopije.

U dokumentu se automatski izrađuje numeracija poglavlja i podstavki na temelju odabranih odjela i hijerarhije aktivnosti. Datum i oznaka službenog broja u ovom MVP-u još nisu konfigurabilna polja dokumenta; potrebna je dorada prema vašem internom obrascu.

## Struktura projekta

```text
backend/             Rust API i PostgreSQL migracije
document-service/    DOCX/PDF renderer
frontend/            React / TypeScript / Vite
templates/           očišćeni službeni Word predložak
docs/                model i razvojni plan
scripts/             pomoćne skripte
```

## Razvoj bez Dockera

API zahtijeva PostgreSQL i varijable `DATABASE_URL`, `JWT_SECRET`, `BOOTSTRAP_ADMIN`, `BOOTSTRAP_PASSWORD`, `DOC_SERVICE_URL`.

```bash
cd backend
cargo run
# novi terminal
cd frontend
npm install
npm run dev
```

Vite dev server prosljeđuje `/api` prema `localhost:8095`. Document-service može raditi zasebno s `uvicorn app:app --port 8081` i predloškom dostupnim na `/app/templates/` (ili kroz Docker).

## Poznata ograničenja MVP-a

- Nema AD/LDAP/SSO prijave, reseta lozinke ili promjene korisničkog računa.
- Nema posebnog odobravanja svakog pojedinačnog unosa niti komentara i priloga.
- Import starih Word izvješća i automatski prijenos stavki iz prethodnog mjeseca nisu implementirani: registar ostaje trajan, mjesečni zapis unosi se iznova.
- Nema drag-and-drop preslagivanja niti uređivača finalnog dokumenta u browseru.
- Odjeli se biraju ručno, a dokument se može generirati samo od predanih odjela.
- PDF konverzija zahtijeva LibreOffice u `documents` kontejneru.
- API za izmjenu aktivnosti postoji, ali frontend trenutačno prikazuje registar i omogućuje dodavanje; uređivanje kroz UI planirano je.

## GitHub

```bash
git init
git add .
git commit -m "Initial ReportFlow MVP"
git branch -M main
git remote add origin https://github.com/USERNAME/REPOSITORY.git
git push -u origin main
```

Datoteka `.env` već je u `.gitignore`; ipak provjerite `git status` prije objave. **Službeni DOCX predložak sadrži elemente Plovputova službenog obrasca: koristite privatni GitHub repo ili izostavite predložak iz javne objave.**
# reportflow
