# PLANv3dot2 — styrk kjernen og fullfør kontraktene

Basert på gjennomgang av **HEAD `623c5d5`**, med pakkeversjon **3.0.1**,
5. september 2026. Dette er neste arbeidsplan, ikke en beslutning om å gi
ut `3.2.0`. Arbeidet bør leveres som kompatible rettelser og små tillegg i
3.x der det er mulig. Versjonsbump, tag og publisering besluttes separat.

Gjennomgangen gjelder innholdet i HEAD. De lokale, utrackede mappene
`examples/mr-egg-en/` og `examples/mr-egg-nb/` er ikke vurdert som leveranser.

## 1. Retning

> Gjør eksisterende patterns, språkpakker og artifacts pålitelige på alle
> støttede flater før Skald får flere funksjoner.

Kjerneproduktene er:

- Rust-craten og den native CLI-en.
- npm-pakken, WASM-motoren, npm-CLI-en og nettleserinngangen.
- De eksisterende en-US-, nb-NO- og nn-NO-dataene, med deres avgrensede dekning.
- `.skald`-kjøring, receipts og forklaring av faktiske runtime-valg.

Story Runner og Playground er referanseapplikasjoner. De skal demonstrere
og teste kjernen, men skal ikke drive fram en større VM, en ny plattform
eller betalt modellbruk som krav for å forbedre grunnproduktene.

PLANv3 var en bred utviklingsplan. PLANv4 velger bevisst færre leveranser:
samme kjøringssemantikk, riktig replay og verifiserbar distribusjon. Resten
av v3-listen blir ikke automatisk videreført som releasekrav.

## 2. Hva HEAD allerede har løst

Dette er eksisterende funksjoner som skal bevares, ikke nye v4-milepæler:

| Levert i 3.0.x | Grunnlag i HEAD |
| --- | --- |
| Kanoniske seeds, store heltall som tekst og navngitt run profile | [rng.rs](crates/skald/src/rng.rs), [lib.js](packages/skald-lang/lib.js) |
| Versjonerte språkpakker, nb/nn-kjerner, capabilities og innledende preflight | [dict/json.rs](crates/skald/src/dict/json.rs), [preflight.rs](crates/skald/src/preflight.rs), [locales](locales/README.md) |
| Artifact-format 2, SHA-256, dependency-/dictionary-hash og receipts per seed | [artifact.rs](crates/skald/src/artifact.rs), [artifact.mjs](packages/skald-lang/artifact.mjs) |
| Installerbar npm-pakkesmoke, språkpakkeeksporter og Playground-typekontroll | [smoke-packages.sh](scripts/smoke-packages.sh), [CI](.github/workflows/ci.yml) |
| Autoritativ engelsk vocab-kilde med driftstest og en begrenset engine-cache | [vocab_source.rs](crates/skald/tests/vocab_source.rs), [lib.js](packages/skald-lang/lib.js) |
| StoryState 2, eksplisitte patcher, variasjonsmetadata og sample-import | [runner.mjs](examples/story/runner.mjs), [eval.mjs](examples/story/corpus/eval.mjs) |

Norskpakker er små, originale og kuraterte ISC-data. De er ikke fullstendige
norske ordbøker eller en generell grammatikkmotor. Denne avgrensningen
beholdes.

## 3. Viktigste funn i HEAD

De eksisterende testene passerer lokalt, men dekker ikke alle kombinasjonene
som produktene tilbyr. Følgende funn begrunner prioriteringen:

