# cur-top

[![crates.io](https://img.shields.io/crates/v/cur-top.svg)](https://crates.io/crates/cur-top) [![downloads](https://img.shields.io/crates/d/cur-top.svg)](https://crates.io/crates/cur-top) [![CI](https://github.com/qwpp7g79y4-maker/cur-top/actions/workflows/ci.yml/badge.svg)](https://github.com/qwpp7g79y4-maker/cur-top/actions/workflows/ci.yml) [![PEPAI checked](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fqwpp7g79y4-maker%2Fcur-top%2Fmain%2F.github%2Fbadge.json)](QUALITY.md) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://github.com/qwpp7g79y4-maker/cur-top/blob/main/Cargo.toml) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/qwpp7g79y4-maker/cur-top#license)

Offline top-kosten uit een AWS CUR 2.0-CSV. De tool telt bedragen op die al in het bestand staan.

## Waarom

`cur-top` leest een Cost and Usage Report 2.0-export en toont de grootste diensten, gebruiksoorten en resourcetags van de laatste factuurmaand, met het verschil ten opzichte van de maand ervoor.

Hij leest alleen de lokale `.csv`- of `.csv.gz`-bestanden die je noemt. Hij gebruikt geen netwerk, geen AWS SDK, geen account, geen token en geen betaalde dienst. Hij schrijft niets weg, behalve met `--output`. Hij telt de kostenkolom op die al in het bestand staat; hij vermenigvuldigt geen tarief met een gebruikshoeveelheid en schat geen prijs.

## Installatie

Rust 1.85 of nieuwer.

```sh
cargo install --locked --path .
```

De crate heeft geen git-tag. Releasearchieven voor Linux, Windows en macOS worden gebouwd door `.github/workflows/release.yml` zodra een `v*`-tag wordt gepusht; zo'n tag bestaat nog niet. De ondersteunde systemen staan onder Beperkingen (Systeemeisen).

## Snelstart

```sh
cat > my-cur.csv <<'CSV'
bill_billing_period_start_date,line_item_product_code,line_item_usage_type,line_item_unblended_cost,line_item_unblended_rate,line_item_usage_amount,resource_tags
2026-11-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,3.50,0.0104,336,"{""user_team"":""platform""}"
2026-11-01T00:00:00Z,AmazonS3,TimedStorage-ByteHrs,1.00,0.023,43,"{""user_team"":""platform""}"
2026-10-01T00:00:00Z,AmazonEC2,BoxUsage:t3.micro,1.00,0.0104,96,"{""user_team"":""platform""}"
CSV
```
De repository bevat hetzelfde bestand als `examples/cur.csv`. De opdrachten hieronder lezen die kopie, zodat niets in de repository wordt overschreven.

```console
$ cur-top examples/cur.csv
cur-top
dictionary: https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html
read: 2026-11-09
amount: line_item_unblended_cost
month: 2026-11
previous: 2026-10
previous_month_present: yes
rows: 3
rows_without_amount: 0
rows_without_tags: 0
rows_outside_months: 0

service
AmazonEC2 amount=3.50 previous=1.00 delta=2.50
AmazonS3 amount=1.00 previous=0.00 delta=1.00

usage_type
BoxUsage:t3.micro amount=3.50 previous=1.00 delta=2.50
TimedStorage-ByteHrs amount=1.00 previous=0.00 delta=1.00

tag
user_team=platform amount=4.50 previous=1.00 delta=3.50

totals
amount=4.50 previous=1.00 delta=3.50
```
## Aan de slag in 5 minuten

Korte route; de Engelse tekst staat in [README.md](README.md).

1. Installeer met `cargo install --locked --path .` (Rust 1.85 of nieuwer).
2. Neem een CUR 2.0-bestand (`.csv` of `.csv.gz`), of gebruik `examples/cur.csv`.
3. Draai `cur-top examples/cur.csv` voor de tekstuitvoer, of `--json` of `--md`.
4. Lees de bedragen per dienst, gebruiksoort en tag: huidige maand, vorige maand en het verschil. Alle bedragen komen uit het bestand zelf; er is niets geschat.
5. Een fout geeft exit 2 en één regel op standaardfout; lees "Afsluitcodes" voor de rest.

## Opties

```text
cur-top [OPTIONS] <FILE>...
```

- `<FILE>...`: een of meer AWS CUR 2.0-bestanden (`.csv` of `.csv.gz`). Minstens één is verplicht. Standaardinvoer wordt niet gelezen.
- `--amount <COLUMN>`: de kostenkolom die wordt opgeteld: `line_item_unblended_cost` (standaard), `line_item_blended_cost` of `line_item_net_unblended_cost`.
- `--top <N>`: hoeveel sleutels per tabel (dienst, gebruiksoort, tag). 1 tot 10000, standaard 10. De totalen bevatten elke rij van de twee maanden, niet alleen de getoonde sleutels.
- `--json`: JSON in plaats van het tekstrapport.
- `--md`: Markdown-tabellen in plaats van het tekstrapport. Niet samen met `--json`.
- `--output <FILE>`: schrijf het rapport naar een bestand in plaats van naar standaarduitvoer.
- `--quiet`, `--verbose`: alleen standaardfout verandert. Ze kunnen niet samen.
- `--help` (`-h`) en `--version` (`-V`). Een debugbouw zet de git-commit achter de versie.

`--quiet` en `--verbose` veranderen alleen standaardfout: een geslaagde run print daar niets, en `--verbose` voegt één regel `context:` toe (gelezen rijen en getoonde sleutels per tabel). Het rapport op standaarduitvoer en de afsluitcodes blijven gelijk; beide samen is een gebruiksfout (exit 2).

`--output FILE` schrijft het rapport naar een bestand in plaats van standaarduitvoer (die dan leeg blijft). Het rapport gaat eerst naar een nieuw bestand in dezelfde map en vervangt `FILE` daarna door een hernoeming, zodat `FILE` het hele rapport bevat of ongewijzigd blijft. Een bestaand bestand wordt vervangen; een symbolische link op `FILE` wordt als link vervangen en nooit doorgeschreven; een pad dat bestaat en een pijp, een socket of een apparaat is wordt geweigerd met `not a regular file` en blijft zoals het was; een ontbrekende map wordt niet aangemaakt; een `FILE` dat ook een invoerbestand is wordt geweigerd (twee hardlinks naar één bestand worden alleen op Unix als hetzelfde bestand gezien, niet op Windows). Een schrijffout (ontbrekende map, een map als `FILE`, volle schijf, geen rechten) geeft exit 2 met `cur-top: could not write report to <pad>: <reden>`. Een invoerfout laat een bestaand `FILE` ongemoeid.

Het JSON-rapport begint met `"schema_version": 1`. Het getal verandert als een veld wordt verwijderd, hernoemd of van betekenis verandert; een nieuw veld verandert het niet. Een mislukte run print geen JSON maar één foutregel op standaardfout en exit 2, dus elk JSON-document heeft het veld.

## Voorbeeld

De opdrachten hieronder lezen het meegeleverde `examples/cur.csv` en schrijven niets weg. De verwachte uitvoer van het tekstrapport staat onder Snelstart.

```console
$ cur-top --json examples/cur.csv
{
  "schema_version": 1,
  "dictionary": "https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html",
  "dictionary_date": "2026-11-09",
  "amount_column": "line_item_unblended_cost",
  "month": "2026-11",
  "previous_month": "2026-10",
  "previous_month_present": true,
  "rows": 3,
  "rows_without_amount": 0,
  "rows_without_tags": 0,
  "rows_outside_months": 0,
  "service": [
    {
      "key": "AmazonEC2",
      "amount": "3.50",
      "previous": "1.00",
      "delta": "2.50"
    },
    {
      "key": "AmazonS3",
      "amount": "1.00",
      "previous": "0.00",
      "delta": "1.00"
    }
  ],
  "usage_type": [
    {
      "key": "BoxUsage:t3.micro",
      "amount": "3.50",
      "previous": "1.00",
      "delta": "2.50"
    },
    {
      "key": "TimedStorage-ByteHrs",
      "amount": "1.00",
      "previous": "0.00",
      "delta": "1.00"
    }
  ],
  "tag": [
    {
      "key": "user_team=platform",
      "amount": "4.50",
      "previous": "1.00",
      "delta": "3.50"
    }
  ],
  "total": {
    "amount": "4.50",
    "previous": "1.00",
    "delta": "3.50"
  }
}
```
`--md` print dezelfde bedragen als Markdown-tabellen, één per groep, in plaats van het tekstrapport. Het kan niet samen met `--json` (gebruiksfout, exit 2). Een `|` in een sleutel wordt `\|` en een regeleinde of ander verborgen teken wordt `\u{..}`, zodat een sleutel in zijn cel blijft. Een backslash, een backtick en `* _ [ ] ( ) < > & ! ~ @` krijgen een backslash ervoor (een `#` aan het begin, een `:` voor `/` en de `.` na `www` ook), zodat een renderer een sleutel als tekst toont en niet als link, afbeelding, HTML of kop. Bedragen worden niet opnieuw afgerond. Het rapport zegt dat de bedragen uit het bestand zijn opgeteld en geen schatting zijn.

```console
$ cur-top --md examples/cur.csv
# cur-top

Month 2026-11 against 2026-10 (amount column `line_item_unblended_cost`). 3 rows read.

Not an estimate: the amounts are summed from the file.

## Service

| Key | Amount | Previous | Delta |
| --- | ---: | ---: | ---: |
| AmazonEC2 | 3.50 | 1.00 | 2.50 |
| AmazonS3 | 1.00 | 0.00 | 1.00 |

## Usage type

| Key | Amount | Previous | Delta |
| --- | ---: | ---: | ---: |
| BoxUsage:t3.micro | 3.50 | 1.00 | 2.50 |
| TimedStorage-ByteHrs | 1.00 | 0.00 | 1.00 |

## Tag

| Key | Amount | Previous | Delta |
| --- | ---: | ---: | ---: |
| user\_team=platform | 4.50 | 1.00 | 3.50 |

## Totals

| Amount | Previous | Delta |
| ---: | ---: | ---: |
| 4.50 | 1.00 | 3.50 |
```
## Hoe het telt

C1. Elke rij telt `line_item_unblended_cost` op zoals geschreven. De tool vermenigvuldigt `line_item_unblended_rate` niet met `line_item_usage_amount`.
C2. De kolommen zijn de CUR 2.0-namen `bill_billing_period_start_date`, `line_item_product_code`, `line_item_usage_type`, `line_item_unblended_cost`, `line_item_blended_cost`, `line_item_net_unblended_cost` en `resource_tags`. Alle zijn verplicht behalve `line_item_blended_cost` en `line_item_net_unblended_cost`; `resource_tags` is een verplichte kolom (in een rij mag hij leeg zijn). Een naam in de kop moet exact kloppen: hij wordt niet getrimd, dus een kopcel met een spatie om de naam telt als ontbrekende kolom. De twee andere kostenkolommen zijn alleen verplicht als je ze met `--amount` kiest. Bron: https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html gelezen op 2026-11-09.
C3. `--amount` accepteert alleen `line_item_unblended_cost`, `line_item_blended_cost` of `line_item_net_unblended_cost`.
C4. De maand is de `YYYY-MM` van `bill_billing_period_start_date`. De huidige maand is de laatste maand in de bestanden, ook als al haar bedragen leeg zijn (haar bedragen zijn dan `0.00`). De vorige maand is de kalendermaand ervoor. De maand `0000-01` is geldig en heeft geen vorige maand: het rapport toont `previous: omitted`.
C5. Het verschil (delta) is het huidige bedrag min het vorige bedrag.
C6. Een leeg bedrag wordt overgeslagen, en de fout voor een slecht bedrag bevat de cel niet. ASCII-spatie, tab, CR en LF rond een bedrag worden verwijderd: een tab aan een kant van `1.00`, of een CR of LF aan een kant van een geciteerd `1.00`, blijft 1.00. Een form feed, een verticale tab of een no-break space aan een kant wordt niet verwijderd en maakt het bedrag ongeldig.
C7. Rijen uit andere maanden dan de huidige en de vorige blijven buiten de totalen.
C8. Heeft de vorige kalendermaand geen rijen, dan zijn de vorige bedragen `0.00` en is `previous_month_present` gelijk aan `no`.
C9. Een negatief bedrag in de CUR, zoals een tegoed, wordt opgeteld zoals geschreven.
C10. Dezelfde rij in twee bestanden wordt twee keer geteld.
C11. Elk `resource_tags`-paar `key=value` krijgt het volle regelbedrag. Een regel met twee tags telt bij beide. In een tagregel wordt `\` in de sleutel en in de waarde `\\`, en `=` in de sleutel wordt `\=`, zodat `a=b`/`c` en `a`/`b=c` uit elkaar blijven, en de waarden ` x` en `\u{20}x` ook. `--json` toont dezelfde tagregel. De taglimieten staan in `src/tags.rs`: hoogstens 50 tags per cel, een sleutel van 1 tot 128 tekens, een waarde van hoogstens 256 tekens, geen stuurtekens, geen dubbele sleutel en een cel van hoogstens 65536 bytes. JSON-escapes in een tagcel worden gedecodeerd, en een surrogaatpaar zoals `\ud83d\ude00` is één teken.
C12. Een regel met een lege `resource_tags`-cel telt wel mee voor de dienst en de gebruiksoort.
C13. De tabellen tonen de top `--top` sleutels op huidig bedrag, daarna op sleutel. `--top` loopt van 1 tot en met 10000. Een sleutel met huidig bedrag nul valt uit de tabellen. De totalen bevatten elke rij van de twee maanden.
C14. Data Exports gzip/csv wordt gelezen als de eerste bytes de gzip-magic `1f 8b` zijn, wat de bestandsnaam ook is, ook `.csv`. Een naam die op `.gz` eindigt moet gzip bevatten: zonder de magic wordt het bestand geweigerd met `not gzip` en exit 2. Parquet en zip worden geweigerd. Athena wordt niet gebruikt. Zie https://docs.aws.amazon.com/cur/latest/userguide/dataexports-export-delivery.html gelezen op 2026-11-09.
C15. `line_item_usage_account_id` en andere extra kolommen worden genegeerd en niet getoond.
C16. Er is geen netwerkaanroep en geen AWS SDK.
C17. Een bedrag met meer dan 28 decimalen, of cijfers die niet onafgerond in een decimaal van 28 cijfers passen, is ongeldig en geeft exit 2; er wordt nooit stil afgerond. Een bedrag heeft hoogstens 40 tekens. Een lege productcode of gebruiksoort, of een waarde langer dan 1024 bytes, is ongeldig. Een maandtotaal of verschil dat niet exact in een decimaal van 28 cijfers past, faalt met `amount total does not fit`; het wordt nooit afgerond. Elke fout geeft exit 2. Een ongeldig bedrag, productcode, gebruiksoort, factuurtijdstip of tagcel faalt zonder de cel te echoën.
C18. Uitvoerbedragen zijn decimale tekst met minstens twee decimalen en zonder wetenschappelijke notatie.

## Afsluitcodes

| Code | Betekenis |
| --- | --- |
| 0 | Het rapport is geschreven (naar standaarduitvoer, of naar `--output`). |
| 2 | Een invoer- of gebruiksfout: een onleesbaar of niet ondersteund bestand, een ongeldige cel, een ontbrekende kolom, een foute optie, een `--output` die niet te schrijven is of een invoerbestand noemt, of een standaarduitvoer die niet te schrijven is (`could not write output`). Eén regel op standaardfout, geen rapport. |
| 130 | SIGINT (Ctrl-C) op Unix. |

Op Unix stopt SIGINT (Ctrl-C) tijdens het lezen van de bestanden de run met exit 130: geen rapport, niets op standaarduitvoer en één regel `cur-top: interrupted by SIGINT` op standaardfout. Windows houdt zijn standaardgedrag bij Ctrl-C.

## Beperkingen

Versie 1.0.0 leest alleen AWS CUR 2.0-CSV. Invoer is `.csv` of gzip `.csv.gz` (als gzip gelezen als het bestand met de magic `1f 8b` begint). Parquet en zip worden geweigerd. Athena wordt niet gebruikt. Er is geen netwerk en geen AWS SDK. De tool vermenigvuldigt `line_item_unblended_rate` niet met `line_item_usage_amount`. Tags komen uit `resource_tags` als JSON-object, en elke tag op een regel krijgt het volle bedrag van die regel. Dezelfde rij in twee bestanden wordt twee keer geteld. Een symlink wordt gevolgd. Een map wordt geweigerd. Een bestand groter dan 536870912 uitgepakte bytes wordt geweigerd. Een gzip-bestand wordt geweigerd als de gecomprimeerde bytes van één member meer dan 1048576 boven de bytes liggen die die member uitpakt, als het bestand meer dan 40000 members heeft, of als de gecomprimeerde bytes van alle members van alle invoerbestanden samen 1073741824 overschrijden (`gzip input is larger than 1073741824 bytes`, exit 2, geen rapport). Gemeten op de releasebinary, met één CUR-member en daarna stored members van 1048573 bytes (209711 lege stored blocks, net onder de overhead per member) en één kortere stored member zodat het bestand exact is: 268435456 bytes eindigde met 0 in 0.77 s (piek 2904 KB), 536870912 bytes met 0 in 1.52 s (piek 2624 KB) en 1073741824 bytes met 0 in 3.10 s (piek 2796 KB). De gemeten tijd groeide met 1.97 keer en met 4.03 keer. Een bestand van 1073741825 bytes eindigde met 2 met die melding en geen rapport in 3.12 s (piek 2848 KB). De vorige maand is de kalendermaand voor de laatste factuurperiode in het bestand. De tabellen zijn geen voorspelling. De kwaliteitstest dekt een CSV van minstens 100 MiB. De testreeks van 168 tests (geteld op Linux; elf tests draaien alleen op Linux) en de meting van 100 MiB staan in [QUALITY.nl.md](QUALITY.nl.md).

Meer grenzen, elk gemeten op de releasebinary (details in [QUALITY.nl.md](QUALITY.nl.md)):

- Bestandsnamen. De controle op het achtervoegsel let op hoofdletters: een naam moet op `.csv` of `.gz` in kleine letters eindigen, dus `UPPER.CSV` en `export.CSV` falen met `not a csv or csv.gz` en exit 2 (tenzij het bestand met de gzip-magic begint). Op Linux wordt een naam die geen geldige UTF-8 is gelezen als hij op `.csv` eindigt; in een foutmelding staat hij als bytes, bijvoorbeeld `bad\xff.txt`.
- Tekstrapport. In de sleutels van dienst en gebruiksoort wordt een spatie, een stuurteken of een onzichtbaar of richtingwisselend teken getoond als `\u{..}`, een backslash als `\\`, en de `=` van `amount=`, `previous=` en `delta=` als `\=`, zodat twee verschillende sleutels nooit dezelfde regel geven. `--json` toont de sleutels zoals ze zijn, dus een richtingwisselend of onzichtbaar teken in een sleutel staat daar ruw (tagregels houden de escaping van C11); lees `--json` met een programma, niet in een terminal die zulke tekens uitvoert. Een gebruiksfout op standaardfout toont een stuurteken, richtingwisselend of onzichtbaar teken van je argumenten als `\u{..}` op één regel, ook na een escapereeks en ook in een argument dat geen geldige UTF-8 is (getoond met U+FFFD). Een bestandsnaam in een foutmelding toont stuurtekens en onzichtbare tekens ook als `\u{..}`.
- Invoerregels. Een bestand is UTF-8-tekst, met een BOM vooraan en CRLF-regeleinden toegestaan. `bill_billing_period_start_date` is een UTC-tijd met een `Z`, met 1 tot 6 breukcijfers als er een breuk is, bijvoorbeeld `2026-11-01T00:00:00Z` of `2026-11-01T00:00:00.123456Z`; een datum alleen, een offset zoals `+01:00`, een spatie in plaats van de `T`, een kleine `t` of `z` en 7 breukcijfers zijn ongeldig. ASCII-spatie, tab, CR en LF rond het factuurtijdstip, de productcode, de gebruiksoort, het bedrag en `resource_tags` worden verwijderd. Een CR of LF wordt alleen verwijderd als hij binnen de cel staat, dus die cel is geciteerd; een ongeciteerde CR of LF beëindigt het record. Een form feed, een verticale tab of een no-break space aan een kant wordt niet verwijderd en maakt dat veld ongeldig. De CSV-lezer is tolerant: een geciteerde cel gevolgd door tekst, zoals `"A"x`, wordt gelezen als de waarde `Ax`. Standaardinvoer wordt niet gelezen: `-` is een bestandsnaam en faalt met `No such file or directory`, en `/dev/stdin` faalt (`not a file` voor een pipe of een terminal).
- Rijnummers. Het nummer in `row N` van een fout is het record van dat bestand, geteld vanaf 1 na de kop; een geciteerde cel met regeleinden is één record. Elk bestand begint opnieuw bij 1, en de melding noemt het bestand, bijvoorbeeld `file 2: row 1: invalid amount`.
- Geheugen. Het geheugengebruik is ongeveer 24 keer de invoer als bijna elke rij een eigen dienst- en gebruiksoortsleutel heeft: een bestand van 13 MB met 300000 unieke sleutels gebruikte 314 MiB, en een bestand van 92 MB met 2000000 unieke sleutels 2.1 GiB. Een bestand van 14 MB met 20000 rijen van elk 25 unieke tags gebruikte 303 MiB. De limiet van 536870912 bytes geldt voor de invoer, niet voor deze toestand; bij extrapolatie (niet gedraaid) zou een bestand op die limiet met alleen unieke sleutels ongeveer 12 GiB nodig hebben. Een enkele cel wordt geheel in het geheugen gehouden: een productcode van 400 MiB (een bestand van 400 MiB) werd geweigerd met `invalid line_item_product_code` en exit 2 na 913 MiB gebruik. Een bestand met weinig verschillende sleutels heeft weinig geheugen nodig.
- Een bestand dat tijdens het lezen wordt afgekapt. Een CSV heeft geen controlesom, dus een afkapping midden in het laatste veld van de laatste rij, zonder slotnieuweregel, kan onopgemerkt blijven als het bedrag de laatste kolom is: `...,2.50` afgekapt tot `...,2` wordt als 2 opgeteld. Een afkapping die `2.` of een korte rij overlaat is een fout. Een gzip-bestand merkt een afkapping wel.
- Gzip-fouten. Een gzip-bestand dat is afgekapt, een verkeerde CRC heeft of bytes na zijn laatste member heeft, faalt met `could not read file` en exit 2; er wordt geen rapport geprint.
- Gzip-overhead per member. Lege gzip-members pakken uit tot 0 bytes, dus de limiet van 536870912 uitgepakte bytes houdt ze niet tegen. Elke member mag hoogstens 1048576 gecomprimeerde bytes uitgeven boven de bytes die die member zelf uitpakt, en een bestand mag hoogstens 40000 members hebben. Uitgepakte bytes van een eerdere member verhogen geen van beide limieten. Voorbij een van die limieten print de tool `gzip overhead is larger than 1048576 bytes` en eindigt met 2, zonder rapport. De gecomprimeerde bytes van alle members van alle invoerbestanden mogen samen niet boven 1073741824 komen. Daarboven print de tool `gzip input is larger than 1073741824 bytes` en eindigt met 2, zonder rapport. Een `.csv.gz`-naam en een `.csv`-naam waarvan de eerste bytes de gzip-magic zijn, gebruiken dezelfde controles. Standaardinvoer wordt niet gelezen. Gemeten op de releasebinary, één member van 20971497 uitgepakte bytes (61192 gecomprimeerd) gevolgd door lege members van 20 bytes: 500000 members (10061192 bytes), 1000000 members (20061192 bytes) en 2000000 members (40061192 bytes) eindigden elk met 2 in 0.35 s (piek 2636 KB, 2688 KB en 2896 KB). Voor deze lege members van 20 bytes bleef de gemeten tijd 0.35 s toen de invoer 4× en nog eens 4× groeide. Een zwaarder bestand, één CUR-member gevolgd door stored members van 1048573 bytes (209711 lege stored blocks, net onder de overhead per member) en één kortere stored member, eindigde met 0 bij 268435456 bytes in 0.77 s (piek 2904 KB), bij 536870912 bytes in 1.52 s (piek 2624 KB) en bij 1073741824 bytes in 3.10 s (piek 2796 KB). De gemeten tijd groeide met 1.97 keer en met 4.03 keer. Een bestand van 1073741825 bytes eindigde met 2 met `gzip input is larger than 1073741824 bytes` en geen rapport in 3.12 s (piek 2848 KB).
- Standaarduitvoer. Kan het rapport niet worden geschreven (gesloten stdout, een volle schijf, `/dev/full`, een gesloten pipe), ook voor `--help` en `--version`, dan print de tool `could not write output` en eindigt met 2. Een stdout die al gesloten was voor het programma startte (`>&-`) wordt alleen op Linux herkend; op andere systemen niet. Die controle en de SIGINT-afhandeling zijn de enige `unsafe`-code van het project. Ze staan in `src/stdout_probe.rs` en `src/interrupt.rs`, en de bibliotheek houdt `#![forbid(unsafe_code)]`. Een stdout die alleen voor lezen is geopend (`1<file`) wordt niet herkend: op Linux eindigt de run met 0 en schrijft niets (op de andere systemen niet gemeten).
- Systeemeisen. De Linux-releasebinary die met glibc is gebouwd heeft GLIBC_2.34 of nieuwer nodig. Een bouw voor `x86_64-unknown-linux-musl` werkt en heeft geen glibc-eis. Een minimumversie van macOS of Windows is niet gemeten: de tests liepen alleen op Linux, en voor de twee andere systemen is de code alleen op types gecontroleerd en met clippy gelint (zie [QUALITY.nl.md](QUALITY.nl.md)).

## Onderhoud

Meld bugs via GitHub Issues: https://github.com/qwpp7g79y4-maker/cur-top/issues. Na een release staat de tool in sluimermodus. Een nieuwe versie komt alleen bij een gemelde bug, een aanvraag of een beveiligingsprobleem; beveiliging gaat altijd voor.

## Ondersteuning

Meld bugs en vragen via GitHub Issues: https://github.com/qwpp7g79y4-maker/cur-top/issues. Lees voor een beveiligingsprobleem eerst [SECURITY.md](SECURITY.md). Voeg de opdracht en een kleine synthetische CSV toe, nooit een echt kostenrapport.

## Licentie

MIT OR Apache-2.0. Zie LICENSE-MIT en LICENSE-APACHE.

De licenties van de externe crates in het programma staan in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).

## Contact

- E-mail: pep_ai@icloud.com
- Site: https://www.pepai.nl
- X: https://x.com/pepai_nl
- Ko-fi: https://ko-fi.com/pepai
- Git: https://github.com/qwpp7g79y4-maker/cur-top
