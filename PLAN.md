# Plan: historier med Skald og LLM

## Retning

Skald skal ikke konkurrere med en LLM om å skrive plot eller prosa. LLM-en gjør det
semantiske arbeidet: plot, årsak, tempo, verb og kollokasjon. Skald gjør den
kontrollerbare delen: seedede navn, stabile referenter, sceneord og små lukkede
`{a|b|c}`-valg. Hosten binder delene sammen, validerer kontrakten og kan sende
strukturerte feil tilbake til LLM-en.

Det viktigste produktløftet for historier er derfor ikke at hele sluttteksten er
«ikke skrevet av en modell». En god historie vil med vilje bestå av mye LLM-skrevet
rammetekst. Løftet er:

> LLM-en skriver rammen; Skald instansierer referenter og avgrensede valg
> deterministisk og etterprøvbart.

Skald er særlig nyttig når man trenger familier av historier, rerolls, stabile
karakterer, replay med samme seed, avgrensede sceneord, tester eller provenance. For
én engangshistorie uten slike krav er en LLM alene enklere.

## Fast scope-grense

Følgende skal ikke inn i VM-en:

- `[plot]` eller andre story-spesifikke tags
- persistent verdensmodell, entity graph eller lore-database
- kausalitets-, dialog- eller dramaturgimotor
- ubegrensede host-loops eller modellkall
- provider-spesifikk LLM-kode
- automatisk omskriving av ferdig Skald-output

Nye storybehov skal først løses i schema, prompt, palett, statisk analyse eller host.
En ny VM-primitiv krever en selvstendig, generell begrunnelse utenfor story-pipen.

## Arbeidsdeling

| Lag | Ansvar | Skal ikke eie |
| --- | --- | --- |
| LLM | Plot, årsak, tempo, syntaks, verb, kollokasjon og komplette grammatiske rammer | Seed, tilfeldige navn, filstier eller runtime-policy |
| Skald | Seedede dictionary-valg, carriers, former, små blocks og receipt-data | Plot, verdensstate eller prose repair |
| Host | Schema, seed, cast, paletter, lint-policy, rendering, retry-grense og artifact | Kreativ omskriving eller nye språksemantikker |

## Status

2.0.0 er shipped. 2.1-grunnmur, Story Runner, mock-loop, playground Story JSON, CI og package-smoke er implementert på `main` men **ikke tagget/publisert**. Publisering venter på eksplisitt 2.1-beslutning.

Den opprinnelige historiekvalitet-planen er levert som 2.0.0 MVP:

| Punkt | Status | Merknad |
| --- | --- | --- |
| `--story` lint-notater | Ferdig | Statisk AST-lint, men diagnostics og beat-indekser må forbedres |
| Dictionary-overlay | Ferdig | Rust- og npm-API støtter overlay; CLI/story-host gjør det ikke |
| `examples/story/` | Ferdig | Schema, prompt, host, inn/grim og goldens finnes |
| Playground og kokebok | Ferdig | Story-notater vises; scene-overlay og Story JSON mangler |

Kjernearkitekturen er riktig for målet: `Program` lagrer AST og kilde, hver run får
en ny seedet runtime-kontekst, og carriers lever innen én samlet run. Ingen plot- eller
verdensmodell finnes i VM-en.

Verifisert baseline 4. september 2026:

- `cargo test --workspace` passerer.
- `node packages/skald-lang/test.mjs` passerer 14 native/WASM-caser.
- Begge eksisterende story-fixtures matcher sine committed goldens byte for byte.
- Playground bygger med Vite.
- Baseline har ingen CI-workflow som håndhever dette ved senere endringer.

## Målarkitektur

```text
Untrusted brief + trusted controls: seed + paletteIds + run-policy
                         |
                         v
                  LLM -> StoryDraft JSON
                         |
                         v
        schema -> story-policy -> carrier/query checks
                         |
              feil ------+------> LLM-revisjon, maks 2
                         |
                         v
              host bygger cast-prelude + overlays
                         |
                         v
                  én seedet Skald-run
                         |
                         v
             StoryArtifact: tekst + cast + receipt
```