| Funn | Observerbar konsekvens | Tiltak |
| --- | --- | --- |
| Rust `Options.locale` leses ikke av engine-kjøringen; dictionary og capabilities kan skilles fra språkpakken. | Direkte Rust-API håndhever ikke samme locale-kontrakt som npm-wrapper/CLI. | Samlet, validert språkprofil ved kjøring. |
| En nb-pakke med `capabilities: {}` får engelske defaults. Formlengde kan slippe gjennom hvis bare tabellens `subs` brukes. | `[a]katt` kan bli `a katt`; en manglende bestemt form kan bli lemma. | Fullfør eksisterende loader-validering. |
| Preflight følger ikke alltid faktisk carrier-/query-semantikk. | Recall med ukjent form godtas; binding i ett block-alternativ regnes som binding etter hele blocken. | Streng runtime-kontroll og presis, avgrenset preflight. |
| npm `--pack` returnerer før `--dict`-overlays lastes; native merger dem. | Norsk språkpakke + kaffe-palett fungerer i native, men gir ukjent tabell i npm. | Samme sammensetning og dictionary-hash på alle flater. |
| Manifest lagrer ikke valget av tom base fra `--dict-only`. | `manifest --dict-only --dict ...` lykkes, men etterfølgende `run` uten flaggene feiler med dictionary-hash-mismatch. | Lagre hele oppskriften for dictionary-lasting. |
| Artifact-run uten seed lagrer heller ingen effektiv seed. | Native receipt kan ikke gjenskapes ved `verify`; WASM kan tilfeldigvis gi samme resultat uten en fullstendig oppskrift. | Velg og lagre seed før artifact-kjøring. |
| Receipt-verifikasjon sammenligner pattern-hash og tekst, men validerer ikke format/run profile. | Endring til `formatVersion: 999` og feil `runProfile` blir fortsatt godkjent i begge CLI-er. | Streng receipt-validering og binding til kjøringsoppskriften. |
| Browser-fixturen kjøres i Node etter at `engine.js` har initialisert WASM. | Testen beviser ikke kald initialisering eller lasting av assets i en nettleser. | Én faktisk browser-smoke av installert pakke. |
| Top-level `Engine`-typer avviker fra engine-subpath; artifact-subpath mangler declarations. | Gyldig JavaScript får TypeScript-feil eller blir `any`. | Felles declarations og en liten forbrukertest. |

Kildepunkter: `run_ctx` i [engine.rs](crates/skald/src/engine.rs),
pakkevalideringen i [dict/json.rs](crates/skald/src/dict/json.rs),
`walk`/`check_query` i [preflight.rs](crates/skald/src/preflight.rs),
`loadDicts` i [npm-CLI](packages/skald-lang/cli.mjs),
artifact-kommandoene i [native CLI](crates/skald/src/bin/skald.rs),
`verifyReceipt`/`verify_receipt` i artifact-modulene og
[browser-fixture.mjs](scripts/browser-fixture.mjs).

## 4. Arbeidsområde A — samme korrekte kjøring overalt

### A1. Bind språkpakken som en helhet

Locale, capabilities og dictionary skal valideres sammen ved bruk av en
språkpakke. Gjenbruk eksisterende typer og legg til en trygg konstruksjonsvei
for Rust/WASM fremfor å kreve at hver caller bygger riktig kombinasjon av
løse `Options`-felter.

- Manglende pakke og locale-mismatch gir samme type feil i Rust, npm og CLI.
- Ufullstendige capabilities avvises eller får dokumenterte defaults fra
  pakkens locale. En norsk pakke skal aldri få engelske defaults i stillhet.
- Alle entries valideres mot effektive `subs`, også uten toppnivå `forms`.
  Påkrevde former må finnes og være gyldige.
- Språkpakke + overlays bruker samme deklarerte rekkefølge på alle flater.
  Raw overlays er eksplisitte tillegg under basepakkens profil; de er ikke
  en bakvei til å omgå formkrav eller bytte locale.
- npm skal enten anvende alle dokumenterte options eller avvise kombinasjonen.
  `languagePack` sammen med `dictionary` skal ikke stille ignorere dictionary.
- Legacy-kall uten språkpakke beholder dokumentert engelsk oppførsel.

Ferdig når samme pakke med én og to overlays gir samme tekst, kandidatvalg
og effektive dictionary-hash via Rust, WASM, npm, compiled API og begge
CLI-er. Inkluder både gyldig overlay, rekkefølgekonflikt og ugyldige former.

