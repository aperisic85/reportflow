# ReportFlow — jednostavan radni prostor

## Cilj

Početna stranica i radni ekrani trebaju podsjećati na jednostavan uredski dokument: svijetla radna stranica, obični redci, nekoliko jasnih naredbi. Zadržana je paleta grafit/siva/narančasta/žuta, ali s manje vizualnih naglasaka.

## Promjene

- **Početna:** umjesto četiri KPI kartice, istaknutog hero panela i panela s brzim radnjama prikazuje kratki sažetak, obične poveznice, popis odjela i opcionalno posljednje izmjene.
- **Mjesečni unos:** Plan i Izvješće kao dvije jednostavne kartice/oznake, po jedna kratka uputa; aktivnosti kao redci koji se otvaraju klikom na naslov. Opis iz registra i mjesečni plan čitaju se na zahtjev.
- **Registar:** redci s nazivom i statusom, dodatni opis, hijerarhija i redoslijed tek pod "Detalji". Nova aktivnost unosi se iz sklopivog obrasca.
- **Globalno:** manje bordera, sjena i grafičkih oznaka; ujednačena širina sadržaja i prored, jednostavnija navigacija.

## Očuvano

Spremanje, statusi, ovlasti, promjena hijerarhije, mjesečni plan/izvješće, pretraga, prijenos nedovršenih i izrada službenih dokumenata rade putem postojećih API poziva; nema migracija ni promjena Rust backenda.

## Ručna provjera

1. Uđi kao djelatnik i voditelj: pregledaj Plan i Izvješće, otvori redak, spremi sadržaj i ponovno ga otvori.
2. Testiraj pretraživanje i filtre tijekom mjesečnog unosa.
3. Pregledaj Registar: detalji, povijest, nova aktivnost i promjena redoslijeda.
4. Kao direktor pregledaj početni popis svih odjela i poveznice.
5. Provjeri mobilni prikaz, tipkovničku navigaciju i izgled dugih naziva aktivnosti.

**Napomena:** ova promjena namjerno nije dodavanje novih funkcionalnosti. Riječ je o uklanjanju suvišne vizualne složenosti.