Tre dokumenttyper holdes adskilt:

1. **StoryRequest** er hostens envelope. `narrativeBrief` er brukerens premiss,
   formkrav, inversjon, protagonistisk svakhet og slutt. Det er kreativt bindende:
   modellen skal realisere dette i beat-overflaten, ikke oppsummere det. Samtidig er
   feltet ubetrodd operasjonelt input, ikke en Skald-seed. Numerisk `seed`, tillatte
   paletter og kjørepolicy er separate, betrodde kontrollfelter. Briefen avgrenses tydelig i
   modellprompten og får aldri overstyre schema eller policy.
   `deviation` og `expansion` er separate skalaer fra 0 til 100. Deviation styrer
   tillatt narrativ avstand fra briefens hendelsesforløp. Expansion angir graden av
   meningsfull videreutvikling, ikke en fast ordlengde; ordmåling brukes bare som et
   interpolert sikkerhetstak. Fri tekst i `theme` styrer tematisk og tonal behandling,
   for eksempel humoristisk eller alvorlig. Alle tre låses gjennom reparasjon og
   lagres i replay-payloaden.
2. **StoryDraft** er ubetrodd LLM-output: schema-versjon, cast-intensjon og beats.
   Cast kan være tomt. Eksplisitte egennavn og navngitte ikke-menneskelige figurer i
   briefen er kanoniske literaler; bare navnløse roller som skal få et generert navn
   legges i cast.
3. **StoryArtifact** har en kanonisk replay-payload: draft, seed, løst cast, mønster,
   tekst, diagnostics, versjoner, hashes og trace. Volatil modelltelemetri kan følge
   med i et separat felt, men inngår ikke i replay-hashen.

LLM-en skal ikke velge seed. Dersom caller ikke sender seed, lager hosten en seed og
skriver den inn i artifactet før rendering.

Den ferdige prosastilen skrives inn i beat-rammene av LLM-en. Rendering skal bare
binde cast og velge små, grammatisk lukkede alternativer; det finnes ingen senere
«humanize»-pass som kan bryte replay eller provenance. Join av renderede beats er
sluttteksten.

Formkrav i `narrativeBrief` gjelder selve beat-overflaten. Ber briefen om bilag,
arbeidspapirer, brev eller vitneforklaring, skal hvert relevant beat være en faktisk
linje i dette artefaktet, ikke ordinær fortellerprosa som omtaler formen utenfra.
Prompten krever dette eksplisitt. Før semantisk evaluering finnes, kan den
deterministiske validatoren bare håndheve schema og Skald-sikkerhet; den kan ikke
bevise litterær brief-troskap.

## Godkjente hull som planen skal lukke

### Story-kontrakt og host

- `story.schema.json` er dokumentasjon, men håndheves ikke av hosten.
- `cast.query` valideres overflatisk og brukes ikke til å bygge eller kontrollere
  mønsteret.
- En ubundet `<::hero>` blir tom tekst og kan passere som vellykket historie.
- En ukjent query blir stående som `<raw>` og kan passere.
- Seed er valgfri i dagens schema.
- Scene-overlay finnes, men kan ikke velges av story-hosten.
- Hosten gjør ett lokalt render-kall; den har ikke et provider-nøytralt LLM-grensesnitt
  eller en avgrenset reparasjonsloop.

### Lint og provenance

- Story-notater er fritekst i stedet for stabile, maskinlesbare diagnostics.
- Beat-splitteren teller både setningspunktum og påfølgende newline og kan rapportere
  JSON-beats som 1, 3, 5 og så videre.
- Dokumentert forbud mot blant annet `<verb-walk>` håndheves ikke konsekvent.
- Dagens verb+noun-heuristikk er grov og forstår ikke scene-policy.
- Små `{a|b|c}`-blocks har ingen story-profil for antall eller størrelse.
- Provenance går tapt gjennom captures og blocks. En block med både glue og en query
  kan feilaktig bli rapportert som 100 prosent glue eller 100 prosent dictionary.
- Valgt block-alternativ spores ikke med source span og alternativindeks.

