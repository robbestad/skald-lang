# PLANv3 — kontrollerbar tekstvariasjon, språk og portable artifacts

Foreslått plan basert på PLAN.md, de to v3-utkastene og kodegjennomgang av
`b9ed0e4` den 4. september 2026. Dette dokumentet planlegger arbeidet; det
implementerer ingen av funksjonene nedenfor. Pakkeversjonene forblir **2.2.0**
til eksplisitt godkjenning av versjonsbump, tag og publisering.

## 1. Produktretning og prioritering

> Skald gjør et menneske- eller modellskrevet pattern om til seedede
> tekstvarianter med avgrenset språklig frihet, dokumentert opprinnelse og
> identisk replay når kjøringsgrunnlaget er låst.

Skalds hovedformål er kontrollerbar tekstvariasjon. Historier er en krevende
referanseapplikasjon, sammen med spilldialog, produkttekst, undervisning,
lokalisering, testdata og annen tekst som skal kunne realiseres lokalt.

| Lag | Ansvar |
| --- | --- |
| Forfatter eller modell | Mening, stil, syntaks, kausalitet og godkjente alternativer |
| Skald-runtime | Deterministiske valg, dictionary-oppslag, deklarerte former, carriers og synkronisering |
| Språkpakke | Ord, morfologiske trekk, formkontrakter, språkprofil og kildeinformasjon |
| Host | Inputvalidering, policy, artifact, state-overganger, I/O og eventuell modellintegrasjon |

**Anbefalt hovedmål:** Gjør Skald til en språkbevisst pattern-runtime med
portable artifacts og eksplisitt kontroll over hvilke deler av teksten som
kan variere. Lever bokmål først, deretter nynorsk med samme kvalitetskrav.

De to utkastene er enige om produktgrensen, men uenige om rekkefølgen.
Denne planen prioriterer runtime-, replay- og locale-kontraktene som
grunnmur. Arbeidet med en-US-rubrikk og ekte eval-data starter samtidig.
Betalte modellkjøringer skal ikke blokkere utvikling av locale-kontrakten;
en gjennomført redaksjonell eval er likevel et krav før 3.0-release.

Vannmerkeformuleringen skal være presis:

> Skald legger ikke inn modellvannmerker eller skjult metadata. Modellfrie
> patterns kan kjøres fullstendig lokalt. For modellskrevne patterns
> dokumenterer Skald oppgitt opprinnelse og faktiske runtime-valg, men
> hevder ikke å fjerne modellopprinnelse eller eksisterende vannmerker.

## 2. Hva prosjektet faktisk har i 2.2

| Område | Funn i koden | Konsekvens for v3 |
| --- | --- | --- |
| Runtime | `Program` holder AST/kilde; hver run får egen PCG32-kontekst. `Options` og `Dictionary` har ikke locale. Se [engine.rs](crates/skald/src/engine.rs) og [rng.rs](crates/skald/src/rng.rs). | Bevar den lille motoren; legg til eksplisitt kjøringskontrakt. |
| Dictionary/carriers | Entries og carriers har allerede former, klasser og uttale. Ingen generell resolver for grammatisk samsvar. Manglende form kan falle tilbake til første form. Se [types.rs](crates/skald/src/dict/types.rs) og [query.rs](crates/skald/src/query.rs). | Gjenbruk rader og former; norsk krever streng formvalidering og deklarerte relasjoner. |
| Språk | `[a]`, verbaliserte tall og deler av case-formattering er engelske. State, korpus og eval er en-US; request mangler locale. | Norsk er mer enn flere dictionary-filer. |
| Språkdata | `vocab/` og `crates/skald/vocab/` er separate kildetrær. Native build foretrekker sistnevnte, og npm eksporterer derfra. Se [build.rs](crates/skald/build.rs). | Velg én autoritativ kilde og en reproducerbar pakkeløype. |
| Artifacts | `.skald` er rå patterntekst. JSON-artifact lagrer palette-ID/hash, men ikke alle eksakte avhengigheter. `replay` rendrer med dagens registry uten å verifisere den gamle hashen. Se [host.mjs](examples/story/host.mjs). | Dagens eksport er nyttig, men ikke en portabel, låst replay-kontrakt. |
| Hash/seed | Replay-hashen er en kort JS-hash av `JSON.stringify`. Effektiv seed etter navneretry ligger i telemetry. npm-CLI konverterer numeriske seeds via `Number`. | Definer kanoniske bytes, seed-type, retry-data og integritet på tvers av språk. |
| Variasjon | Eksakte literal→pattern-endringer finnes. Identiske blocks synkroniseres automatisk etter tekstlikhet. `variationDiagnostics` beskytter required literals. Se `applySkaldTransform` og `syncRepeatedChoices` i [runner.mjs](examples/story/runner.mjs). | Legg til variasjons-ID, policy og eksplisitt synkronisering. |
| Provenance | Kjerne har picks, choices, emitted/channel og final-output parts. Den vet ikke hvem som skrev en literal eller et alternativ. | Skill forfatteropphav fra motorens observerbare valg. |
| StoryState | Extract unionerer åpne tråder. Lister har stille avkorting; nye cast-identiteter kan erstatte tidligere identitetslisten. | State 2 må ha tapsfrie, eksplisitte overganger og validerte grenser. |
| Eval | 14 briefs, seks med drafts. Mock hopper over de øvrige. «llm-only» er et regex-strippet draft; «grammar» måler bare forekomst av `<`. `--approve-expensive` avslutter med «live eval is not wired». Se [eval.mjs](examples/story/corpus/eval.mjs). | Ekte baseline, prøvegenerering/import og redaksjonell skåring må leveres. |
| Distribusjon | npm publiserer engine, CLI og engelsk dictionary. Story Runner ligger i `examples/story/`. | Generell artifact-støtte må være tilgjengelig i de publiserte produktene. |