### A2. La runtime håndheve det preflight ikke kan bevise

En syntaktisk gjennomgang av AST-en er ikke nok til å garantere at en
carrier blir bundet på den valgte kjørebanen. Rettingen skal være liten:

- Recall validerer den faktisk bundne raden og etterspurt form.
- Ukjent form, også i flertallssyntaks, blir en feil i streng kjøring.
- Preflight skiller match-, unique- og rhyme-carriers og bruker de faktiske
  options, blant annet NSFW-valget, ved kandidatkontroll.
- Alternative blocks deler ikke ubetinget et globalt sett av «bundne» navn.
  Rapporter usikker binding når den ikke kan avgjøres enkelt.
- Pack-/artifact-kjøring avviser unresolved-events på den faktiske kjørebanen.
  Kontroll skjer via runtime-data, ikke søk etter `<` i sluttteksten.
- Gyldige funksjoner, skips, repeats og rimgrupper skal ikke avvises bare
  fordi preflight behandler dem som vanlig lineær tekst.

Behold en tydelig forskjell mellom statiske feil, mulige dynamiske feil
og faktiske runtime-feil. Det kreves ingen generell kontrollflytanalyse,
semantisk tekstforståelse eller ny tag.

Akseptansesettet skal inneholde disse eksisterende hullene:

- `<noun animal ::dyr> / <::dyr imaginary_form>` med nb-pakken.
- `{<noun animal ::dyr>|ingenting} / <::dyr>` over begge alternativer.
- Ukjent form i `<noun..imaginary_form>`.
- En ellers gyldig pakke med bare NSFW-kandidater og `nsfw: true`.
- En gyldig rhyme-gruppe med riktig uttaledata og capability.

## 5. Arbeidsområde B — en receipt skal kunne gjenskapes

Videreutvikle formatene som finnes. Ikke bygg et nytt arkivformat, register
eller generelt distribusjonssystem.

### B1. Bevar hele den effektive oppskriften

Manifest må beskrive hvordan kjøringsgrunnlaget rekonstrueres:

- Base: innebygd en-US, tom dictionary eller navngitt lokal språkpakke.
- Ordnet liste over overlays og hashes av alle nødvendige data.
- Locale, capabilities og semantiske options som faktisk brukes.
- Eventuelle uttalesidecars. En støttet runtime-avhengighet kan ikke være
  et udokumentert manuelt flagg ved replay.

Bygg render og dictionary-hash fra den samme effektive dictionary-en.
Unngå at en parallell JS-normalisering kan hashe andre data enn motoren
faktisk bruker. Foretrekk gjenbruk av kjernens serialisering der det er mulig.

Ved artifact-run uten seed velger hosten en seed én gang og lagrer den i
receiptet før den kan hevde replay. Rå pattern-kjøring kan fortsatt være
useedet. En annen seed lager fortsatt en separat receipt.

`run` skal ikke stille endre oppskriften med `--case`, `--nsfw`, språkpakke
eller andre semantiske overrides. Avvis dem i låst kjøring, eller krev en
eksplisitt ny oppskrift. Seed er den allerede definerte instansvariabelen.

### B2. Skill resultat fra CLI-presentasjon

Receipt lagrer strukturert hovedtekst og named channels, uavhengig av om
CLI viser ren tekst, `--channels` eller `--explain`. I HEAD kan hele CLI-
JSON-strengen havne i receiptets `text`-felt. Visningsvalg skal ikke gjøre
en tidligere receipt uverifiserbar eller endre hvilken fil som overskrives.

Receipt bindes til den eksakte oppskriften, for eksempel med en kanonisk
recipe-hash. Verifikasjon kontrollerer format, run profile, seed-type/verdi,
oppskrift, tekst og channels. Eventuell trace har sin egen avgrensede kontrakt;
full trace er ikke nødvendig for å bevise at tekst og channels gjenskapes.

Skill resultatene fra `verify` tydelig:

- Oppskrift og avhengigheter er validert.
- En konkret receipt er kjørt på nytt og samsvarer.
- Legacy-format kan leses, men mangler data for full verifikasjon.

Fravær av receipt skal ikke presenteres som utført replay. `verify` skal
heller ikke anvende nye options som gjør en endret receipt «riktig».
Hash-likhet er fortsatt integritet mot oppgitt grunnlag, ikke bevis på
forfatteropphav eller avsenders identitet.

### B3. Kompatibilitet og avgrensede tester

Ikke fjern eksakt runtime-versjonskontroll for å få gamle artifacts til å
passere. Lesing/`inspect` kan vise metadata fra eldre versjoner uten å love
at de kan kjøres. Import er eksplisitt og omskriver ikke originalene.

Hvis nødvendige nye felt endrer replay-kontrakten, versjoneres formatet.
Gamle artifacts uten tilstrekkelige data merkes som sådanne; verktøyet
gjetter ikke bort manglende `--dict-only`, seed eller kjøringsoptions.

Ferdig når én liten konformitetspakke kan produseres i native og leses/
verifiseres i npm, og omvendt. Den dekker tom base, norsk pakke + overlay,
store/text seeds, automatisk seed, channels, flytting av artifact-mappen
og manipulert format/profil/oppskrift. Ingen modellkall er nødvendig.

## 6. Arbeidsområde C — bevis at de publiserte pakkene virker

### C1. TypeScript og browser

- Del `Engine`-declarations mellom top-level og `skald-lang/engine`.
- Lever typer for det eksisterende `skald-lang/artifact`-subpathet.
  Dokumenter dette som Node-spesifikt; ikke flytt fil-I/O til nettleseren
  for å få kunstig symmetri mellom eksportene.
- Kjør en liten strict TypeScript-forbrukertest mot installert tarball,
  med de dokumenterte importene og compile-time/run-time options.
- Legg til én minimal app fra installert tarball og kjør den i en faktisk
  nettleser med kald WASM-initialisering. Ingen forutgående Node-init.
- Test top-level og engine-only, relative WASM-/JSON-assets og en-US/nb/nn.
  En lokal testserver og lokale assets er tilstrekkelig. Testen skal ikke
  kontakte modeller, CDN eller eksterne språkdatakilder.
- Verifiser faktisk støttet minste Node-versjon før `engines`-løftet
  videreføres. Ikke bygg en stor plattformmatrise uten et konkret behov.

Bruk samme få patterns og forventede resultater som i konformitetspakken.
Dette er verifikasjon av eksisterende produkter, ikke en ny Playground.

### C2. Måling som grunnlag for små forbedringer

Lag en reproduserbar før/etter-måling med **3.0.1** som nærmeste baseline:
compile-tid, varm `.run()`, `explain()`, pakke-lasting og WASM-minne under
en avgrenset gjentatt arbeidsmengde. Registrer commit, verktøyversjoner,
plattform og antall kjøringer. Skill JS-heap fra WASM-minne.

Dagens benchmark måler compiled `.run()`, ikke selve compile-kostnaden,
og JS-heapdelta er ikke en måling av WASM-minne. Pack-kjøringer parser også
på nytt i preflight. Mål dette før det velges en optimalisering.

Den eksisterende cachegrensen på 16 engines er levert. Undersøk faktisk
levetid/minne før eventuell endring av cache/free; en slettet cache-entry
er ikke alene bevis på minnelekkasje.

Behold build-grensen på **500 000 byte gzippet WASM**. En målt regresjon
skal vurderes, ikke skjules ved å øke grensen. Historiske 2.2/3.0-rapporter
beholdes som snapshots; manglende historiske minnetall er ikke en grunn
til å blokkere en konkret rettelse i dagens produkt.

## 7. Små forbedringer innen eksisterende språk og explain

Disse følger hovedrettelsene og skal holdes avgrenset:

1. **Før eksisterende entry-ID til output.** Språkpakkens `Entry.id`
   forsvinner i dag ved binding og finnes ikke i `QueryPick`. Bevar entry-ID
   og valgt form gjennom `BoundEntry` og relevante trace-data. Koble til
   den innlastede pakkerevisjonen. Ingen ny klassifisering av menneske-/
   modellopphav, og ingen påstand om identitet for legacy-data uten ID.
2. **Gjør norske eksempler pålitelige.** Dagens brede `åpnet <noun n definite>`
   kan gi «åpnet barnet» og «åpnet eplet»; testene godtar dette uttrykkelig.
   Bruk kuraterte kandidater som passer predikatet, eller komplette lukkede
   fraser. Ikke løs et dårlig eksempel ved å bygge en semantisk motor.
3. **Dokumenter faktisk språkdekning.** Behold en liten oversikt over
   tilgjengelige former og unsupported-operasjoner. Utvid bare paradigmer
   som et konkret produkttekst-, dialog- eller undervisningseksempel trenger.
   Alle alternativer i de små eksemplene kontrolleres, ikke bare antall seeds.

Kildegrunnlag: [dict/types.rs](crates/skald/src/dict/types.rs),
[output.rs](crates/skald/src/output.rs), [nb_no.rs](crates/skald/tests/nb_no.rs)
og [locales/README.md](locales/README.md).

## 8. Story Runner: vedlikehold, ikke nytt hovedspor

Core-release skal ikke avhenge av at hybrid slår llm-only. HEAD har vurderte
hybrid-fixtures, men ingen llm-only-samples og ingen lagrede manus i de 18
variasjonsradene. Det finnes derfor ikke en slik sammenlignende konklusjon.
Behold ærlig rapportering og valgfri sample-import; ikke bygg ny live-eval
eller review-arkitektur som del av denne planen.

Tre konkrete rettelser kan gjøres som en separat, liten vedlikeholds-PR:

- **Locale ved roundtrip:** `createStoryArtifact` bevarer ikke locale/
  språkpakken. Et lagret nb-artifact kan bli rendret med engelske navn, og
  initial `extractStoryState` kan gi en-US. Bevar kjøringsgrunnlaget og
  gjenbruk core-kontrakten der det passer. Dette er et hull i eksempelhosten,
  ikke et bevis på samme feil i core-artifact-modulen.
- **Patch-ID og innhold:** Samme patch-ID med endrede operasjoner behandles
  nå som allerede anvendt. Bind ID til normalisert payload-hash: identisk
  retry er no-op; annet innhold er konflikt. Ingen StoryState 3.
- **Variasjonstall:** Eval teller block-kombinasjoner, men omtaler tallet
  som en øvre grense for hele variasjonsrommet. Rapporter hva som faktisk
  telles; dictionary-/cast-valg gjør totalrommet ukjent i dagens beregning.

Stabile alternativ-ID-er for Story Runners sync og en full forfatterhistorikk
utsettes. Dokumenter dagens posisjonsbaserte begrensning, og avvis kjente
ugyldige grupper; ikke utvid core-syntaksen for å fullføre hele v3-ønskelisten.

## 9. Dette velges bort nå

| Forslag fra tidligere plan | Beslutning |
| --- | --- |
| Nye språk og store ordbank-importer | Utsettes. Gjør de tre eksisterende språkflatene korrekte først. |
| Generell grammatisk trekkbinding og produktiv sammensetning | Utsettes. Kuraterte former/fraser dekker de dokumenterte behovene. |
| Navngitte RNG-strømmer, delvis reroll og valglåser | Utsettes. Ingen ny RNG-semantikk uten et konkret brukerbehov. |
| Generelle typede runtime-inputs og begreps-ID-er på tvers av språk | Utsettes. Dette er nye API-er, ikke nødvendige rettelser i dagens kontrakt. |
| Artifact-register, signering, automatisk nedlasting eller plugin-system | Tas ikke inn. Lokale filer og eksplisitte avhengigheter er nok. |
| Ny editor, større Playground eller browser-artifactplattform | Tas ikke inn. Kun nødvendig verifikasjon og presis visning av eksisterende data. |
| Publisering av Story Runner som egen pakke | Utsettes. Ingen dokumentert grunn til å gjøre eksempelhosten til nytt produkt nå. |
| Nye tags, VM-omskriving, verdensmodell eller prose-rewrite | Tas ikke inn. Ingen av hovedfunnene krever dette. |