### CLI, WASM og API-paritet

- Native CLI har ikke `--dict`; scene-JSON krever npm- eller Rust-API.
- Rust-CLI setter alltid `dictionary: None` og `merge: false`.
- WASM-bindingen eksponerer ikke konfigurerbart budget som run-option. Standardbudsjett
  håndheves, men caller kan ikke overstyre det.
- WASM håndterer story-lint og merge via JS-wrapperen i stedet for samme options-kontrakt
  som Rust.
- Rust og JS har ulik default for dictionary merge.
- `compile(...).run({ dictionary })` ser støttet ut i TypeScript-typene, men compiled
  WASM er bundet til engine/dictionary ved compile.
- Både native og npm `--story` returnerer exit 2 for argument/fil, men exit 0 for
  pipet stdin.
- Ingen av REPL-handlerne kan toggle story-modus. npm-hjelpen annonserer likevel
  `:story`; native-hjelpen gjør det ikke.

### Produkt og release-hygiene

- Repoet har ingen CI-workflow.
- Story-goldens finnes, men kjøres ikke automatisk.
- Playground kan vise story-lint, men kan ikke laste scene-palett eller Story JSON.
- Crate- og npm-pakker er forberedt, men package-innhold og installasjon er ikke en
  automatisert release-gate.
- Det finnes ingen etablert GitHub Release-flyt. Dette er valgfritt med mindre
  prosjektet skal distribuere binærartefakter via GitHub.

## Milepæl 0: 2.1-grunnmur

Dette arbeidet gjøres før Story Runner-kontrakten stabiliseres, men de additive
CLI-/WASM-API-ene og `choices`-feltet publiseres samlet som en minor-versjon. En
eventuell 2.0.1-utgivelse skal begrenses til kompatible bugfikser, CI, tester og
dokumentasjon. Ingen av delene legger til språkfunksjoner.

### 0.1 CI først

Opprett `.github/workflows/ci.yml` med separate, cachebare jobber for:

1. Rust-format/lint og `cargo test --workspace`.
2. WASM/npm-build via `scripts/build-npm.sh`.
3. `node packages/skald-lang/test.mjs`.
4. Ende-til-ende story-goldens og negative story-fixtures.
5. `npm run build --prefix playground`.
6. Package-smoke for crate og npm-tarball.

CI skal installere eksplisitt Rust-versjon, `wasm32-unknown-unknown`, `wasm-pack` og
støttet Node-versjon. Genererte npm/WASM-filer skal ikke behøve å bli committet.

### 0.2 Native CLI-overlay

Legg til følgende kontrakt:

- `--dict <path>` laster dictionary-JSON og merger over innebygd en-US som standard.
- Flagget kan gjentas; overlays anvendes deterministisk fra venstre mot høyre.
- `--dict-only` starter uten en-US og bruker bare oppgitte dictionaries.
- Ugyldig JSON, manglende fil og inkompatible tabeller er exit 1 med tydelig feil.
- `--story` beholder exit 2 for rene story-policy-feil.

En eksplisitt CLI-filbane er betrodd brukerinput. LLM-generert StoryDraft får aldri
angi filbaner; der brukes bare allowlistede `paletteIds`.

### 0.3 WASM- og wrapper-paritet

- Innfør en versjonert options-object-kontrakt for seed, case, NSFW, story og budget.
- Behold eksisterende posisjonelle WASM-metoder som kompatibilitetslag fram til en
  senere major-versjon.
- La en `Engine` ta en ferdig dictionary og kunne lage en avledet engine med overlay.
  WASM skal ikke lese filer eller bake inn en-US.
- La top-level npm-wrapperen eie lasting av standard en-US.
- Velg og dokumenter én merge-default på tvers av Rust, WASM og npm.
- Bestem at dictionary er en compile-default og ikke en per-run-option for compiled
  WASM. Snevre inn `Compiled`-typene tilsvarende, eller implementer reell per-run-støtte;
  typene skal aldri love en ignorert option.
- `output(..., { story: true })` og `explain(..., { story: true })` skal ha dokumentert
  og testet likhet eller en tydelig begrunnet forskjell.