Lokal baseline ved denne gjennomgangen:

- `cargo test --workspace --offline`: bestått.
- `node examples/story/test.mjs`: bestått.
- `node packages/skald-lang/test.mjs`: bestått, inkludert 14 native/WASM-goldens.

Dette bruker tilgjengelige lokale build-artifacts og er ikke en ny full
release-build. Testene bekrefter eksisterende oppførsel, ikke litterær kvalitet
eller v3-kontraktene. Et konkret udekket avvik er at seed-strengen
`9007199254740993` behandles ulikt av npm-API og npm-CLI.

## 3. Avgjørelser som skal tas med inn i implementasjonen

1. **Ingen modell ved runtime.** Også en menneskeskrevet `.skald`-fil skal
   kunne pakkes, valideres og kjøres uten Story Runner eller provider.
2. **Én locale per artifact.** `en-US` er kompatibilitetsdefault for gamle
   kall. Nye artifacts oppgir alltid locale. Bokmål og nynorsk er separate.
3. **Ingen stille språkfallback.** Manglende norsk ord/form erstattes ikke
   med engelsk. `fallbackLocale` utsettes; senere fallback må være eksplisitt,
   avgrenset til navngitte ressurser og synlig i receiptet.
4. **Sidecar fremfor ny pattern-header.** `tekst.skald` beholder ren syntaks;
   `tekst.skald.json` er versjonert manifest. Avhengigheter følger med lokalt.
5. **Én kanonisk state-patch.** CLI-flagg og modellens `closedThreads` er
   innganger til samme overgang, ikke parallelle state-mekanismer.
6. **Oppgitt semantikk er ikke bevist semantikk.** `role`, `preserves` og
   opphav fra en modell kan ikke alene godkjenne variasjonen.
7. **Ingen ubetinget «ingen nye tags»-lov.** Start med dagens primitives og
   host-materialisering. En eventuell ny primitiv krever et konkret generelt
   behov, for eksempel samsvar i både produkttekst og dialog. Den er ikke et
   forhåndsløfte i 3.0.
8. **Separat versjonering.** Pakke/runtime, kjøringsprofil, artifact-format,
   språkpakke, StoryDraft, StoryState og prompt har forskjellige versjoner.
   Dagens delte `SCHEMA_VERSION` skal ikke styre alle disse kontraktene.

## 4. Runtime- og locale-kontrakten

### 4.1 Locale skal gjøre en observerbar forskjell

Locale og språkprofil føres gjennom Rust, WASM-options, npm/TypeScript,
begge CLI-er, StoryRequest, StoryState, artifact, paletter, korpus og eval.
Compiled engine binder dictionary og språkprofil ved opprettelse; en per-run
option som ikke støttes, skal avvises fremfor å ignoreres.

Språkpakken deklarerer capabilities, slik at en runtime kan støtte locale
uten å late som den støtter alle språkoperasjoner:

| Operasjon | Krav i 3.0 |
| --- | --- |
| Dictionary/formoppslag | Valg fra riktig språkpakke; ukjent eller manglende påkrevd form er en strukturert feil. |
| Artikler og bestemthet | Norske former/fraser leveres eksplisitt. `[a]` beholder engelsk betydning og avvises i norsk strict-profil. |
| Grammatisk samsvar | Verifiser deklarert genus, tall og form i avgrensede variasjonsgrupper. Ikke inferer fri setningsgrammatikk. |
| Pronomen | Bundne former velges fra deklarert referentprofil; ikke fra et nytt tilfeldig pronomenoppslag. |
| Tall | Lås støttede tall- og flertallsregler. Norsk verbaliseringsstøtte må være implementert og testet før capability aktiveres; ellers eksplisitt unsupported-feil. |
| Case og tegnsetting | Norsk profil skal ikke arve engelske title-case-regler. Forfatterens tegnsetting og whitespace beholdes; ingen generell prose-rewrite. |
| Segmentering | Locale-spesifikke forkortelser og dialoggrenser i host; opprinnelige tekstslices beholdes. |
| Rim og Unicode | Eksplisitte krav til uttaledata og Unicode-regex-støtte. Språkstøtte betyr ikke automatisk rimstøtte. |

`dictionaryLocale` er språkpakkenes faktiske metadata, ikke en ekstra fri
request-verdi som kan motsi `locale`. Lokale navn og språkagnostiske
produktkoder kan deklareres som nøytrale ressurser. Det åpner ikke automatisk
for blanding av engelske og norske grammatiske tabeller.

