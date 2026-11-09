# ReportFlow — Graphite / Ember / Saffron

## Vizualni smjer

Moderan, miran poslovni izgled bez generičke plave SaaS palete. Odabrane su tamne površine za navigaciju, svijetla topla podloga za sadržaj i narančasta za primarne radnje. Žuta daje selektivni naglasak za aktivnu navigaciju i informacije koje trebaju pažnju.

| Element | Boja | HEX |
|---|---|---|
| Navigacija / grafit | Grafitno crna | `#242321` |
| Glavni tekst | Skoro crna | `#201E1B` |
| Pozadina | Topla siva | `#F5F2EC` |
| Površine kartica | Topla bijela | `#FFFDFA` |
| Primarna akcija | Narančasta | `#DC630C` |
| Hover primarne akcije | Tamna narančasta | `#AF4505` |
| Naglasak | Žuta / šafran | `#F7C645` |
| Rubovi | Svijetlosiva topla | `#E9E2D7` |

## Obuhvat

- Bočna navigacija i brand
- Početni dashboard, naslovni paneli, KPI kartice i brze radnje
- Mjesečni plan/izvješće, objašnjenja i kartice aktivnosti
- Registar aktivnosti, povijest i pretraživanje
- Uređivač sektorskih dokumenata i pregled dokumenta
- Prijava, forme, fokus, gumbi, mobilni prikaz

Sve promjene su CSS-only. **Nema promjena API-ja, modela, baze ni poslovne logike.**

## Provjere prije mergea

1. Na desktopu i mobitelu provjeriti bočnu navigaciju, posebno aktivnu stavku i horizontalni scroll.
2. Provjeriti fokus tipkovnicom, kontrast teksta i gumba te disabled stanja.
3. Pregledati mjesečni Plan i Izvješće, uključujući prazne/spremljene kartice.
4. Otvoriti direktorov dashboard, urednik dokumenata i povijest aktivnosti.
5. Provjeriti crvene poruke greške i zelene potvrde — semantičke boje ostale su odvojene od brendinga.

## Napomena

Postojeći CSS sadrži naslijeđene boje. Neutralizirane su i usklađene s novom paletom, a ključni dijelovi su dodatno stilizirani posebnim pravilima na kraju datoteke.