### 0.4 Lint- og CLI-korrekthet

- Gjør newline etter setningspunktum til samme skille, ikke et tomt ekstra beat.
- Legg regresjonstest for riktige beat-indekser.
- Håndhev den dokumenterte story-policyen for åpne verbqueries, inkludert
  `<verb-walk>`.
- Gjør story-exitkode lik for argument, `-f` og stdin i begge CLI-er.
- Implementer `:story` i npm-REPL eller fjern det fra hjelpeteksten. Native REPL skal
  ha samme eksplisitte valg.

### 0.5 Korrekt provenance

- Skill mellom **execution trace** og **final-output lineage**:
  - `picks` og `choices` beskriver alle evaluerte beslutninger og får feltene
    `channel: string | null` og `emitted: boolean`, også for skjult cast og
    ikke-emitterende uttrykk.
  - `parts` og `density` beskriver fortsatt bare ferdig `main`-output for
    bakoverkompatibilitet.
  - Named channels ligger i `partsByChannel: Record<string, OutputPart[]>`; `parts`
    er kompatibelt alias for `partsByChannel.main`. Named channels blandes ikke inn i
    main-density.
- La captures bevare både tekst og lineage-segmenter.
- Bevar dictionary/glue-lineage gjennom blocks, repeats, functions og channels.
- Definer transformasjonsregler eksplisitt:
  - case-transformasjon endrer tekst, men beholder origin
  - `[a]` legger inn runtime-avledet glue foran det opprinnelige segmentet
  - `[replace]` beholder lineage for urørt input og bruker replacement-bodyens lineage
    for erstattet output
  - `[let]`, block-weights og andre ikke-emitterende evalueringer lager trace-events,
    men ingen final-output parts
- Legg til `choices` i explain-output, eksempelvis:

```json
{
  "kind": "block",
  "span": { "start": 10, "end": 29 },
  "alternative": 1,
  "repeatIndex": 0
}
```

- Pattern-ord i et block-alternativ er fortsatt pattern/glue; trace viser separat at
  Skald valgte alternativet.
- `density.queries` teller dictionary-query-events hvis tekst bidrar til ferdig
  `main`; det skal derfor ikke forventes å være lik `picks.length`. Glue ratio regnes
  fra final-output lineage. Scope og invariants dokumenteres og testes eksplisitt.

### Definition of done for 2.1-grunnmuren

- Alle CI-jobber er grønne fra en ren checkout.
- CLI kan bruke `--dict docs/beats/data/inn.json` til å rendre et mønster som
  kombinerer en-US, for eksempel `<firstname>`, med scene-tabellen `<inn_drink>`.
- Samme pattern, seed og dictionary gir samme output via Rust, WASM og npm.
- Story-feil gir samme exitkode uavhengig av inputkanal.
- Nested blocks har korrekt provenance og choice-trace.
- `cargo package` og `npm pack --dry-run` lykkes.
- Begge pakkene installeres og smoke-testes fra de produserte artifactene.

## Milepæl 1: StorySpec og Story Runner for 2.1

### 1.1 Stabil StoryDraft-kontrakt

Oppdater schemaet til minst følgende form:

```json
{
  "schemaVersion": 1,
  "cast": [
    { "id": "hero", "query": "<firstname female>" },
    { "id": "other", "query": "<firstname male>" }
  ],
  "beats": [
    "<::hero> the {knight|ranger|traveler} came to the inn.",
    "<::other> waited beside the {fire|window|door}."
  ]
}
```

Schema og semantisk validering skal kreve:

- kjent `schemaVersion`
- ingen ukjente felter
- ikke-tom cast og ikke-tomme beats der briefen krever personer
- carrier-ID-er med en trygg, enkel grammatikk
- unike cast-ID-er
- nøyaktig én enkel query-AST per `cast[].query`; feltet kan ikke inneholde carrier,
  tags, blocks, surrounding text eller flere queries
- størrelsesgrenser for dokument, beats og blocks
- ingen seed, palette-filbane eller provider-konfigurasjon fra LLM-en

### 1.2 Gjenbrukbar runner

