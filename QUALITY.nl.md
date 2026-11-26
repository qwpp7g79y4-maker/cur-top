# Kwaliteit

pep: 8.6

Verslag van 2026-11-25. Geteste commit: `3cc300c` (`3cc300c8c302d468c265172468afbdfa60fc4701`), in een schone worktree van `origin/main`. Toolchain: rustc 1.99.0 en cargo 1.99.0 (stable). Machine: Linux 7.2.15. Elke controle hieronder draaide op 3cc300c; een controle die niet in de tabel staat is door deze meting niet uitgevoerd ("niet uitgevoerd"). Geen tag, geen push, geen publicatie.

Reeks: 168 geslaagd, 1 genegeerd, 0 mislukt (op 3cc300c; Linux, `cargo test --locked`, debug en release; elf van de tests draaien alleen op Linux).

## Opdrachten

| Opdracht | Exit |
| --- | ---: |
| `cargo fmt --check` | 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | 0 |
| `cargo test --locked` | 0 (168 geslaagd, 0 mislukt, 1 genegeerd) |
| `cargo test --release --locked` | 0 (168 geslaagd, 0 mislukt, 1 genegeerd) |

Gemeten op 2026-11-25 met Rust 1.85 (rustc 1.85.1, cargo 1.85.1), op de code van 3cc300c, vanuit een checkout op 1a21ec7 (die commit wijzigt alleen de verslagen): `cargo +1.85 fmt --check` exit 0; `cargo +1.85 clippy --all-targets --locked -- -D warnings` exit 0; `cargo +1.85 test --locked` exit 0 (168 geslaagd, 0 mislukt, 1 genegeerd, debug). `cargo +1.85 test --release --locked` exit 0 (168 geslaagd, 0 mislukt, 1 genegeerd, release, 2026-11-25). `cargo +1.85 build --release`: niet uitgevoerd.

Niet uitgevoerd op 3cc300c: de typecontrole en lint voor Windows en macOS, `cargo package`, `cargo build --release`, `cargo about`, `groff`, `scripts/check-glibc.sh`, `quality_benchmark_100_mb`, `cargo deny`, `cargo audit`. Hun uitkomsten in de secties hieronder komen uit eerdere verslagen (per sectie gedateerd) en zijn hier niet herhaald: niet uitgevoerd op 3cc300c.

## Testresultaat per doel

Debug (`cargo test --locked`) en release (`cargo test --release --locked`) geven dezelfde aantallen.

| Doel | Geslaagd | Mislukt | Genegeerd |
| --- | ---: | ---: | ---: |
| unittests `src/lib.rs` | 70 | 0 | 0 |
| unittests `src/main.rs` | 0 | 0 | 0 |
| `tests/cli.rs` | 54 | 0 | 0 |
| `tests/docs_clean.rs` | 4 | 0 | 0 |
| `tests/handbook_help.rs` | 2 | 0 | 0 |
| `tests/markdown.rs` | 5 | 0 | 0 |
| `tests/quality.rs` | 0 | 0 | 1 |
| `tests/output.rs` | 11 | 0 | 0 |
| `tests/readme.rs` | 12 | 0 | 0 |
| `tests/schema.rs` | 2 | 0 | 0 |
| `tests/sigint.rs` | 1 | 0 | 0 |
| `tests/verbosity.rs` | 6 | 0 | 0 |
| `tests/version.rs` | 1 | 0 | 0 |
| doc-tests | 0 | 0 | 0 |
| totaal | 168 | 0 | 1 |

De 168 is het aantal op Linux. Tien tests in `tests/cli.rs` en één in `tests/verbosity.rs` (`a_full_stdout_is_exit_2_without_a_context_line`) draaien alleen op Linux. Zeven meer zijn alleen voor Unix (een symlink in `tests/cli.rs` en in `src/run.rs`, een apparaatbestand, en `tests/sigint.rs`; ook `a_symlink_as_output_is_replaced_not_followed`, `a_fifo_as_output_is_refused_and_stays_a_fifo` en `a_unix_socket_as_output_is_refused_and_stays_a_socket`), dus op macOS en Windows draaien minder tests. `documented_test_total_matches_real_count` telt de `#[test]`-regels in de bronnen; hij draait ze niet.

