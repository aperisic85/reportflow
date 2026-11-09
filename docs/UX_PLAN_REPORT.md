# UX unosa mjesečnih planova i izvješća

## Ključne razlike

**Registar aktivnosti** = trajna baza poslova i projekata. Tu se unose naziv, osnovni opis, vrsta aktivnosti i hijerarhija. Nije mjesto za unos mjesečne realizacije. Duge postavke za hijerarhiju i redoslijed dostupne su tek klikom na "Detalji".

**Mjesečni unos → Plan** = što je namjera obaviti tijekom odabranog mjeseca. Polje je nazvano **Što planiramo napraviti?**; status realizacije se ne prikazuje jer aktivnost još nije realizirana.

**Mjesečni unos → Izvješće** = što je stvarno obavljeno tijekom odabranog mjeseca. Polje je nazvano **Što je stvarno realizirano?**; uz njega se prikazuje plan za isti mjesec ako postoji i status realizacije.

Svaki od ta dva tipa ima vlastite zapise u bazi (`activity_updates.kind` = `plan` odnosno `report`). Promjena tipa ne pretvara plan u izvješće niti mijenja postojeće podatke. Odabir statusa u izvješću ne mijenja trajni status aktivnosti u Registru.

## Poboljšanja
- Vizualno istaknuti način rada Plan / Izvješće, s kratkim uputama
- Kartice aktivnosti koje se otvaraju prema potrebi, sa stanjem **Spremljeno/Nije uneseno**
- Oznaka promijenjenog sadržaja prije spremanja
- Pretraživanje po nazivu/opisu te filter **sve / bez unosa / sa spremljenim unosom**
- Kratki sažetak broja aktivnosti i mjesečnih zapisa
- Odvojeni opis iz registra, plan i stvarna realizacija
- Sažetiji Registar aktivnosti, s rijetko korištenim postavkama iza gumba Detalji
- Jasna terminologija u pregledu dokumenta odjela i pri predaji plana

## Ručne provjere nakon instalacije
1. Otvori Plan, unesi mjesečni zadatak i spremi. Promijeni na Izvješće — uneseni plan mora ostati odvojen, a u izvješću se mora pojaviti u informativnom bloku.
2. U izvješću odaberi status realizacije, unesi realizaciju i spremi; provjeri da ponovnim otvaranjem ostaje spremljeno.
3. Provjeri da se broj spremljenih zapisa i filter "Bez mjesečnog unosa" ažuriraju nakon spremanja.
4. Provjeri različite mjesece, odjele, prazna polja, duge nazive i mobilni prikaz.
5. Provjeri prava korisnika: djelatnik, voditelj, direktor, administrator.
6. Potvrdi da predaja dokumenta zaključava uređivanje i da se prikazuje odgovarajući status.

## Napomene
- Nema promjena sheme baze niti API ruta.
- Ovo je vizualna i terminološka dorada postojećeg React sučelja.
- Provjera Rust builda nije dio ove promjene.
