# ReportFlow vNext – radne upute

## Obuhvat

Ova verzija proširuje postojeći MVP bez mijenjanja `0001_init.sql` ili postojećih podataka.

1. **Mjesečni unos → Prenesi nedovršene iz prošlog mjeseca**: Voditelj može prenijeti zapise trajnih i projektnih aktivnosti sa statusom realizacije različitim od `completed`, ako su osnovne aktivnosti i dalje `in_progress`. Ne kopira se stara mjesečna realizacija (`content` ostaje prazan), jer nije dokaz da je posao izvršen u novom mjesecu. Postojeći zapisi novog mjeseca nikada se ne prepisuju (`ON CONFLICT DO NOTHING`).
2. **Povezivanje plana i izvješća**: Dok korisnik uređuje izvješće, vidi relevantan plan istog mjeseca u plavom informativnom okviru. Tekst plana ne postaje automatski realizacija. Nije potrebno prethodno stvoriti izričiti mjesečni plan: u tom slučaju koristi se stalni opis aktivnosti iz registra.
3. **Registar aktivnosti → redoslijed i podstavke**: Voditelj i administrator mogu pomicati stavke gore/dolje unutar istog roditelja i premjestiti ih pod drugu aktivnost. Backend sprječava cikluse te pripadnost drugom odjelu.
4. **Sektor / Uprava → Uredi / pregledaj**: Direktor otvara nacrt nove sektorske revizije, uređuje naslov, opis i mjesečnu realizaciju stavaka, uklanja ili premješta stavke i sprema. Mijenja se samo `sector_documents.snapshot` u toj reviziji. Izvorne aktivnosti i predani dokumenti odjela ostaju nepromijenjeni. Zaključane revizije se ne mogu uređivati.

## Važno

- **Uklanjanje stavke** iz sektorskog nacrta ne briše aktivnost iz registra. Novi sektorski nacrt može ponovno preuzeti izvorne stavke.
- Ako se uređuje podstavka, pomicanje je unutar istog roditelja. Word generator automatski numerira prema hijerarhiji.
- Sve stare zaključane revizije ostaju nepromijenjene.
- Privremeni prikaz u pregledniku nije pravi WYSIWYG Word prikaz; konačno oblikovanje određuje `document-service`.
- Nakon slanja odjelnog dokumenta, naknadna promjena registra može utjecati na sljedeće generiranje sektorskog nacrta. Za formalnu zamrznutu predaju odjela preporučena je zasebna snapshot funkcija u sljedećoj fazi.

## Deploy

```bash
# iz korijena reportflow repozitorija
cp .env.example .env  # samo pri prvoj instalaciji; nemoj prepisati postojeći .env
# Ako .env već postoji, NEMOJ ga ponovno kreirati.
docker compose up -d --build api frontend
```

Prije nadogradnje produkcijske instance napravi backup:

```bash
docker compose exec -T db pg_dump -U reportflow -d reportflow -Fc > reportflow-before-vnext.dump
```

Nakon nadogradnje: provjeri četiri funkcionalnosti na testnom mjesecu prije rada s pravim izvješćima.