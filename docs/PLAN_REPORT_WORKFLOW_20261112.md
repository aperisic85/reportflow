# ReportFlow – mjesečni plan, izvješće i nazivi aktivnosti

## a) Izvješće za budući mjesec

- Moguće je otvoriti i popunjavati **nacrt** izvješća za budući mjesec.
- **Predaja** izvješća za budući mjesec odbija se na backendu do prvog dana tog mjeseca po vremenskoj zoni `Europe/Zagreb`.
- Korisnik u pregledu odjela dobiva poruku i deaktiviran gumb predaje za buduće razdoblje.
- Planovi se mogu predati unaprijed; izmjene budućih mjeseci nisu blokirane.

## b) Prijenos iz izvješća u plan

U prikazu **Mjesečni unos → Plan** nalazi se opcija **Prenesi nedovršene iz prošlog izvješća**. Prikazuje se popis neizvršenih aktivnosti prethodnog mjeseca s pojedinačnim odabirom.

- Kao izvor koristi `activity_updates.kind='report'` prethodnog mjeseca, uključujući prijelaz iz siječnja na prethodni prosinac.
- Predlaže stavke sa statusom `in_progress`, `postponed` ili `not_started`; trajno završene ili otkazane aktivnosti ne prenosi.
- Korisnik sam označava koje želi prenijeti.
- Kreira prazne planske zapise (`kind='plan'`, `progress='not_started'`) za novi mjesec. Nema kopiranja realizacije.
- Postojeći planski zapis se ne prepisuje i ne duplicira.
- Predani plan nije moguće mijenjati prijenosom dok se ne vrati u nacrt.
- Prijenos obavlja voditelj odjela ili administrator, uz provjeru ovlasti na API-ju.

## c) Uređivanje naziva aktivnosti

U Registru, ovlaštenom voditelju ili administratoru dostupan je gumb **Uredi**. Otvara jednostavna polja za naziv i opis aktivnosti.

- Aktivnost zadržava isti UUID, poveznice na plan, izvješće i rasprave.
- Svaka promjena naziva/opisa putem ove operacije zapisuje se u `activity_field_changes` (stara i nova vrijednost, korisnik, timestamp).
- Promjena naziva ne mijenja već predane snapshotove izvješća.
- Povijest promjena dostupna je odmah u obrascu za uređivanje.

## d) HTML prefiks

HTML pregled **predane verzije odjela i sektorske revizije** kod mjesečnog zapisa ispisuje **Mjesečna realizacija:** za izvješće, odnosno **Mjesečni plan:** za plan. Prikaz je uvjetovan `kind` dokumenta, tako da plan ne prikazuje pogrešno naziv realizacije.

## Tehničke napomene

- Nova migracija `backend/migrations/0003_activity_field_changes.sql`. Prethodne migracije ne mijenjaju se.
- Prije produkcijske primjene napraviti backup baze.
- Provesti ručni test predaje budućeg izvješća, izbora i prijenosa stavki, evidentiranja povijesti te HTML prikaza oba tipa dokumenta.
- Po korisnikovu zahtjevu **ne provjerava se Rust build**. React build i statička provjera UI-ja nisu zamjena za testiranje API-ja/migracija na testnoj bazi.
