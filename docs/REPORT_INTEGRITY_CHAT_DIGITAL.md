# ReportFlow — integritet izvješća, digitalni pregled i rasprave

## 1. Integritet i verzioniranje izvješća

- Svaka **nova predaja** odjela stvara verziju (`department_document_versions`) s brojem, autorom, vremenom i punim JSON snapshotom uključenih aktivnosti.
- Povrat u nacrt i dorada ostaju dopušteni. Ponovna predaja stvara sljedeću verziju; prijašnja ostaje netaknuta.
- Direktor pri izradi sektorskog izvješća koristi **isključivo spremljene predane verzije**, ne trenutne podatke aktivnosti. U svakoj sekciji objedinjene revizije pohranjuje se `source_version`.
- Ako odjel naknadno preda novu verziju, direktor u popisu sektorskih dokumenata vidi **Novija verzija odjela**. Stara sektorska revizija ostaje nepromijenjena.
- Svaka izmjena mjesečnih unosa automatski stvara novi zapis u `activity_update_revisions` sa sadržajem, autorom i datumom/vremenom. Promjene nacrta sektorskog dokumenta spremaju se u `sector_document_edits`.
- Postojeći audit događaji ostaju aktivni.

**Važno za povijesne podatke:** migracija za postojeće predane dokumente stvara jednu *početnu rekonstruiranu verziju* iz trenutno raspoloživih podataka. Ne može rekonstruirati prijašnje izmjene koje stara aplikacija nije čuvala.

## 2. Aktivnost kao mali projekt: Chat / Rasprava

U Registru klik na naziv aktivnosti ili **Chat** otvara razgovore te aktivnosti.

- Voditelj odjela ili administrator otvara raspravu s nazivom i početnom porukom te odabire sudionike.
- Aktivnost može imati više rasprava (npr. "Dopuniti izvješće", "Dostaviti podatke", "Riješiti alarm").
- Sudionici čitaju i pišu poruke. Voditelj može dodavati osobe i označiti raspravu dovršenom ili je ponovno otvoriti.
- Poruke su neizmjenjivi zapisi s autorom i vremenom; posebna tablica bilježi otvaranje rasprave, dodavanje osobe i promjenu statusa.
- Razgovori nisu javni: pristup imaju sudionici, voditelji relevantnog odjela i direktor/administrator u okviru svoje uloge.

## 3. Notifikacije / Inbox

- U bočnoj navigaciji je **Inbox** s brojem nepročitanih obavijesti.
- Poziv u raspravu i nova poruka generiraju trajnu obavijest u `user_notifications`.
- Klik u Inboxu otvara točnu aktivnost i raspravu; čitanjem se obavijesti označavaju pročitanima.
- Broj nepročitanih se u otvorenoj aplikaciji osvježava otprilike svakih 30 sekundi. Nema push obavijesti na mobitel ili e-mail u ovoj fazi.

## 4. Digitalno izvješće kao HTML

- Predane odjelne verzije otvaraju se kao HTML stranice s datumom, autorom i brojem verzije.
- Direktor za svaku sektorsku reviziju može kliknuti **HTML pregled**.
- Zaglavlje, sekcije odjela, aktivnosti, uvod i zaključak prikazuju se za čitanje u aplikaciji. Word/PDF ostaju dostupni kao zaseban službeni ISO izvoz.
- HTML koristi React tekstualno renderiranje, bez interpretacije korisničkog HTML-a.

## Instalacija i provjere

1. **Prije primjene nove migracije napraviti backup PostgreSQL baze.**
2. Provjeriti da stara migracija `0001_init.sql` nije mijenjana. Novi model podataka dolazi u **`0002_report_integrity_collaboration.sql`**.
3. Testirati predaju dokumenta odjela (v1), povrat u nacrt, dopunu i ponovnu predaju (v2). Obje verzije moraju ostati dostupne.
4. Izraditi objedinjeni dokument, zatim predati noviju verziju odjela i provjeriti upozorenje o novijoj verziji.
5. Testirati raspravu s voditeljem i djelatnikom: poziv, slanje poruke, Inbox, oznaku pročitanog, dodavanje korisnika i dovršetak rasprave.
6. Provjeriti pristup: djelatnik iz drugog odjela ne smije čitati tuđe rasprave; sudionici i voditelji vide samo ono što im pripada.
7. Provjeriti HTML pregled odjela i direktora na desktopu/mobitelu.

## Ograničenja prve verzije

- Nema privitaka, e-mail notifikacija, push poruka niti stvarnog WebSocket chata; ovo je jednostavno dopisivanje uz osvježavanje stranice/Inboxa.
- Nema automatskog prihvaćanja promjena odjela u već izrađene objedinjene revizije.
- Ovo je nova migracija i opsežna backend promjena, zbog čega zahtijeva funkcionalno testiranje na testnoj bazi prije produkcije.
- **Rust build nije provjeravan prema korisnikovu zahtjevu.** Prolazak React builda ne dokazuje ispravnost SQL migracija ni Rust endpointa.