Native/npm kan fortsatt tilby en-US som enkel standard. Norsk velger en
egen baseordbok før paletter legges over. `--dict-only` unngår engelsk
base, men er alene ikke et bevis på at de innlastede pakkene har samme locale.
WASM fortsetter uten innbakte språkdata.

Engine-cache må identifiseres av effektiv dictionary-hash, locale og
språkprofil, og ha en eksplisitt størrelsesgrense eller frigjøringsmekanisme.
Dagens wrapper-cache er en ubegrenset `Map`. Browser-løypa skal kunne bruke
lokale data eksplisitt; dagens top-level import henter en-US med `fetch`.
Offline-støtte må derfor demonstreres med lokalt tilgjengelige assets.

### 4.2 Streng språkpakke og validering før kjøring

Definer et versjonert språkpakkeformat med ID, locale, innholdsversjon,
formatversjon, capabilities, kilde/lisens, formdefinisjoner og dictionary-data.
Hver entry får stabil ID, og morfologiske trekk skilles fra frie temaklasser.

Bruk en streng loader for nye pakker. Dagens JSON-loader ignorerer ukjente
toppfelter, og `.dic`-kompilatoren ignorerer blant annet `#version` og
`weight`-metadata. Å legge metadata i eksisterende filer er derfor ikke
tilstrekkelig håndheving.

Valider minst ukjente felt/versjoner, dupliserte ID-er, tabellnavn,
formlengder, manglende former, locale-mismatch, alias-konflikter og overlay-
rekkefølge. Manglende former skal ikke utfylles med lemma i strict-profil.

Legg til en generell preflight som bruker parser/AST og faktisk dictionary:
ukjent tabell/form, ubundet carrier, tomt kandidatsett og manglende capability
skal gi stabile diagnostics med kildeposisjon. Dynamiske avhengigheter som
ikke kan bevises statisk, rapporteres som slike og kontrolleres ved kjøring.
Ingen garanti om at statisk analyse kan avgjøre alle kjøringer.

### 4.3 Seed og determinisme som publisert protokoll

Identisk replay krever identiske pattern-bytes, seed-type/verdi, inputdata,
kjøringsprofil, effektive dictionary-data, overlays og semantisk relevante
options. Samme seed alene er ikke et tilstrekkelig løfte.

- Definer kanonisk seed som enten `u64` kodet som desimalstreng eller
  tekstseed med eksplisitt type. Trygge JS-heltall kan være API-sukker.
  Avvis avrundede/utrygge tall, negative tall og brøker i heltallsvarianten.
- Dokumenter tolkning av `"42"`, `42`, tom streng og ledende nuller.
  CLI skal aldri gå via `Number` for store seed-verdier.
- Lås RNG-algoritme, seed-hashing, kandidatorden og synkroniseringssemantikk
  i en navngitt kjøringsprofil. Endring av dictionary kan endre output selv
  om RNG er uendret; innholdshashen er derfor del av kontrakten.
- Opprinnelig seed, effektiv seed og deterministisk retry-regel tilhører
  replay-data. De skal ikke bare ligge i volatil telemetry.
- Paritetstester dekker tekstseeds, Unicode, null, store heltall, grenseverdier,
  weights, carriers, channels og feiltilfeller i tillegg til vanlig prosa.

Samle options-kontrakten i én versjonert representasjon med tynne adaptere
for hver plattform. Dagens WASM-metoder har fortsatt posisjonelle argumenter.
TypeScript-deklarasjonene for top-level og `skald-lang/engine` må samsvare
med faktisk API; generer eller del deklarasjonene fremfor parallelle kopier.

## 5. Norsk: liten, korrekt språkflate før stort ordforråd

Lever en kuratert `nb-NO`-pakke først og bruk `nn-NO` til å kontrollere at
kontrakten faktisk er generell. Begge inngår i målet for 3.0, men implementeres
etter hverandre. En uferdig nynorskpakke skal ikke annonseres som støtte;
eventuell flytting til 3.1 krever en eksplisitt endring av release-scope.

### 5.1 Dekning og samsvar

- Substantiv: genus, entall/flertall, bestemt/ubestemt og relevante
  uregelmessigheter. Adjektivformer må følge deklarert ramme.
- Artikler: `en/ei/et` og `ein/ei/eit` håndteres sammen med substantivets
  valgte paradigme. Ikke velg artikkel uavhengig av substantivet.
- Pronomen: gjenbruk bound-entry-mekanismen og materialiser en host-eid
  `story_cast`-overlay med `name`, `nom`, `acc`, `poss` og `self` der
  dette er tilstrekkelig. Personlige pronomen deklareres; de gjettes ikke
  fra navn eller fra grammatisk genus.
- Possessiver/refleksiver: en flat `poss`-kolonne løser ikke alle tilfeller.
  `min/mi/mitt/mine` avhenger av det eide ordet; `sin/sitt/sine` trenger
  også syntaktisk binding. Hosten må få relasjonen eksplisitt, eller
  forfatteren må levere en komplett, lukket frase.
- Verb: lagrede former og kuraterte alternative predikater. Ingen fri
  ombytting som endrer argumentstruktur eller kollokasjon.
- Sammensetninger: kuraterte sammensatte ord og godkjente sammensetningsformer.
  Produktiv sammensetning og automatisk valg av bindebokstav utsettes.
