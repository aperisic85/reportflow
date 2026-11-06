# Povijest aktivnosti i napredno pretraživanje

## Funkcionalnost
- **Povijest i pretraga** dostupna je u bočnom izborniku.
- Iz **Registra aktivnosti** gumb **Povijest** otvara kronološki pregled iste aktivnosti.
- Povijest prikazuje spremljene mjesečne zapise iz `activity_updates` za plan i izvješće; redoslijed je najnoviji mjesec prvi, a uključuje naziv korisnika koji je zadnji izmijenio mjesečni zapis.
- Aktivnosti bez mjesečnih zapisa prikazuju se samo s opisom iz registra.
- Pretraga obuhvaća naziv i opis iz `activities` te mjesečni tekst `activity_updates.content`. Filteri: odjel, godina, mjesec i vrsta dokumenta.
- Za pretragu nisu potrebni novi DB stupci ni migracije; upiti su parametrizirani.

## Privatnost i ovlasti
- Administratori i direktor mogu pregledavati sve odjele, uz mogućnost filtriranja.
- Voditelji i djelatnici mogu pretraživati i otvarati povijest samo svojeg odjela; filtriranje drugog odjela odbija se HTTP 403.
- Neautentificirani korisnici ne mogu dohvatiti rezultate.
- Upit mora imati 2–120 znakova, a broj rezultata ograničen je na najviše 100.
- Znakovi `%`, `_` i `\` pretražuju se doslovno (ne kao SQL wildcards).

## Ograničenja prve verzije
- Riječ **povijest** ovdje znači slijed mjesečnih zapisa za istu aktivnost, ne pojedinačne revizije svakog pritiska gumba **Spremi**. Izmjena istog mjesečnog zapisa i dalje zamjenjuje prethodni tekst.
- Pretraga trenutno koristi PostgreSQL `ILIKE`. Za veći broj aktivnosti može se naknadno uvesti trigram indeks ili full-text search s hrvatskom konfiguracijom.
- Rezultati su ograničeni na 100; nema paginacije.
- Filtriranje godine/mjeseca odnosi se na **mjesečne zapise**, pa aktivnosti bez takvih zapisa neće biti prikazane kada je filtar aktivan.

## Provjera
```bash
cd backend && cargo test && cargo check
cd ../frontend && npm install && npm run build
```
Nakon pokretanja sustava provjeriti:
1. Korisnik RE odjela pronalazi vlastite aktivnosti, ali ne može čitati povijest HG aktivnosti.
2. Direktor pretražuje sve odjele i filtrira pojedinačni odjel.
3. Prikaz povijesti jedne aktivnosti s više planova i izvješća sortiran je po razdobljima.
4. Aktivnost bez mjesečnih zapisa pojavljuje se u pretrazi po nazivu i ima prazan vremenski slijed.
5. Pojmovi sa znakovima `%` ili `_` tretiraju se kao doslovni tekst.