Refaktorer `examples/story/host.mjs` til en miljønøytral, importérbar kjerne med tynne
Node- og browseradaptere. Fil-I/O, CLI-prosess og browser-fetch skal ikke ligge i den
delte kjernen. Før en eventuell publisering skal minst disse operasjonene finnes:

- `validateStoryDraft(draft)`
- `analyzeStoryDraft(draft, policy)`
- `buildStoryPattern(draft, cast, palettes)`
- `renderStory(request, draft)`
- `createStoryArtifact(request, draft, result)`

CLI-en bør støtte separate `check`- og `render`-moduser og alltid kunne returnere JSON.
Runneren beholdes i `examples/story/` til kontrakten er bevist. Deretter kan den
eventuelt publiseres som et separat subpath eller en separat pakke; den skal ikke
gjøre top-level `skald()` større.

### 1.3 Cast som faktisk input

`cast` skal være autoritativt, ikke dekorativ metadata:

- Hosten bygger en skjult prelude før modellens beats, for eksempel
  `[out:cast_hero]{<firstname female :: hero>}`.
- Modellskrevne beats bruker `<::hero>` og får ikke definere cast på nytt.
- Alle inline match-carrier-definisjoner i modellskrevne beats avvises; bare recall av
  deklarerte ID-er er tillatt.
- Prelude kjøres først i samme Skald-run slik at carriers lever gjennom alle beats.
- Cast-kanaler og query-picks brukes til artifactets løste cast.
- Dupliserte valgte navn avvises eller løses med en dokumentert, deterministisk
  retry-regel som lagres i artifactet.
- Cast-queryens klasser og eventuelle pronomenkrav kryssjekkes.

Hvis det senere trengs bundne pronomenformer, kan hosten materialisere en midlertidig
`story_cast`-overlay med former som `name`, `nom`, `acc`, `poss` og `self`. Dette bruker
eksisterende entry/carrier-semantikk og krever ingen ny story-tag.

### 1.4 Betrodde scene-paletter

- StoryRequest inneholder `paletteIds`, ikke filstier.
- En host-eid registry mapper ID til dictionary-JSON og en kort LLM-manifestasjon.
- Paletter valideres og merger i deklarert rekkefølge før compile.
- Scene-tabeller skal være namespacede/nye som default. Override av en core-tabell
  som `firstname`, `noun` eller `place` krever eksplisitt host-policy.
- Kollisjoner mellom to paletter er feil med mindre request-policyen uttrykkelig
  erklærer en deterministisk precedence-rekkefølge.
- Modellprompten får bare tabellnavn, klasser, former og bruksbeskrivelse som faktisk
  er tilgjengelig i den valgte scenen.
- Story-policyen kan allowliste de valgte scene-tabellene.
- Story Runner CLI og Playground skal bruke samme palette-registry og merge-kjerne.
  Den generelle native CLI-en bruker eksplisitte, betrodde `--dict`-filbaner; npm- og
  Rust-API-ene tar dictionary-verdier direkte.

### 1.5 Strukturerte diagnostics

Diagnostics får stabil form:

```json
{
  "code": "STORY_OPEN_VERB",
  "severity": "error",
  "beatIndex": 1,
  "span": { "start": 18, "end": 29 },
  "message": "Open verb query in a story frame",
  "hint": "Write the predicate or use a small closed block"
}
```

`beatIndex` er nullbasert og peker direkte på `draft.beats[beatIndex]`; UI kan vise
`beatIndex + 1`. `span` er beat-lokale UTF-8 byte-offsets, samme enhet som Rust-parseren.
Hosten beholder et source map fra generert prelude/joined pattern tilbake til hvert
beat. Hvert array-element analyseres som ett beat, også når det inneholder flere
setninger; setningsposisjon kan være valgfri underlokasjon, men endrer ikke
`beatIndex`. Eksisterende `story:`-strenger kan beholdes som kompatibel rendering av
de strukturerte funnene.

Minimumskoder:

- ugyldig schema eller størrelse
- ukjent/ikke-tillatt query-tabell
- åpen verb-, noun-, adjective- eller place-query i streng story-profil
- ubundet, ukjent eller redefinert carrier
- cast-query som ikke matcher deklarasjonen
- ukjent palette-ID
- for stort eller komplekst block
- strukturert unresolved-query-event etter rendering
- duplisert generert cast-navn
- runtime-budget eller tom referent

Uløste queries skal registreres av runtime/analyse med original source span. Hosten
skal ikke lete etter `<...>` i ferdig tekst, fordi escaped literaltekst eller en
dictionary-verdi ellers kan gi falske treff.

### 1.6 Streng story-profil

Dette er syntaktisk policy, ikke verdensmodell:

- Modellens beats kan bruke carrier-recall, godkjente pronomenformer, små blocks og
  eksplisitt allowlistede scene-tabeller.
- Åpne globale verb-, adjective-, noun- og place-tabeller er feil som default.
- Avanserte tags er av som default i modellskrevne beats og kan åpnes eksplisitt i
  host-policy.
- Blocks får konfigurerbare grenser for antall alternativer, nesting og ord per
  alternativ.
- Alle alternativer må passe den samme grammatiske rammen; dette forklares i prompten
  og evalueres redaksjonelt, ikke med verdenslogikk i VM-en.

### Definition of done for StorySpec/Runner

- Samme StoryRequest-kontroller, StoryDraft, Skald-versjon og eksakte palette-data gir
  byte-identisk kanonisk replay-payload.
- Beat-endringer etter cast-prelude endrer ikke valgt cast.
- Ingen `<raw>`, tom referent eller ubundet carrier kan godkjennes.
- Kun paletter valgt av caller kan brukes.
- Alle diagnostics peker på riktig JSON-beat og source span.
- StoryArtifact inneholder minst seed, Skald-versjon, schema-/promptversjon,
  palette-/dictionary-hash, mønster, tekst, løst cast, picks, choices og diagnostics.

## Milepæl 2: provider-nøytral LLM-loop for 2.1

Definer et lite host-grensesnitt, ikke en VM-funksjon:

```text
StoryModel.plan({ narrativeBrief, deviation, expansion, theme })
  -> StoryIntent
StoryModel.design({ narrativeBrief, storyIntent, deviation, expansion, theme })
  -> StoryDesign
StoryModel.compose({ narrativeBrief, storyIntent, storyDesign, diagnostics })
  -> { text }
StoryModel.reviewManuscript({ narrativeBrief, storyIntent, storyDesign, manuscript })
  -> { ok, scores, diagnostics[] }
StoryModel.segment({ manuscript })
  -> literal StoryDraft
StoryModel.skaldize({ manuscript, segmentedDraft, paletteManifest })
  -> { cast, substitutions }
StoryModel.reviewSkaldization({ segmentedDraft, transform, draft })
  -> { ok, diagnostics[] }
StoryModel.revise({ draft, diagnostics, revisionPlan })
  -> locally repaired StoryDraft
StoryModel.review({ narrativeBrief, draft })
  -> { ok, diagnostics[] } // optional semantic gate
```

Flyten er:

1. En adapter med `plan` bygger først en StoryIntent med låste ankere, et lite sett
   utviklingsbevegelser, tematisk bruk/unngå-liste, eventuell komisk mekanisme og
   ønsket slutteffekt.
2. `design` lager en komposisjonsplan med distinkte bevegelser, motivfunksjoner,
   rytme og oppsett til slutten.
3. `compose` skriver ett helhetlig manus uten Skald-syntaks eller beat-grenser.
4. `reviewManuscript` avviser manglende valg/konsekvens, gjentatt dramatisk funksjon
   og fortellerkommentar som forklarer temaet. Hvert problem må ha et eksakt tekstutdrag.
   Eksakte titler, navn og formularer fra StoryIntent kontrolleres deterministisk.