De genegeerde test is `quality_benchmark_100_mb`.

## Niet uitgevoerd
- `quality_benchmark_100_mb` (`#[ignore]`): niet uitgevoerd. Opdracht: `cargo test --release --locked --test quality -- --ignored --exact quality_benchmark_100_mb`.
- `cargo +1.85` (minimaal ondersteunde versie): `fmt --check`, `clippy` en `test --locked` (debug) en `test --release --locked` draaiden op 2026-11-25 en slaagden (zie "Opdrachten"); releasebuild: niet uitgevoerd.
- Typecontrole, lint en tests voor Windows en macOS: niet uitgevoerd.
- `cargo package`, `cargo build --release`, `cargo about`, `groff`, `scripts/check-glibc.sh`, `cargo deny`, `cargo audit`: niet uitgevoerd.
- Een fuzz-campagne en een mutatierun: niet uitgevoerd.
- GitHub Actions en de release-workflow: door deze meting niet uitgevoerd.
- `git push`, `git tag` en `cargo publish`: niet gedaan.

## 100 MiB

`tests/quality.rs` draait `quality_benchmark_100_mb`. De test bouwt een CUR 2.0-CSV van minstens 100 MiB (`100 * 1024 * 1024` bytes in de bron) en telt die één keer op.

Gemeten op 2026-11-20 (Linux, AMD Ryzen 9 5950X, rustc 1.99.0), releaseprofiel al gebouwd, op de geteste commit: `quality_benchmark_100_mb` ok, 1 geslaagd, 0 mislukt, klaar in 1.04s, exit 0. Dit bestand heeft één sleutel, dus het zegt niets over geheugen bij veel sleutels; zie "Open punten".

## Standaarduitvoer met de hand

Gemeten op 2026-11-19 met de debug- en de releasebinary, voor `examples/cur.csv`, `--help` en `--version`. Alle drie gedragen zich gelijk, in beide profielen.

| Geval | Exit | Melding |
| --- | ---: | --- |
| `>&-` | 2 | `cur-top: could not write output` |
| `>/dev/full` | 2 | `cur-top: could not write output` |
| `>/dev/null` | 0 | geen |
| `1<>/dev/null` | 0 | geen |
| Python `subprocess.run(..., stdout=subprocess.DEVNULL)` | 0 | geen |
| `\| true` (gesloten pipe) | 2 | `cur-top: could not write output`, geen paniek |
| `1<file` (een alleen-lezen geopend bestand), releasebinary, rapport | 0 | geen, er is niets geschreven (zie "Open punten") |
| `/proc` vervangen door een leeg bestandssysteem, uitvoer naar een bestand | 0 | geen (615, 587, and 14 bytes geschreven) |
| `/proc` vervangen door een leeg bestandssysteem, `>&-` | 2 | `cur-top: could not write output` |
| `/proc` vervangen door een leeg bestandssysteem, `>/dev/null` | 0 | geen |

## Bekende grenzen

- Deze pagina beschrijft controles. Het is geen garantie en geen fuzzrapport.
- De meting van 100 MiB komt uit één run op 2026-11-20 (zie hierboven).
- Alleen een CUR 2.0-CSV of CSV-gzip wordt gelezen. Parquet en zip worden geweigerd, en Athena wordt niet gebruikt.
- De limiet voor productcode en gebruiksoort is 1024 bytes, geen tekens (`src/summary.rs`).
- De bedraglimiet van 40 tekens (`src/money.rs`) wordt getest op 40 en 41 tekens door `claim_c17_amount_limit_is_40_characters`.
- Een bestand met de naam `.csv` dat met de gzip-magic `1f 8b` begint wordt als gzip gelezen (`src/run.rs`); `claim_c14_gzip_magic_under_a_csv_name_is_read_as_gzip` bouwt dat geval.
- "Geen netwerkaanroep" wordt alleen via de afhankelijkhedenlijst gecontroleerd (`claim_c16_manifest_has_no_sdk_athena_or_parquet`). Geen test let op de systeemaanroepen.
- Windows en macOS: zie "Niet uitgevoerd". Een minimumversie van macOS of Windows is niet gemeten.
- De glibc-claim (GLIBC_2.34) is met `objdump -T` op één bouw gelezen en met `scripts/check-glibc.sh` gecontroleerd. De job `glibc` van de CI bouwt op `ubuntu-22.04` en draait hetzelfde script (zie "Niet uitgevoerd").