- Valgfrie normerte former: språkpakken/profilen låser en konsistent praksis
  per dokument. Ikke tilfeldig veksling mellom formsett uten forfattervalg.

Det ønskede eksemplet `<noun animal :: pet>` etterfulgt av `<pron poss pet>`
er **ikke en eksisterende referanserelasjon**: `pet` blir ikke automatisk
forstått som pronomenets eier. Før syntaks fryses, demonstreres samsvar med
dagens carriers og en avledet dictionary-rad eller en komplett frase.
Hvis dette blir uhåndterlig, lages en avgrenset designbeslutning for generell
trekkbinding; ikke en story-spesifikk tag-familie.

### 5.2 Datakilder, pakking og språkfaglig kontroll

Norsk ordbank er en relevant kandidat til bøyningsgrunnlag: ressursene har
lemma, paradigmer og morfologiske trekk. Katalogen beskriver også former som
er mulige, men sjeldne i faktisk bruk. Bruk derfor et kuratert utvalg til
generering, med eksplisitte kollokasjoner og scene-paletter, fremfor å gjøre
hele fullformslisten til tilfeldige kandidater.
Kilder: [bokmål](https://www.nb.no/sprakbanken/ressurskatalog/oai-nb-no-sbr-5/)
og [nynorsk](https://www.nb.no/sprakbanken/ressurskatalog/oai-nb-no-sbr-41/).

Lås nedlastet snapshot, checksum, importskript, rettelser og attribusjon.
Katalogsidene oppgir CC BY, mens et separat
[vilkårsdokument](https://www.nb.no/sbfil/dok/norsk_ordbank.pdf) har andre
vilkår om videreformidling. Avklar og dokumenter vilkårene for det konkrete
snapshotet før avledede ordlister distribueres. Repoets ISC-lisens overføres
ikke automatisk til innhentede språkdata.

Velg én autoritativ kildekatalog; generer eventuelle crate-kopier og npm-data
fra den. CI kontrollerer innholdslikhet og deterministisk eksport.
Språkpakker lastes separat slik at flere språk ikke øker standard WASM-binary.

### 5.3 Språklige akseptansekrav

Egne nb/nn-korpuser, minst én scene-palett og én eksplisitt fortsettelse per
locale. Korpuset skal også ha produkt-/UI-tekst og en undervisningsoppgave,
slik at samsvar demonstreres uten Story Runner.

Kjør 100 faste offline-seeds per godkjent språkfixture. For små lukkede
alternativsett kontrolleres alle alternativer, fordi 100 seeds kan overse
sjeldne eller lavt vektede valg. Testene skal dekke genus, bestemthet, tall,
pronomen, sammensatte ord, tegnsetting, norske forkortelser og Unicode-spans.

Skillet mellom maskinell og språklig kontroll skal være tydelig: null
uløste queries og riktig formvalg er CI-krav; idiomatikk og stil vurderes
språkfaglig. Antallet grønne seeds er ikke et grammatikkbevis for fri prosa.

## 6. Portable `.skald`-artifacts og faktisk verifikasjon

### 6.1 Format og to forskjellige dokumenter

```text
tekst.skald             eksakte, redigerbare pattern-bytes
tekst.skald.json        manifest og kjøringsoppskrift
packs/                 eksakte lokale språkpakker/paletter ved behov
tekst.receipt.json     valgfritt resultat av én bestemt kjøring
```

Skillet mellom oppskrift og receipt er nødvendig: artifactet kan ha en
default-seed, mens `run --seed 42` lager en ny instans. Et nytt resultat
skal ikke omskrive hash eller historikk for et tidligere receipt.

Manifestet låser:

- Artifact-formatversjon, nøyaktig runtime-versjon og kjøringsprofil.
- Locale, språkprofil, språkpakkeversjoner, capabilities og ordered overlays.
- Pattern-hash, avhengighetshasher og hash av den effektive dictionary-en.
- Default-seed med type, semantiske options, inputkontrakt og runtime-budget.
- Variasjonspolicy og deklarert opphav der det finnes.

Alle avhengigheter er innebygd i pakken eller finnes lokalt med eksakt hash.
En ID som `standard-nb@1` eller et versjonsintervall er alene utilstrekkelig.
Runtime skal ikke laste ned manglende språkdata automatisk.

Receiptet legger til forespurt/effektiv seed, inputverdier, cast, output,
channels, diagnostics og observerte valg. Latency, tidsstempel og provider-
request-ID ligger utenfor den deterministiske delen. Story-manus og
redaksjonell historikk kan følge med som separat authoring-data; ren
runtime trenger ikke hele briefen eller provider-informasjonen.

### 6.2 Hash og kommandoer

Definer kanonisk serialisering og SHA-256 over UTF-8-bytes. Objektfelter
sorteres, semantisk array-rekkefølge beholdes, og tall/seed følger én
spesifikasjon. Pattern og output skal ikke normaliseres eller få påført en
ekstra newline under eksport. JSON-whitespace og tekst-whitespace må skilles.

Mål for publiserte native- og npm-CLI-er:

```bash
skald run tekst.skald --seed 42
skald inspect tekst.skald
skald verify tekst.skald
```

npm tilbyr samme underkommandoer gjennom `skald-lang`. Rust/npm/WASM får
tilsvarende manifest-/verifikasjons-API; filtilgang er fortsatt hostens ansvar.
Gamle argument-, stdin- og `-f`-kall bevares som rå pattern-modus.

- `inspect`: viser manifest, locale, avhengigheter, capabilities, deklarert
  variasjon og opphav uten å kjøre en modell.
- `verify`: sjekker format, eksakte avhengigheter, hashes, capabilities og
  preflight. Med et receipt kjører den også den lagrede instansen og
  sammenligner output/channels og den definerte deterministiske trace-delen.
- `run`: krever verifiserbart kjøringsgrunnlag for artifact-modus. Endret
  pattern eller feil pakke gir en tydelig feil; oppdatering av manifest
  er en eksplisitt authoring-operasjon.

En hash verifiserer innhold mot det oppgitte grunnlaget. Den beviser ikke
hvem som skrev teksten, at avsenderen er ekte, eller at meningen er korrekt.
Signering er en separat mulig utvidelse.

### 6.3 Kompatibilitet

Gamle 2.2-patterns og JSON-artifacts får en dokumentert importbane. Import
bevarer kilden og markerer ukjent opphav/manglende avhengigheter. Uten de
gamle eksakte ressursene kan verktøyet ikke hevde å ha verifisert historisk
replay. 2.2-runtime forblir veien til gammel semantikk der v3 har bevisste
endringer. Ingen automatisk overskriving av gamle artifacts.

## 7. Selektiv variasjon, synkronisering og provenance

### 7.1 Variasjon som data

Utvid dagens substitusjoner med en stabil `variationId`, kildereferanse,
rolle, tillatt omfang, bevaringskrav og review-status. Eksempel på
designretning, ikke et ferdig schema:

```json
{
  "variationId": "arrival-pace",
  "beatIndex": 2,
  "literal": "walked carefully",
  "pattern": "{walked carefully|moved cautiously}",
  "role": "micro_action",
  "policy": "bounded",
  "preserves": ["intent", "tone", "argument_structure"],
  "reviewStatus": "unreviewed",
  "syncGroup": null
}
```

Roller: `identity`, `plot_fact`, `motif`, `voice`, `micro_action`,
`surface_detail`, `decorative`. `unsafe` hører hjemme i vurderingen av
risiko, ikke som en type tekstinnhold. Policy skiller `locked` og `bounded`.
Hosten eier policyen; en modell kan ikke godkjenne sin egen endring ved å
sette `stability: safe`.

Substitusjoner valideres mot den opprinnelige teksten med span og forventet
literal, anvendes atomisk og kan ikke overlappe. Ukjent ID, endret grunnlag
eller endring av et låst område avvises. Source map følger originaltekst →
substitusjon → kompilert pattern → output; UTF-8-byteoffset er kontrakten,
med eksplisitt konvertering til JS/editor-posisjoner.

### 7.2 Synkronisering uttrykker identitet

Behold 2.2s tekstbaserte autosynkronisering i legacy-import, men bruk
eksplisitt `syncGroup` i nye artifacts. To forekomster av `{red|blue}`
kan være uavhengige; samme egenskap i forskjellige bøyningsformer kan
være koblet. Tekstlikhet er ikke en pålitelig identitetskontrakt.

Lukkede synkroniserte grupper får stabile alternativ-ID-er. Kompilatoren
kan ordne parallelle formsett etter samme ID og bruke eksisterende locked-
synkronisering. Manglende eller motstridende alternativer skal gi feil.
Dette gir kontrollert samsvar uten at VM-en tolker plot eller fri prosa.

### 7.3 Opphav og observerte valg er to akser

Forfatteropphav registreres per kildeområde/alternativ som `human`, `model`,
`mixed` eller `unknown`, med kilde-ID og revisjon der dette er tilgjengelig.
Runtime registrerer separat literal-utslipp, block-valg, dictionary-entry,
formvalg og host-generert struktur. Et modellskrevet alternativ som Skald
velger er både modellskrevet og runtime-valgt.

Koble kildeområder til eksisterende `picks`, `choices` og output-parts.
Transforms, captures, channels og ikke-emitterende uttrykk må beholde
denne forskjellen. Opphav skal aldri infereres fra glue-ratio eller
dictionary-density. Eldre materiale får `unknown` fremfor oppdiktet opphav.

## 8. StoryState 2: én eksplisitt, atomisk overgang

State er et avgrenset notat med identiteter, fakta, motiver og åpne tråder.
Modellen foreslår endringer; hosten validerer og anvender dem. Ingen
tilstand utledes automatisk fra ferdig prosa.

### 8.1 Operativ state og historikk holdes adskilt

Innfør en egen StoryState-formatversjon og en patch med `baseStateHash`,
patch-ID og eksplisitte operasjoner. Minimum: legge til fakta, åpne tråder
og lukke tråder. Endring/fjerning av låste fakta krever eksplisitt caller-
operasjon; et vanlig modellforslag kan ikke omskrive dem.

Bruk stabile tråd-ID-er i det nye formatet. `storyIntent.closedThreads`
og repeterbar `--closed-thread "..."` kan fortsatt ta tekst for enkel
authoring og 2.2-import: hosten mapper eksakt streng etter trim til én
åpen tråd. Ukjent eller tvetydig treff gir `STORY_SCHEMA`, ikke fuzzy match.

- Patch valideres mot riktig base og anvendes samlet etter godkjent
  sluttartifact. En mislykket compose/review/render endrer ingen state.
- To identiske innsendelser av samme patch skal ikke gi to overganger.
  En ny patch med gammel base avvises som konflikt.
- Lukkeoperasjonen fjerner tråden fra den operative listen. Audit-loggen
  registrerer hendelsen, men brukes aldri som nye lukkeinstruksjoner.
- En senere eksplisitt gjenåpning er mulig. Historisk `closedThreads`
  skal ikke lukke tråden på nytt i neste episode.
- Uten patch/intent videreføres eksisterende åpne tråder. Ny `endingSetup`
  normaliseres til en eksplisitt åpneoperasjon og lagres i overgangen.
- En tråd som åpnes og lukkes i samme overgang må ha en definert regel;
  v3 avviser motstridende operasjoner på samme ID fremfor å gjette rekkefølge.
- Lukking skaper ikke automatisk et nytt faktum. Det krever `addFacts`.

Eksisterende identiteter flettes etter ID og bevares også når de ikke
opptrer i neste episode. Rebinding, navneendring og ugyldig state avvises.
Ingen stille avkorting til åtte tråder eller 24 fakta: overskridelse gir
diagnostic, og caller kan lage en eksplisitt komprimering/arkivering.

### 8.2 Host, CLI og tester

`extractStoryState` produserer en validert etter-state fra innkommende state
og den godkjente overgangen. `applyStoryState` låser identiteter/literaler/
fakta og viser de gjenværende åpne trådene i prompten.

Behold `host.mjs state <artifact.json>`, legg til `--patch <patch.json>`
og repeterbar `--closed-thread`. Flaggene normaliseres til samme patch;
uventet blanding eller konflikt avvises. `loop --state` og en eksplisitt
JSON-request-inngang bruker samme validering. Dagens `loop <fil>` leser
filen som brieftekst, så envelope-støtte må implementeres og dokumenteres.

Offline-tester: grim-return/tower-voice med og uten lukking, minst tre
episoder, gjenåpning, ukjent tråd, duplikat-patch, feil basehash, ugyldig
state, størrelsesgrense, nye og fraværende identiteter og mislykket story-run.
`STORY_IDENTITY_DRIFT` og låste literaler beholder sin betydning.

## 9. Eval: tre forskjellige spørsmål

### 9.1 Er kjøringen korrekt?

Dette er CI: schema, policy, formvalg, carriers, låste literaler, ingen
uløste queries, full tekstdekning, state-overgang og replay. Behold dagens
10-seed story-matrise og legg til 100-seed-matrisen for språkfixtures.
Bruk runtime-events for unresolved queries; ikke søk etter `<` i output.

### 9.2 Hvor mye variasjon finnes faktisk?

Rapporter unike outputs, kollisjonsandel, valgfrekvenser og observerte
alternativer per `variationId`. Vis forskjellen mellom teoretisk
kombinasjonsrom og observert variasjon; synkronisering, weights og like
overflateformer gjør enkel multiplikasjon misvisende.

Syntaktisk variasjon og semantisk avstand kan være eval-observasjoner.
De skal ikke få en automatisk minstegrense: liten avstand er ofte målet
for trygg produkttekst. Embedding-/modellbaserte mål krever en separat,
versjonert eval og blir ikke runtime-avhengigheter.

### 9.3 Holder teksten redaksjonell kvalitet?

Gjør rubrikken konkret før nye samples genereres. For hver dimensjon
defineres eksempler på 0 = tydelig brudd, 1 = blandet/svak og 2 = oppfylt:
grammatikk/kollokasjon, kausalitet, referentklarhet, uønsket idérepetisjon,
form, sluttvirkning og stemme. Schema/reparasjonsutfall lagres separat
som maskinelle resultater. Manglende redaksjonell vurdering er `null`.

Lever ekte sample-import eller live-generering til eval-harnessen:

1. **Parvis manus→variant:** sammenlign et låst, godkjent manus med dets
   Skald-varianter. Dette isolerer tap som selve variasjonen introduserer.
2. **Hybrid mot reell llm-only:** samme brief, stilkontroller og dokumentert
   modelloppsett. Llm-only er faktisk prosa uten Skald-transformasjon,
   ikke en regex-strippet draft. Kall, tokenbruk og kostnad oppgis for
   begge løypene; ulikt budsjett må fremgå av konklusjonen.
3. **Negativ kontroll:** Mad Libs/åpne verbqueries der dette gir en relevant
   kontrast. Menneskeskrevet kontroll tas bare med når en slik tekst finnes.

Blindpakken inneholder brief/krav som vurdereren trenger, sample-ID og
tekst, men ikke condition eller fasit. Manifestet holder koblingen og
versjoner separat. Registrer også feil, avvisninger og utelatte briefs;
ikke rapporter bare vellykkede kjøringer. Eval-import må håndtere `stateFrom`
for fortsettelser, som dagens mock ikke gjør.

Før kjøring fryses sampleliste, seeds, rubrikk og vurderingsregel. Første
release-eval dekker alle 14 en-US-briefs og egne nb/nn-sett. Rapporter
parvise forskjeller på kausalitet, form, slutt og stemme, i tillegg til
enkeltfeil. Releasekravet er ingen uavklart systematisk regresjon i disse
dimensjonene og ingen kjente alvorlige brudd i de godkjente fixtures.
Et lite korpus gir en dokumentert redaksjonell vurdering, ikke et generelt
statistisk bevis på at hybrid alltid er like god som llm-only.

Live-generering krever eksplisitt `--approve-expensive`, endelige
kall-/token-/kostnadsgrenser og timeout/abort. Dagens stopp før neste
modellkall er ikke alene et hardt dollartak for det siste kallet.
Prisgrunnlag og eventuell estimert reservasjon må dokumenteres; der
kostnad ikke kan beregnes, skal det ikke loves et håndhevet dollartak.
Lagrede samples skal kunne vurderes helt offline.

Eval-funn styrer endringer i compose/review/skaldize og eksisterende
diagnostics som `STORY_WRITERLY_ASIDE` og `STORY_DEVELOPMENT_FLAT`.
Ingen ny review-arkitektur uten et påvist behov. Mr. Egg kan bli en fast
regresjonsserie når brief, referansetekst og opphav er lagt inn; prosjektet
har i dag tester med Mr. Egg, men ikke en komplett slik korpusserie.

## 10. Andre utvidelser som er verdt å prioritere

De viktigste tilleggene utover utkastene er direkte knyttet til hull i
dagens kontrakter. De første fem er tatt inn i 3.0-sporene over.

| Utvidelse | Praktisk verdi | Prioritet |
| --- | --- | --- |
| Streng preflight og capability-sjekk | Et ukjent bøyningsnavn blir en konkret feil før publisering, ikke et plausibelt feil ord. | 3.0 |
| Kanoniske seeds og komplett kjøringsoppskrift | Samme lagrede instans virker også utenfor maskinen som laget den. | 3.0 |
| Eksplisitte variasjons-/synkroniserings-ID-er | Skiller «samme egenskap» fra «tilfeldigvis samme blocktekst». | 3.0 |
| Én språkdatakilde med stabil entry-ID, kilde og pakkerevisjon | Hindrer drift mellom crate/npm og gjør et valgt ord etterprøvbart. | 3.0 |
| Atomiske state-patcher med basehash | Gjør fortsettelser, forgrening og retry pålitelige uten verdensmodell. | 3.0 |
| Variasjonsinspektør og diff | Vis original→pattern→to seeds, med årsak, formvalg og opphav per endring. Bygg videre på dagens Playground/parts. | Liten første versjon i 3.0; rik editor senere |
| Typede, eksterne inputverdier | Produktnavn, beløp og kundeopplysninger kan bindes som data uten å interpoleres som kjørbar Skald-syntaks. Inputskjema og escaping testes. | Design/prototype i 3.0; publisert kontrakt i 3.1 |
| Navngitte RNG-strømmer og delvis reroll | Bytt én detalj uten at alle senere valg flytter seg når patternet redigeres. Krever egne stabile valg-ID-er og versjonert semantikk. | Opt-in 3.1; ingen endring av legacy-RNG |
| Felles begreps-ID-er på tvers av locale | Samme vare/oppgave kan realiseres på nb/nn/en med språkspesifikke former. Krever koblede begreper, ikke bare lik seed. | 3.1 |
| Lokale valglåser / golden receipt | En redaktør kan beholde et godt alternativ og variere resten. Bruk eksplisitte valg-ID-er; låste valg må valideres mot ny pakke. | 3.1 |
| Checkpoints i authoring-pipen | Gjenoppta fra godkjent manus etter providerfeil; unngå ny compose og unødvendig kostnad. | Etter grunnkontraktene |
| Dictionary-diff og smale datapakker | Vis hvilke entries/former en oppgradering endrer; bygg mindre pakker når query-avhengighetene kan bestemmes. | Etter 3.0; dynamiske queries må håndteres eksplisitt |

Ikke gjør hele listen til releasekrav. Særlig navngitte RNG-strømmer,
generell inputbinding og krysspråklige begreper trenger egne designprøver.
Stabile ID-er og versjonerte manifests i 3.0 gjør disse utvidelsene mulige
uten å love dem ferdige nå.

## 11. Milepæler og PR-rekkefølge

Hver PR skal være offline-testbar og avgrenset. Ingen versjonsbump eller
publisering inngår i denne stacken. Store rader deles ved behov.

| PR | Leveranse | Avhengighet / ferdigkriterium |
| --- | --- | --- |
| A | Eval-protokoll, korpusinventar, reell sample-import, ærlige mock-felter og korrekt produkttekst. | Kan starte straks; blindpakke/manifest verifiseres offline. |
| B | Replay-/seed-spesifikasjon, separate formatversjoner, store seed-paritetstester. | Grunnlag for nye artifacts; ingen tvetydig seed-konvertering. |
| C | Locale/språkpakkeformat, capabilities, streng loader og én autoritativ datakilde. | Avhengig av B-kontraktene; nb/nn uten installert pakke gir «missing language pack», ikke gammel en-US-schemafeil. |
| D | Manifest, lokale dependencies, recipe/receipt, `run`/`inspect`/`verify` i distribuerte CLI/API-er. | B + C; replay lykkes med låste data og feiler ved manipulert/manglende grunnlag. |
| E | Variasjonsmetadata, eksplisitt sync, source maps og opphav per område. | B–D; legacy-import bevarer gammel autosync. |
| F | StoryState 2, patch/closedThreads, JSON-request-inngang og tre-episoders regresjoner. | Basehash fra B/D; kan utvikles parallelt med E og språkdata. |
| G | nb-NO-kjerne, bundne former/pronomen, palett, segmentering og fixtures utenfor story. | C + E; samsvar valideres og 100-seed QA passerer. |
| H | nn-NO med samme krav; Rust/npm/WASM- og reell browserparitet. | G brukes som mønster; egne språkfixtures og språkfaglig gjennomgang. |
| I | Live/importert redaksjonell eval, nødvendige promptrettelser og enkel variasjonsdiff. | A + ferdige språk/state-spor; rapporten lagres separat fra blindpakker. |
| J | Migrasjonsguide, docs, changelog, benchmarks og ren release-kandidatverifikasjon. | Alle 3.0-krav oppfylt; deretter separat releasebeslutning. |

Milepæler:

- **3.0-alpha:** B–D bevist med en-US og liten språkfixture; locale,
  capabilities og replay er reelle kontrakter. Eval-protokoll fra A finnes.
- **3.0-beta:** E–G og StoryState-overganger fungerer; bokmål har godkjent
  minimumsdekning. Norsk authoring kan prøves uten modell.
- **3.0-rc:** H–J ferdig, begge norske pakker vurdert, redaksjonell rapport
  lagret og distribuerte artifacts verifisert fra ren checkout.
- **3.0.0:** Kontrakter fryses og publisering skjer først etter eksplisitt ja.

## 12. Samlede releasekrav og avgrensninger

3.0 er ferdig når:

- Håndskrevne patterns kan kjøres med en-US/nb-NO/nn-NO og eksakte lokale
  pakker uten modell eller skjult nettverkstilgang.
- Locale-mismatch, manglende former og unsupported capabilities gir
  strukturerte feil. Norsk samsvar er demonstrert innen den deklarerte
  språkflaten, uten løfte om fri grammatisk forståelse.
- Samme recipe, inputverdier og seed gir byte-identisk output/channels på Rust, npm og
  WASM, også for store/text seeds og Unicode. Verifikasjon oppdager feil
  pattern, språkpakke, option eller receipt.
- Selektiv variasjon har eksplisitt policy og synkronisering. Opphav og
  runtime-valg kan inspiseres separat; ukjent opphav forblir ukjent.
- Fortsettelser bevarer identiteter og åpne tråder, lukker bare eksplisitt
  og kan fortsette gjennom minst tre episoder uten audit-/state-drift.
- en-US-, nb- og nn-eval er versjonert; systematiske kvalitetsregresjoner
  er håndtert. CI krever ingen modelltilgang eller subjektiv skåring.
- Ren build kjører Rust fmt/clippy/tests, WASM/npm, Story Runner,
  TypeScript-sjekk, Playground-build, browser-fixtures og package-smoke.
  Dagens Vite-build alene erstatter ikke en TypeScript-sjekk.
- Pakkesmoke installerer de produserte pakkene i et tomt prosjekt og prøver
  package exports, begge npm-importer, CLI, manifest og språkpakker. Direkte
  filimport av en utpakket npm-pakke er ikke tilstrekkelig alene.
- Compile-/run-tid, minne, trace-overhead, dictionary-lastetid og pakke-
  størrelser er målt mot 2.2. Dagens grense på 400 kB gzippet WASM beholdes
  med mindre en dokumentert beslutning endrer den; språkdata måles separat.

Dokumentasjonsarbeidet omfatter README, npm-README, kokebok, promptkort,
Story Runner-README, eval.md, migrasjonsguide, språkpakkenes kildefiler og
`CHANGELOG.md` med en unreleased 3.0-seksjon. Budskap, tabell-/formmanifest
og versjoner må ha autoritative kilder slik at kopier ikke driver fra
hverandre. PLAN.mds historiske 2.1/2.2-del beholdes som historikk; den aktive
delen peker til denne planen når implementasjonsarbeidet starter.

Følgende er fortsatt utenfor:

- Historiemodell, entity graph, lore-DB eller kausalitetsmotor i VM-en.
- Automatisk trådlukking fra prosa, generell essay-omskriving eller humanize-pass.
- AI-detektor, vannmerkefjerning eller løfte om modellfri forfatteropprinnelse.
- Full lexical coverage som default eller flest mulig ordbytter som kvalitetsmål.
- Automatisk oversettelse, produktiv sammensetningsmotor eller generell
  syntaktisk analyse som forutsetning for norsk.
- Modellkall i VM-en, provider-nøkler i Playground eller live LLM-seeds i CI.
- Nettverksregistry, signeringsinfrastruktur og en generell pluginplattform
  som krav for å åpne eller kjøre et artifact.