5. `segment` deler først et godkjent manus i en literal StoryDraft uten omskriving.
6. `skaldize` foreslår eksakte literal→pattern-substitusjoner. Hosten anvender dem,
   slik at dette steget ikke kan skrive om resten av prosaen. Alle egnede verb,
   adjektiv, adverb, substantiv, variable referenter og utskiftbare detaljer skal under
   Skald-kontroll. Lukkede grammatiske blokker brukes når åpne queries skader
   argumentstruktur eller kollokasjon. En coverage-review avviser lav parametrisering.
7. Hosten kjører schema-, policy-, carrier- og query-validering uten rendering.
8. En adapter med `review` kjører deretter en streng, strukturert brief-evaluering av
   form, evidens, kausalitet, rytme, fakta og sluttvirkning. Denne kan slås av eksplisitt
   med policy, men er på som standard når adapteren tilbyr den.
9. Ved lokale feil får modellen original `narrativeBrief`, StoryIntent, schema, castkrav, palette-manifest,
   uforandret request-policy, den feilende draften og strukturerte diagnostics tilbake.
   Revieweren peker samtidig ut beats som skal fryses byte-for-byte og de minste
   områdene som skal erstattes. Reparasjon skal ikke omskrive resten av draften.
   Strukturelle feil i bue, rekkefølge, kausalitet eller slutt går tilbake til
   helmanuset før ny segmentering og Skald-transformasjon.
10. Maks to reparasjonsforsøk er tillatt som default.
11. En strukturelt og semantisk ren draft rendres én gang med Skald og blir et StoryArtifact.
12. Runtime-feil og navnekollisjoner håndteres deterministisk av hosten, ikke som fri
   kreativ modellrevisjon.

Krav:

- Provider-adapter injiseres av caller.
- En AI-kjøring oppgir provider, modell og reasoning eksplisitt. Replay gjør aldri det.
- Minst én offline mock-adapter brukes i CI.
- En valgfri eksempeladapter kan vise strukturert output hos én provider, men den er
  ikke del av VM-en eller påkrevd runtime dependency.
- Provider-nøkler skal aldri ligge i Playground-klienten.
- Ferdig Skald-tekst sendes ikke tilbake for prose rewrite som default. En eventuell
  postprosess må være eksplisitt, merkes som ikke-reproduserbar og få eget provenance-
  lag.

### Definition of done for LLM-loop

- En mock-modell som først gir en dårlig draft og deretter en god draft demonstrerer
  hele reparasjonsløkken offline.
- En draft som fortsatt er ugyldig etter retry-grensen ender med stabilt feilartifact
  og non-zero exit.
- Seed, cast-ID-er, `narrativeBrief` og palettevalg endres ikke under reparasjon.
- Antall modellkall og alle diagnostics finnes i receiptet.
- Selve LLM-kallet regnes ikke som reproduserbart. Replay-løftet gjelder rendering fra
  en fast StoryRequest, StoryDraft, Skald-versjon og eksakte dictionary-/palette-data.
- Timestamps, latency, provider-request-ID-er og annen volatil telemetri lagres utenfor
  artifactets kanoniske replay-payload og hash.

## Milepæl 3: kvalitet, Playground og 2.1-release

### 3.1 Automatiske story-tester

Legg til ende-til-ende-tester for:

- begge fullstendige story-goldens
- positiv inn-story og negativ «Don't»-story
- ugyldig schema og ukjente felter
- duplisert/mismatchet cast
- ubundet carrier og carrier-rebinding
- ukjent query og manglende palette
- overlay merge og replace-only
- stdin-/fil-/argument-exitkoder
- compiled dictionary-semantikk
- korrekt provenance gjennom nested blocks og functions
- deterministisk retry ved navnekollisjon

Kjør i tillegg en seed-matrise over alle story-fixtures. Den skal kontrollere:

- ingen rå queries
- ingen tomme referenter
- samme carrier gjennom hele historien
- unike cast-navn når policyen krever det
- identisk replay for samme artifact
- gyldige dictionary- og block-decisions i receiptet

Eksakte goldens beskytter engine/RNG/host-kontrakten. De skal ikke brukes som eneste
mål på prosakvalitet.

### 3.2 Redaksjonell kvalitet

Opprett et lite, versjonert brief-korpus på tvers av sjanger, cast-størrelse og
historielengde. Mål minst:

- schema-pass på første forsøk
- reparasjonssuksess innen retry-grensen
- grammatikk og kollokasjon
- kausal sammenheng mellom beats
- referentklarhet
- uønsket repetisjon
- forskjell mellom åpen Mad Libs, frame+Skald og LLM-only

Strukturelle tester er autoritative i CI. Menneskelig eller LLM-basert vurdering kan
brukes som eksplisitt, versjonert eval og skal ikke gjøre vanlige builds avhengige av
nettverk. Glue ratio er observasjon, ikke en kvalitetsport for historier.

### 3.3 Playground

Utvid Playground først etter at runner-kontrakten er stabil:

- Pattern-modus og Story JSON-modus
- valg blant host-godkjente scene-paletter
- schema- og policyfeil gruppert per beat
- løst cast og seed
- dictionary/glue-segmenter og block-decisions
- komplett receipt
- «Copy repair payload» for manuell modellrevisjon
- ingen provider-nøkler eller direkte modellkall fra nettleseren

Playground skal konsumere den miljønøytrale runner-kjernen, registry-dataene og
diagnostics-kontrakten som Story Runner CLI-en. Browser- og Node-lasting er separate
adaptere.

### 3.4 Dokumentasjon og budskap

- Gjør story-prompten til én kanonisk kilde; README, kokebok og Playground skal hente
  eller genereres fra den for å unngå drift.
- Vis én komplett kommando for brief/draft, check, render og receipt.
- Forklar forskjellen mellom story-frame og høy query-density for NPC/flavor.
- Presiser at pattern/glue er modellskrevet, dictionary-picks kommer fra Skald, og
  block-alternativer er patternskrevet men Skald-valgt.
- Ikke markedsfør story-output som om ingen ord kan stamme fra en modell.

### 3.5 Publish-verifikasjon

Før 2.1-tag:

1. Kjør hele CI-matrisen fra ren checkout.
2. Bygg og inspiser `cargo package`.
3. Kjør `npm pack --dry-run` og kontroller fillisten.
4. Installer crate-/npm-artifact i tomme smoke-prosjekter.
5. Verifiser Node- og browser-importer, CLI, Story Runner og TypeScript-typer.
6. Kontroller versjoner i workspace, crate, wasm, npm, Playground-tekst og docs.
7. Publiser crate/npm etter eksplisitt releasebeslutning.
8. Opprett GitHub Release bare dersom changelog, kildearkiv eller binærartefakter skal
   distribueres der.

### Definition of done for 2.1

- Story Runner kan ta et brief gjennom en mock/provider-adapter, reparere en ugyldig
  draft og produsere en kanonisk, reproduserbar render-payload i StoryArtifactet.
- Native CLI kan laste den samme dictionary-JSON-en med `--dict`; Story Runner CLI og
  Playground deler palette-ID-registry; npm/Rust-API-ene kan bruke dictionary-verdien
  direkte.
- Alle strukturelle og multi-seed-tester er grønne i CI.
- Provenance skiller korrekt mellom pattern/glue, dictionary-picks og Skald-valgte
  block-alternativer.
- Ingen plot, verdensmodell eller provider-avhengighet er lagt til VM-en.
- Crate- og npm-artifact er bygget, inspisert og installasjonstestet.

## Foreslått implementeringsrekkefølge

1. CI-skjelett og automatiserte eksisterende goldens.
2. CLI `--dict`, stdin-exitkode og merge-paritet.
3. WASM options/budget og compiled API-kontrakt.
4. Capture-aware provenance og block choice-trace.
5. Strukturert lint med korrekte beat-indekser.
6. StoryDraft-schema og importérbar runner.
7. Autoritativ cast-prelude og palette-registry.
8. Negative fixtures og seed-matrise.
9. Provider-nøytral reparasjonsloop med offline mock.
10. Playground Story JSON/overlay/receipt.
11. Dokumentasjon, package-smoke og 2.1-release-gate.

Denne rekkefølgen gjør grensesnittene pålitelige før modellintegrasjonen bygges, og
holder all story-intelligens på riktig side av VM-grensen.