## Open punten

Bekende grenzen die niet zijn opgelost. Elk is gemeten met de releasebinary van 2026-11-19 op synthetische bestanden.

- Geheugen bij veel unieke sleutels. De toestand is een map van sleutels, dus het geheugen volgt het aantal verschillende sleutels, niet alleen de grootte van de invoer. Gemeten piek van het resident geheugen (releasebinary): `uniq300k.csv` (13.3 MB, 300000 rijen, elk met een eigen dienst en gebruiksoort) 314 MiB, 1.4 s; `tags20k.csv` (13.7 MB, 20000 rijen, elk 25 unieke tags) 303 MiB, 1.6 s; `uniq2m.csv` (91.8 MB, 2000000 unieke sleutels) 2.1 GiB, 14.2 s. Dat is ongeveer 24 keer de invoer voor unieke dienst- en gebruiksoortsleutels en ongeveer 23 keer voor unieke tags. De limiet van 536870912 bytes geldt voor de invoer, niet voor deze toestand. Extrapolatie, niet gedraaid: een bestand op die limiet met alleen unieke sleutels zou ongeveer 12 GiB nodig hebben. De benchmark van 100 MiB heeft één sleutel en laat dit niet zien.
- Gesloten standaarduitvoer alleen op Linux. `src/stdout_probe.rs` legt vast of bestandsdescriptor 1 bij de start gesloten was (`>&-`). Het wordt alleen op Linux gecompileerd; op andere systemen wordt een gesloten standaarduitvoer niet herkend. Een schrijfactie die faalt (`/dev/full`, een gesloten pipe) geeft op elk systeem exit 2. Die module en `src/interrupt.rs` (de SIGINT-afhandeling) bevatten de enige `unsafe`-code van het project (`#[allow(unsafe_code)]` op beide modules in `src/main.rs`); de bibliotheek houdt `#![forbid(unsafe_code)]`. Niet gedraaid op macOS of Windows.
- Systeemeisen. De releasebinary die met glibc is gebouwd heeft GLIBC_2.34 of nieuwer nodig (hoogste symbool, gelezen met `objdump -T`). Een bouw voor `x86_64-unknown-linux-musl` werkt en haalt de controles voor gesloten, volle en null-standaarduitvoer.
- Bestandsnamen. De controle op het achtervoegsel let op hoofdletters: `UPPER.CSV` en `export.CSV` falen met `not a csv or csv.gz` en exit 2. Een `.gz`-naam zonder de gzip-magic (`notgz.csv.gz` met de tekst `hello`) faalt met `not gzip` en exit 2. Beide zijn het gedocumenteerde gedrag.
- Een bestand dat tijdens het lezen wordt afgekapt. Met het bedrag als laatste kolom en een afkapping in het laatste veld van de laatste rij, zonder slotnieuweregel, is de som fout zonder foutmelding. Gemeten: twee rijen `1.50` en `2.50` geven `amount=4.00`; hetzelfde bestand afgekapt tot `...,2` geeft `amount=3.50` en exit 0; afgekapt tot `...,2.` geeft `invalid amount` en exit 2. Een CSV heeft geen controlesom. Dezelfde gegevens als gzip, afgekapt, geven `could not read file` en exit 2.
- Gzip-fouten worden niet onderscheiden. Een afgekapt gzip-bestand, een verkeerde CRC en bytes na de laatste member geven allemaal `file 1: could not read file` en exit 2, zonder rapport op standaarduitvoer. Gemeten op hetzelfde bestand met twee rijen.
- Gzip-overhead per member. Lege gzip-members pakken uit tot 0 bytes, dus de limiet van 536870912 uitgepakte bytes houdt ze niet tegen. Een gzip-bestand wordt geweigerd als de gecomprimeerde bytes van één member de bytes die die member uitpakt met meer dan 1048576 overschrijden, of als het bestand meer dan 40000 members heeft (`gzip overhead is larger than 1048576 bytes`, exit 2, geen rapport). De gecomprimeerde bytes van alle members van alle invoerbestanden mogen samen niet boven 1073741824 komen (`gzip input is larger than 1073741824 bytes`, exit 2, geen rapport). Uitgepakte bytes van een eerdere member verhogen de overhead per member of de limiet op het aantal members niet. Dezelfde controles gelden voor een `.csv.gz`-naam en voor een `.csv`-naam die met de gzip-magic begint. Standaardinvoer wordt niet gelezen, dus het is geen gzip-pad. Gemeten op de releasebinary op 2026-11-19, één member van 20971497 uitgepakte bytes (61192 gecomprimeerd) gevolgd door lege members van 20 bytes. 500000 members (10061192 bytes), 1000000 members (20061192 bytes) en 2000000 members (40061192 bytes) eindigden elk met 2 in 0.35 s, piek 2636 KB, 2688 KB en 2896 KB. Voor deze lege members van 20 bytes bleef de gemeten tijd 0.35 s toen de invoer 4× en nog eens 4× groeide. Eén CUR-member, daarna stored members van 1048573 bytes (209711 lege stored blocks) en één kortere stored member, eindigde met 0 bij 268435456 bytes in 0.77 s (piek 2904 KB), bij 536870912 bytes in 1.52 s (piek 2624 KB) en bij 1073741824 bytes in 3.10 s (piek 2796 KB). De gemeten tijd groeide met 1.97 keer en met 4.03 keer. Een bestand van 1073741825 bytes eindigde met 2 met `gzip input is larger than 1073741824 bytes` en geen rapport in 3.12 s (piek 2848 KB). Vóór de limieten per member en op het aantal members eindigde de releasebinary op diezelfde drie bestanden met members van 20 bytes met 0 in 1.45 s, 0 in 2.69 s en 2 in 2.92 s.
- Een enkele cel wordt geheel vastgehouden. Een bestand van 419430546 bytes met één productcode van 400 MiB werd geweigerd met `invalid line_item_product_code` en exit 2 na 1.0 s en een piek van 913 MiB. De celgrens is de bestandsgrens van 536870912 bytes.
- Een tolerante CSV-lezer. Een geciteerde cel gevolgd door tekst, zoals `"A"x`, wordt als `Ax` gelezen (exit 0). De productcode `Ax` wordt dan gemeld; geen bedrag verandert.
- Geen standaardinvoer. `-` faalt met `No such file or directory` en `/dev/stdin` faalt met `not a file` (pipe of terminal) of `not a csv or csv.gz` (gewoon bestand), beide exit 2.
- Een alleen-lezen geopende standaarduitvoer. `cur-top examples/cur.csv 1<ro.txt` geeft exit 0, schrijft niets en laat `ro.txt` ongewijzigd: de mislukte schrijfactie wordt niet gezien, net als bij `1</dev/null`. Alleen een standaarduitvoer die bij de start gesloten was wordt herkend. Gemeten op Linux; een besluit, niet opgelost.
- De releaseworkflow (`.github/workflows/release.yml`) bouwt de releasebinaries, voegt de licentiepagina toe en publiceert archieven zodra een `v*`-tag wordt gepusht. Of hij gedraaid heeft: zie "Niet uitgevoerd".
- `--json` toont sleutels zoals ze zijn. Een sleutel met een richtingwisselend of onzichtbaar teken (geen stuurteken: die worden bij het lezen geweigerd) staat ruw in `--json`; het tekstrapport escapet het. Dit is een besluit: JSON moet de sleutel bevatten zoals hij is geschreven.
- De tagregel (tekst en `--json`) toont `\` als `\\` in een sleutel en in een waarde, en `=` in een sleutel als `\=`, zodat twee verschillende tags nooit dezelfde regel geven. Een waarde met `=` wordt niet geëscaped, omdat de eerste niet-geëscapete `=` de sleutel beëindigt.

## Claim → test

Elke claim van `README.md` en `QUALITY.md`, met de test die hem bewijst. De testnamen komen uit de run van 2026-11-20. "Gat" betekent dat geen test hem bewijst, of maar een deel ervan.

| # | Claim | Test | Status |
| --- | --- | --- | --- |
| C1 | Telt de kostenkolom op zoals geschreven; tarief × gebruik wordt niet berekend | `claim_c1_unblended_is_not_rate_times_usage` | getest |
| C2 | CUR 2.0-kolomnamen, verplichte en optionele kolommen | `claim_c2_official_column_names_and_dictionary_date`, `missing_column_names_the_official_column`, `blended_column_is_summed_when_selected`, `net_unblended_column_is_summed_when_selected` | getest |
| C3 | `--amount` accepteert alleen de drie kostenkolommen | `claim_c3_amount_columns_are_the_three_cost_fields`, `invalid_amount_column_hides_the_name` | getest |
| C4 | De maand is `YYYY-MM`, de huidige is de laatste, de vorige is de kalendermaand ervoor | `claim_c4_latest_month_and_calendar_previous`, `claim_c4_latest_month_with_only_blank_amounts_stays_current`, `previous_month_rolls_the_year` | getest |
| C5 | Het verschil is huidig min vorig | `claim_c5_delta_is_current_minus_previous` | getest |
| C6 | Een leeg bedrag wordt overgeslagen; een slecht bedrag toont de cel niet | `claim_c6_blank_amount_is_skipped_without_the_cell`, `parquet_and_bad_amount_hide_the_payload` (cli) | getest |
| C7 | Andere maanden blijven buiten de totalen | `claim_c7_other_months_stay_out_of_the_totals` | getest |
| C8 | Geen vorige maand: `0.00` en `previous_month_present` is no | `claim_c8_absent_previous_month_is_zero` | getest |
| C9 | Een negatief bedrag wordt opgeteld zoals geschreven | `claim_c9_negative_credit_lowers_the_total`, `amount_accepts_a_negative_credit` | getest |
| C10 | Dezelfde rij in twee bestanden telt twee keer | `claim_c10_the_same_row_in_two_files_counts_twice` | getest |
| C11 | Elke tag krijgt het volle bedrag; escaping van sleutel en waarde; taglimieten | `claim_c11_each_tag_receives_the_full_line_amount`, `tag_values_that_differ_only_by_a_backslash_stay_apart_in_text` (cli), `keys_that_differ_only_by_a_backslash_stay_two_lines_in_text` (cli), `tags_whose_joined_text_is_equal_stay_two_lines`, `equal_joined_tags_stay_two_lines` (cli), `tag_limits_accept_the_edge_and_reject_one_more`, `too_many_tags`, `duplicate_key_rejected`, `control_char_rejected` | getest |
| C12 | Lege `resource_tags` tellen nog mee voor dienst en gebruiksoort | `claim_c12_empty_tags_still_count_for_the_service` | getest |
| C13 | De top `--top` sleutels, 1 tot 10000, sleutels met nul vallen weg, de totalen dekken alle rijen | `claim_c13_top_is_limited_and_totals_cover_both_months`, `top_accepts_10000_and_rejects_10001`, `tie_breaks_by_key` | getest |
| C14 | gzip aan de magic `1f 8b`, ook onder een `.csv`-naam; een `.gz`-naam zonder magic is `not gzip`; Parquet en zip geweigerd | `claim_c14_gzip_is_read_and_parquet_and_zip_are_rejected`, `two_gzip_members_are_both_counted`, `claim_c14_gzip_magic_under_a_csv_name_is_read_as_gzip` (cli) | getest |
| C15 | `line_item_usage_account_id` en extra kolommen worden genegeerd | `claim_c15_account_id_is_not_reported`, `account_id_column_is_absent_from_stdout` (cli) | getest |
| C16 | Geen netwerkaanroep en geen AWS SDK | `claim_c16_manifest_has_no_sdk_athena_or_parquet` | gat: alleen de afhankelijkhedenlijst, geen netwerktest |
| C17 | Bedraglimieten, ongeldige cellen geven exit 2 zonder de cel | `claim_c17_invalid_cells_hide_the_value`, `amount_with_more_than_28_decimals_exits_2_and_28_still_works` (cli), `amount_rejects_exponent_plus_and_blank`, `product_code_and_usage_type_stop_at_1024_bytes`, `usage_errors_exit_2` (cli), `claim_c17_amount_limit_is_40_characters` (cli) | getest |
| C18 | Decimale tekst, twee decimalen, geen exponent | `claim_c18_text_has_no_exponent_and_two_digits`, `format_amount_keeps_scale_and_two_digits` | getest |
| C19 | `--quiet` en `--verbose` veranderen alleen standaardfout: `--verbose` voegt bij succes één regel `context:` toe, `--quiet` geen, beide samen geven exit 2 | `verbose_adds_one_context_line_on_stderr_and_keeps_stdout`, `quiet_prints_nothing_extra_and_keeps_stdout`, `an_error_line_is_the_same_with_quiet_and_verbose_and_has_no_context`, `quiet_with_verbose_is_a_usage_error_exit_2`, `a_full_stdout_is_exit_2_without_a_context_line`, `help_lists_quiet_and_verbose` (`tests/verbosity.rs`) | getest |
| L1 | Installeren vanaf een lokaal pad | `install_uses_a_local_path` (`tests/readme.rs`) | getest |
| L2 | Het snelstartbestand is `examples/cur.csv` | `quickstart_heredoc_matches_the_sample_file` (`tests/readme.rs`) | getest |
| L3 | De tekst- en JSON-voorbeelden zijn wat de binary print | `console_examples_match_the_binary` (`tests/readme.rs`), `sample_text_matches_the_locked_report` | getest |
| L4 | Een symlink wordt gevolgd, een map wordt geweigerd | `directory_is_rejected_and_symlink_is_followed`, `symlink_is_followed_and_directory_is_rejected` (cli) | getest |
| L5 | Een bestand boven 536870912 uitgepakte bytes wordt geweigerd | `unpacked_size_limit_accepts_the_edge_and_rejects_one_byte_more` (cli) | getest |
| L6 | De tabellen zijn geen voorspelling | none | gat |
| L7 | De kwaliteitstest dekt een CSV van minstens 100 MiB | `quality_benchmark_100_mb` | test bestaat, genegeerd, niet in de reeks gedraaid |
| L8 | Niet gepubliceerd, geen git-tag | `licenses_changelog_and_cargo_metadata` (the changelog says so only) | gat: de toestand van de repository is niet getest |
| L9 | Het gedocumenteerde testtotaal (README, dit bestand) | `documented_test_total_matches_real_count` (counts `#[test]` lines, does not run them) | getest |
| L10 | Exitcode 0 goed, 2 invoer- of gebruiksfout | `usage_errors_exit_2`, `missing_file_exits_2`, `help_and_version_exit_0` (cli) | getest |
| L11 | De publieke documenten, de broncode, de workflows en de scripts bevatten geen interne verwijzingen | `public_documents_hold_no_internal_references`, `source_workflows_and_scripts_hold_no_internal_references`, `the_patterns_catch_each_kind_of_reference` (`tests/docs_clean.rs`) | getest |
| L12 | Een gebruiksfout toont een stuurteken of richtingteken van de argumenten als `\u{..}` op één regel, ook na een escapereeks en in een argument dat geen UTF-8 is | `usage_error_escapes_raw_direction_newline_and_cr`, `usage_error_with_escape_sequence_and_newline_stays_on_one_line`, `usage_error_with_a_lone_newline_argument_keeps_clap_line_breaks` (cli), `usage_error_with_a_non_utf8_argument_and_newline_stays_on_one_line` (cli, Linux) | getest |
| L13 | Een gesloten, volle of kapotte standaarduitvoer geeft exit 2 met `could not write output`; `/dev/null` en een null-standaarduitvoer geven exit 0 | `closed_full_and_null_stdout`, `help_and_version_to_closed_or_full_stdout_exit_2`, `stdio_null_stdout_exits_0_for_a_report_help_and_version` (cli) | deels getest: alleen Linux, en een gesloten pipe heeft geen test (met de hand gemeten, zie "Standaarduitvoer met de hand") |
| L14 | Invoerregels: UTF-8, BOM en CRLF, de vorm van het factuurtijdstip, ASCII-spatie, tab, CR en LF, een tolerante aanhalingsregel, een kleine-letter-achtervoegsel | `billing_timestamp_accepts_the_utc_form_and_rejects_the_others`, `bom_and_crlf_are_accepted_and_a_quote_followed_by_text_is_joined`, `a_file_name_with_an_upper_case_suffix_is_rejected`, `utf8_bom_is_accepted` | getest |
| L15 | Standaardinvoer wordt niet gelezen | `standard_input_is_not_read` (cli, Linux) | getest |
| L16 | Een alleen-lezen geopende standaarduitvoer geeft exit 0 en schrijft niets | `a_read_only_standard_output_exits_0_and_writes_nothing` (cli, Linux) | getest |
| L17 | Een CSV die in het laatste bedrag is afgekapt kan worden opgeteld; een afkapping na de punt is een fout | `a_csv_cut_inside_the_last_amount_is_summed_and_a_cut_after_the_point_is_an_error` (cli) | getest |
| L18 | Een beschadigd gzip-bestand faalt met `could not read file` en geen rapport | `damaged_gzip_files_fail_with_could_not_read_file_and_no_report` (cli) | getest |
| L19 | De releasebinary heeft GLIBC_2.34 of nieuwer nodig | none in the suite; `scripts/check-glibc.sh` in the CI job `glibc` | gat: het script draaide hier, de job heeft niet gedraaid |
| L20 | Het geheugen is ongeveer 24 keer de invoer bij veel unieke sleutels | none | gat: met de hand gemeten (drie bestanden, zie "Open punten") |
| L21 | Een enkele cel wordt geheel in het geheugen gehouden | none | gat: met de hand gemeten (één bestand) |
| L22 | Een gzip-bestand wordt geweigerd bij meer dan 40000 members, als de gecomprimeerde bytes van één member de uitgepakte bytes van die member met meer dan 1048576 overschrijden, of als de gecomprimeerde bytes van alle members van alle invoerbestanden meer dan 1073741824 zijn (`gzip input is larger than 1073741824 bytes`, exit 2, geen rapport). Voor lege members van 20 bytes eindigden 500000, 1000000 en 2000000 members na één member van 20971497 uitgepakte bytes elk met 2 in 0.35 s (piek 2636 KB, 2688 KB en 2896 KB); die 0.35 s bleef gelijk toen de invoer 4× en nog eens 4× groeide. Stored members van 1048573 bytes (209711 lege stored blocks) na één CUR-member, plus één kortere stored member, eindigden met 0 bij 268435456, 536870912 en 1073741824 bytes in 0.77 s, 1.52 s en 3.10 s (piek 2904 KB, 2624 KB en 2796 KB); de gemeten tijd groeide met 1.97 keer en met 4.03 keer. 1073741825 bytes eindigde met 2 in 3.12 s (piek 2848 KB) zonder rapport | `many_empty_gzip_members_are_rejected_on_every_gzip_path`, `gzip_overhead_accepts_the_budget_and_rejects_one_more_member`, `a_large_prefix_plus_too_many_empty_gzip_members_is_rejected`, `one_member_cannot_use_an_earlier_members_unpacked_bytes`, `many_gzip_members_that_hold_rows_are_still_read`, `gzip_compressed_total_is_rejected_on_every_gzip_path`, `a_large_ordinary_gzip_under_the_compressed_cap_still_runs` (cli) | getest |
| L23 | `--md` print dezelfde bedragen als het tekst- en JSON-rapport; een pipe, verborgen tekens en Markdown- en HTML-tekens in een sleutel worden geëscaped; `--md` met `--json` is een gebruiksfout | `markdown_amounts_equal_text_and_json`, `a_pipe_and_a_line_break_in_a_key_stay_in_one_cell`, `markdown_and_html_in_a_key_are_escaped`, `md_with_json_is_a_usage_error`, `markdown_says_the_amounts_are_read_not_estimated` (`tests/markdown.rs`) | getest |
| L24 | SIGINT tijdens het lezen van de bestanden geeft exit 130, zonder rapport, met lege standaarduitvoer en één regel op standaardfout (Unix) | `sigint_while_reading_exits_130_with_no_report_and_no_panic` (`tests/sigint.rs`, a real SIGINT) | getest |
| L25 | `--output FILE` schrijft het hele rapport via een tijdelijk bestand en een hernoeming; een bestaand bestand wordt vervangen, een symbolische link wordt niet gevolgd, een ontbrekende map wordt niet aangemaakt, een invoerbestand wordt geweigerd, een schrijffout is exit 2 met een geëscapet pad | de elf tests van `tests/output.rs`; een bestaande pijp of socket wordt geweigerd met `not a regular file` | getest |
| L26 | Elk JSON-rapport begint met `schema_version` 1; een mislukte run print geen JSON | `every_json_path_has_schema_version_1_first`, `a_failing_json_run_prints_no_json_only_one_error_line` (`tests/schema.rs`) | getest |
| L27 | `--version` print de crateversie; een debugbouw voegt de git-commit toe als git die geeft | `version_prints_the_crate_version_and_in_debug_the_commit` (`tests/version.rs`) | getest |
| L28 | `about.toml` accepteert dezelfde licenties als `deny.toml`, en `about.hbs` bouwt de licentiepagina van derden voor het releasearchief | `about_toml_accepts_the_same_licenses_as_deny_toml` (`tests/docs_clean.rs`); the page itself was generated by hand with `cargo about` (4 license groups), not in CI | deels getest |
| L29 | `README.nl.md` heeft dezelfde secties, consolevoorbeelden, claims en getallen als `README.md` | `the_dutch_readme_has_the_same_sections_examples_claims_and_numbers`, `readme_sections_follow_the_documentation_standard`, `the_options_section_names_every_long_option_of_help` (`tests/readme.rs`) | getest |
| L30 | De bestanden `examples/cur.expected.*` zijn wat de binary print voor `examples/cur.csv` | `the_example_files_match_the_binary` (`tests/readme.rs`) | getest |
| Q1 | De meting van 100 MiB van 2026-11-20 | `quality_benchmark_100_mb` | gat: genegeerd, bewust gedraaid, geen deel van de reeks |

50 claims: 40 getest, 2 deels getest (L13, waar een gesloten pipe geen test heeft, en L28, waarvan de licentiepagina met de hand wordt gemaakt), 1 met alleen een genegeerde test (L7), 7 zonder test die ze bewijst (C16, met alleen de afhankelijkhedenlijst, L6, L8, L19, L20, L21, Q1). `quality_claim_summary_matches_the_table_rows` telt de rijen.

De man-pagina wordt gecontroleerd door `help_long_options_match_the_man_page_and_unknown_option_is_not_a_panic` (`tests/handbook_help.rs`): hij bevat de `--help`-tekst letterlijk en dezelfde lange opties.