Ny funksjonalitet tas bare inn når en liten fixture demonstrerer et konkret
behov som dagens primitives eller host ikke kan løse rimelig. «Sto igjen i
PLANv3» er ikke tilstrekkelig begrunnelse.

## 10. Implementeringsrekkefølge og ferdigkriterium

| PR | Leveranse | Port |
| --- | --- | --- |
| A | Språkprofil/loader og pack + overlay-paritet. | Samme gyldige og ugyldige innganger behandles likt i native/npm/WASM. |
| B | Carrier-/formkontroll, korrekte preflight-options og streng runtime-feil. | Reproene i A2 er dekket; legacy-oppførsel har egne regresjoner. |
| C | Komplett artifact-oppskrift, seed, receipt-validering og strukturert resultat. | Kryssvis native/npm `manifest → run → verify`, også med endrede/manglende data. |
| D | Felles typer, installert forbrukertest og faktisk browser-smoke. | Kaldstart og dokumenterte imports virker fra den distribuerte pakken. |
| E | Entry-ID i explain, kuraterte eksempler, målinger og dokumentasjon. | Observerbare valg kan spores; eksemplene er riktige og endringene målt. |
| S | Avgrensede Story Runner-/rapportrettelser fra §8. | Offline roundtrip/patch-/rapporttester; ingen nye modellavhengigheter. |

D kan forberedes parallelt med A–C. S er eget vedlikehold og skal ikke vokse
til ny story-arkitektur. En tidlig feilrettingsrelease trenger ikke vente
på E eller på innsamling av redaksjonelle samples.

Alle obligatoriske kontroller skal faktisk kjøres. Hold de nye testene små:
minimal dictionary for query-logikk, få end-to-end-fixtures for CLI/pakking
og én nettleserapp. Behold eksisterende relevante seed-matriser; ikke
multipliser hele CI med nye seed-looper for hver kontrakttest. Ved høy
testkostnad reduseres oppstart/duplisering og målt total kjøretid, fremfor
å gjøre nødvendig dekning permanent skipped eller sjeldnere kjørt.

Planen er gjennomført når:

- Rust, npm, WASM og CLI har samme dokumenterte språk-/overlay-semantikk.
- Streng kjøring returnerer ingen uløste queries eller stille formfallback.
- En akseptert receipt har et komplett grunnlag og kan gjenskapes lokalt.
- `verify` skiller validert oppskrift fra faktisk utført replay.
- Installerte pakker, typer og kald nettleserinitialisering er prøvd i CI.
- Ingen nye modeller, tjenester, språk eller tags er nødvendig for resultatet.
- API-/formatendringer har migrasjonsbeskrivelse og korrekt versjonering;
  3.0.1-data blir ikke skrevet om eller erklært kompatible uten grunnlag.

## 11. Verifikasjon av utgangspunktet

Kjørt lokalt ved denne gjennomgangen:

- `cargo test --workspace --offline --quiet`: bestått.
- `node packages/skald-lang/test.mjs`: bestått, inkludert 14 native/WASM-goldens.
- `node examples/story/test.mjs`: bestått.
- Små separate reproduksjoner av pack + overlay, `--dict-only`-replay,
  seedløs receipt og ugyldig receipt-format/profil bekreftet hullene i §3.
- Avgrenset kode-/API-gjennomgang bekreftet runtime-, type- og browser-gapene.

Dette er kontroll av HEAD med tilgjengelige lokale build-artifacts, ikke
en ny full release-build eller bekreftelse av ekstern publiseringsstatus.
Denne oppgaven legger bare til planen; rettelsene er ikke implementert.
