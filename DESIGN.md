---
name: Dixit
description: "Epigrafe romana: ogni riunione è incisa come un'iscrizione, chi ha parlato è scolpito per nome."
colors:
  marmo-carrara: "#f3f5f6"
  marmo-lastra: "#f9fafb"
  marmo-velo: "#e5e8eb"
  marmo-ombra: "#e8ebed"
  marmo-accento: "#dfe3e7"
  filetto: "#d2d6db"
  bordo-campo: "#c5c9ce"
  inchiostro: "#1d1d1b"
  inchiostro-tenue: "#555c62"
  bronzo: "#7a5c24"
  bronzo-anello: "#9c7d40"
  bronzo-lettere: "#c9a259"
  vinaccia: "#88304d"
  minio: "#ca361c"
  minio-su-pietra: "#e55238"
  peperino: "#292b27"
  peperino-testo: "#e9e9e2"
  peperino-tenue: "#a4a59c"
  peperino-filetto: "#3b3d38"
  peperino-attivo: "#393b35"
  verde-antico: "#2e6b4e"
  lapislazzuli: "#2f5598"
  blu-egizio: "#1c636e"
  terra-verde: "#49602f"
  terra-di-siena: "#894b2a"
  pavonazzetto: "#5b4b8b"
  pietra-altri: "#656b72"
typography:
  titolo-inciso:
    fontFamily: "Cinzel, serif"
    fontSize: "1.875rem"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "0.06em"
  nome-inciso:
    fontFamily: "Cinzel, serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: "20px"
    letterSpacing: "0.06em"
  titolo-rail:
    fontFamily: "Cinzel, serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: "normal"
    letterSpacing: "0.06em"
  corpo:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: "1.25rem"
  etichetta:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif"
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: "1rem"
  nota:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif"
    fontSize: "11px"
    fontWeight: 500
    lineHeight: 1
rounded:
  sm: "0px"
  md: "2px"
  lg: "4px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.bronzo}"
    textColor: "{colors.marmo-lastra}"
    typography: "{typography.etichetta}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  button-live:
    backgroundColor: "{colors.minio}"
    textColor: "#ffffff"
    rounded: "{rounded.md}"
    height: "28px"
    padding: "0 10px"
  button-destructive:
    backgroundColor: "{colors.vinaccia}"
    textColor: "{colors.marmo-lastra}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  button-outline:
    backgroundColor: "{colors.marmo-carrara}"
    textColor: "{colors.inchiostro}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  button-ghost-hover:
    backgroundColor: "{colors.marmo-accento}"
    textColor: "{colors.inchiostro}"
  input:
    backgroundColor: "{colors.marmo-carrara}"
    textColor: "{colors.inchiostro}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "4px 10px"
  state-stamp:
    backgroundColor: "transparent"
    textColor: "{colors.inchiostro-tenue}"
    rounded: "{rounded.sm}"
    height: "20px"
    padding: "0 6px"
  state-stamp-live:
    textColor: "{colors.minio}"
  state-stamp-verify:
    textColor: "{colors.bronzo}"
  rail:
    backgroundColor: "{colors.peperino}"
    textColor: "{colors.peperino-testo}"
    width: "256px"
  rail-item-active:
    backgroundColor: "{colors.peperino-attivo}"
    textColor: "{colors.peperino-testo}"
    rounded: "{rounded.sm}"
    height: "32px"
  speaker-band:
    backgroundColor: "{colors.marmo-ombra}"
    rounded: "{rounded.sm}"
    height: "10px"
  tab-active:
    textColor: "{colors.inchiostro}"
    height: "36px"
    padding: "0 12px"
  dialog:
    backgroundColor: "{colors.marmo-lastra}"
    textColor: "{colors.inchiostro}"
    rounded: "{rounded.lg}"
    padding: "24px"
---

# Design System: Dixit

## Overview

**Creative North Star: "Epigrafe romana"**

Dixit registra ogni riunione come un'iscrizione: chi ha parlato è scolpito per nome, in lettere capitali, nel colore di un pigmento romano. Il mondo è fatto di materiali di pietra. Un fondo di marmo di Carrara (freddo, mai crema), una colonna laterale di peperino scuro, il bronzo per l'azione principale, il minio rosso riservato a un solo significato: "si sta registrando adesso". Rifiuta il default della categoria: sidebar grigia, fumetti di chat, accento indaco.

La densità è quella di uno strumento da scrivania per chi passa la giornata in call: filetti sottili al posto di schede e bordi decorativi, gerarchia ottenuta solo con scala e peso, nessuna ombra a riposo. Il chiaro e lo scuro seguono macOS (`darkMode: 'media'`); in scuro il fondo diventa peperino e le lettere diventano di bronzo.

L'interfaccia è bilingue (italiano e inglese dalla lingua di sistema) tramite `makeT` in `src/i18n/index.ts`, con un file di messaggi per area. Le stringhe non stanno nei componenti.

**Key Characteristics:**
- Fondo di marmo freddo (tinta 210) e colonna di peperino in entrambi i temi.
- Un solo accento di azione, il bronzo (tinta 39); un solo rosso, il minio (tinta 9), solo per la registrazione in corso.
- Un pigmento romano fisso per ogni voce; nessun pigmento vicino al bronzo o alla vinaccia distruttiva.
- Capitali incise (Cinzel 600) solo per nomi dei parlanti e titoli di riunione; sans di sistema per tutto il resto.
- Filetti, niente schede; angoli quasi vivi (2px e 4px); modulo di 4px.
- Mossa firma: lo scalpello. Un nome che si risolve (Parlante A diventa ROSSI·MARIO) viene inciso una volta da sinistra a destra.

## Colors

Palette di materiali: marmo, pietra, bronzo, pigmenti minerali. I valori esadecimali nel frontmatter sono il tema chiaro; il tema scuro ridefinisce le stesse variabili in `src/app/globals.css` (terne HSL).

### Primary
- **Bronzo** (`bronzo`): azione principale (Genera riassunto, Avvia registrazione, Conferma), link, selezione, cursore, tab attivo, stato "Nomi da verificare". In scuro diventa bronzo chiaro (#c9a259).
- **Bronzo-anello** (`bronzo-anello`): anello di focus, 2px, offset 2px. In scuro coincide con il bronzo chiaro.
- **Bronzo-lettere** (`bronzo-lettere`): bronzo chiaro sulla pietra; testi di accento e focus nella colonna, marchio dell'icona.

### Secondary
- **Minio** (`minio`): rosso di registrazione. Compare solo quando Dixit sta registrando: pallino e testo di LiveIndicator, timbro "Registrazione", pulsante `live` (Stop). Sulla colonna scura si usa **Minio-su-pietra** (`minio-su-pietra`, leggibile sul peperino); in scuro anche `--live` vale quel valore.
- **Vinaccia** (`vinaccia`): distruttivo (tinta 340), volutamente lontana dal minio. In scuro #da819e.

### Tertiary: i pigmenti dei parlanti
Un pigmento per voce, assegnato per ordine di apparizione nel roster della riunione (nessuna ripetizione prima della sesta voce). In scuro ognuno ha una versione chiara con la stessa tinta.
- **Verde antico** (`verde-antico`, tinta 151): sempre l'utente ("Io" o il nome impostato in Impostazioni, Automazione). Mai bronzo.
- **Lapislazzuli** (218), **Blu egizio** (188), **Terra verde** (88), **Terra di Siena** (21), **Pavonazzetto** (255): le altre voci, cinque in rotazione.
- **Pietra-altri** (`pietra-altri`): "Altri" e voci sconosciute.

### Neutral
- **Marmo di Carrara** (`marmo-carrara`): fondo dell'app. **Marmo-lastra** (`marmo-lastra`): card, popover, dialoghi. **Marmo-velo**, **marmo-ombra**, **marmo-accento**: rampa di tre gradini per secondario, attenuato e hover.
- **Inchiostro** (`inchiostro`): testo; **inchiostro-tenue**: testo secondario. **Filetto** e **bordo-campo**: linee e bordi dei campi.
- **Peperino** (`peperino`) con **peperino-testo**, **peperino-tenue**, **peperino-filetto**, **peperino-attivo**: la colonna dell'archivio. In scuro la colonna diventa basalto (#121311) e il fondo dell'app peperino (#1c1d1b).

### Named Rules
**The Minium Rule.** Il rosso minio compare solo mentre si registra. Avviare una registrazione è bronzo (`default`); fermarla è `live`. Ogni altro rosso (errore, eliminazione) è vinaccia.

**The Own Pigment Rule.** Il pigmento dell'utente è verde antico e non è mai bronzo: il bronzo è l'azione, non una persona.

**The Hue Gap Rule.** Nessun pigmento sta vicino al bronzo (tinta 39) né alla vinaccia distruttiva (tinta 340). Un nuovo pigmento si sceglie a distanza dalle due, e anche dal minio (tinta 9) quando possibile.

**The Stone Ramp Rule.** I neutri sono una rampa numerata di marmo e di peperino, non grigi puri: tinta 210 per il marmo, 60-90 per la pietra scura.

## Typography

**Display Font:** Cinzel 600 (autoospitato con next/font, fallback serif), capitali, spaziatura 0.06em.
**Body Font:** sans di sistema (-apple-system, SF Pro Text, Helvetica Neue).
**Label/Mono Font:** nessuno; i numeri usano cifre tabulari (`tabular-nums`) per durate, offset e orari.

**Character:** una voce di pietra incisa per le persone e i titoli, una sans neutra e silenziosa per tutto ciò che si legge o si usa. Il contrasto sta tra i due registri, non tra molti pesi.

### Hierarchy
- **Titolo inciso** (Cinzel 600, 30px, 1.25): titolo della riunione nel dettaglio, modificabile. 
- **Nome inciso** (Cinzel 600, 12px / 20px, 0.06em): nome del parlante nel margine sinistro della trascrizione e nel pannello dei parlanti; 14px sulla riga corrente.
- **Titolo-rail** (Cinzel 600, 11px): titolo della riunione nelle righe dell'archivio e nel recupero trascrizioni.
- **Corpo** (sans 400, 14px / 20px): trascrizione, impostazioni, contenuti; trascrizione a misura fissa di 68ch.
- **Etichetta** (sans 500, 12px / 16px): pulsanti, intestazioni di giorno, LiveIndicator.
- **Nota** (sans 500, 11px): timbri di stato e orari sotto la fascia dei parlanti.

### Named Rules
**The Inscription Rule.** Le capitali incise sono solo per nomi di parlanti e titoli di riunione. Mai per pulsanti, etichette, intestazioni di impostazioni o testo corrente.

**The Display-Only Interpunct Rule.** L'interpunto (ROSSI·MARIO) si aggiunge solo alla visualizzazione con `inscribe()`; non si memorizza mai nel titolo o nel nome salvato.

**The Stripped Tail Rule.** Le righe dell'archivio tolgono dal titolo la coda automatica di data e ora (`GG-MM-AAAA HH.MM`) con `railTitle()`, perché la data è già nell'intestazione del giorno e nell'orario della riga.

**The No Relief Rule.** Nessun rilievo CSS sul testo: niente `text-shadow`, niente doppie ombre che simulino la scalpellatura. L'incisione è la forma delle lettere e l'animazione di rivelazione.

## Layout

Modello a due colonne: colonna di peperino a sinistra (256px, 64px quando è chiusa, transizione di larghezza 300ms) con l'archivio datato (data, titolo inciso, durata, stato) e in basso l'azione di avvio; area principale a destra. Il dettaglio riunione si compone, dall'alto: titolo inciso e riga data/durata, azione bronzo in alto a destra (Genera riassunto), fascia dei parlanti a larghezza piena, poi la trascrizione e a destra il pannello del riassunto (la trascrizione occupa il 65% di default).

La trascrizione ha misura fissa: griglia `minmax(0,8rem) 3.5rem minmax(0,68ch)`, nomi scolpiti nel margine sinistro allineati a destra, offset tabulari, testo in sans. Il ritmo è un modulo di 4px (spazi 4, 8, 12, 16, 24). Le impostazioni sono righe separate da filetti (`SettingsSection`, `SettingRow`): titolo breve, descrizione, controllo a destra, nessuna scheda. Finestra dell'avviso di chiamata: `public/banner.html`, stesse variabili, Cinzel 600 a 15px per il titolo.

## Elevation & Depth

Piatto per natura: la profondità viene dal cambio di materiale (marmo contro peperino) e dai filetti, non dalle ombre. Le superfici a riposo non hanno ombra.

### Shadow Vocabulary
- **Float** (`--shadow-float`: `0 12px 32px -12px` inchiostro al 35% più `0 2px 6px -2px` al 12%; in scuro nero al 60% e 40%): solo per superfici che si staccano dalla pagina, cioè dialoghi e toast di Sonner. Diffusa, mai a sbalzo netto.

### Named Rules
**The Flat-At-Rest Rule.** Nessuna ombra su colonna, righe, pulsanti, campi. Solo dialoghi e toast usano Float.

## Shapes

Angoli quasi vivi, da lastra tagliata: raggio base 4px (`--radius`), 2px per pulsanti e campi (`md`), 0 per timbri, righe della colonna e fascia (`sm`). Il timbro di stato è un rettangolo con bordo di 1px e lettere piccole, come un timbro. Il pallino di registrazione è l'unica forma circolare del sistema (8px). La fascia dei parlanti è una striscia di 10px dove ogni turno è un segmento nel pigmento del parlante su un'unica scala di tempo.

## Components

### Buttons
- **Shape:** angoli di 2px, altezza 32px (28px `sm`, 36px `lg`), testo 14px peso 500.
- **Primary (`default`):** fondo bronzo, testo marmo-lastra. Avvia registrazione, Genera riassunto, Conferma.
- **Live:** fondo minio, testo bianco, altezza `sm`. Solo per fermare una registrazione in corso.
- **Destructive:** vinaccia. **Outline:** bordo `input` su fondo. **Secondary / Ghost:** velo di marmo, hover su marmo-accento. **Link:** bronzo.
- **Hover / Focus:** hover sfuma il fondo al 90%; focus ad anello di 1px in bronzo-anello; transizione dei colori, nessun movimento.
- Le varianti `green`, `blue`, `red`, `gray` sono residui di Meetily rimappati sulla palette (bronzo, bronzo, vinaccia, secondario): non usarle in nuovo codice.

### Inputs / Fields
- **Style:** altezza 32px, bordo 1px `bordo-campo`, fondo app, angolo 2px, testo 14px.
- **Focus:** anello di 1px in bronzo-anello. Nella colonna: fondo peperino-attivo, bordo peperino-filetto, anello bronzo-lettere.
- **Disabled:** opacità 50%.

### Navigation
- **Colonna (peperino):** righe da 32px con icone da 16px in peperino-tenue; hover e attivo su peperino-attivo, testo peperino-testo; intestazioni di giorno in 12px peso 500. La riga archivio mostra orario tabulare, titolo inciso (11px), durata e, se serve, il timbro "Nomi da verificare" in bronzo-lettere.
- **Tab:** linea di base con filetto, tab attivo con sottolineatura di 2px in bronzo e testo inchiostro.

### Dialoghi e toast
Dialogo su marmo-lastra, bordo, angolo 4px, padding 24px, ombra Float, ingresso con zoom e dissolvenza di 150ms. I toast Sonner usano le stesse variabili del popover.

### State stamp (timbro di stato)
Vocabolario fisso: ogni stato ha una sola etichetta e un solo colore ovunque. Registrazione (minio), In elaborazione e Salvato (tenue), Nomi da verificare (bronzo), Annullata (tenue, barrata). Rettangolo di 20px con bordo 1px, testo da 11px.

### Speaker name e LiveIndicator (firma)
`SpeakerName` mostra l'etichetta del parlante in capitali incise con interpunti, nel suo pigmento, troncata con titolo di tooltip; quando il nome cambia (Parlante A diventa un nome vero) riceve la classe `chisel`: `clip-path` da sinistra a destra in 420ms con easing `cubic-bezier(0.16, 1, 0.3, 1)`, una volta sola, disattivata con `prefers-reduced-motion`. I segnaposto del backend (Parlante A, Altri, Io) sono tradotti da `speakerLabel`. `LiveIndicator` è un pallino di 8px più etichetta di 12px, in minio (o minio-su-pietra sulla colonna).

### Icona
Generata da `src-tauri/icons/source/dixit-mark.py`: monogramma bronzo "D·" (D con interpunto) su pietra, in tre forme (icona app, icona piccola, template per la barra dei menu). Gli effetti di rilievo e ombra valgono solo per il raster dell'icona, non per l'interfaccia.

## Do's and Don'ts

### Do:
- **Do** usare bronzo per avviare e confermare, `live` solo per fermare la registrazione.
- **Do** passare ogni nome di parlante da `SpeakerName` e ogni titolo mostrato da `inscribe()`; nelle righe archivio anche da `railTitle()`.
- **Do** dare ai nuovi parlanti un pigmento con tinta lontana da 39, 340 e dal pigmento già in uso, con una versione chiara per lo scuro.
- **Do** separare con filetti (`border-border`, `divide-y`), raggruppare con spazio.
- **Do** usare le cifre tabulari per orari, offset e durate.
- **Do** aggiungere stringhe tramite `makeT` nel file dell'area, sia `it` sia `en`.

### Don't:
- **Don't** usare il minio per errori, avvisi o decorazione; l'errore è vinaccia.
- **Don't** dare all'utente un pigmento bronzo o vicino al bronzo.
- **Don't** usare pigmenti con tinta vicina a 39 o 340.
- **Don't** applicare rilievo CSS al testo (`text-shadow` o simili).
- **Don't** salvare interpunti nei dati; si aggiungono solo in visualizzazione.
- **Don't** usare Cinzel per pulsanti, etichette, intestazioni o corpo del testo.
- **Don't** usare schede, fumetti di chat, ombre a riposo, bordi decorativi o accenti indaco.

## Contraddizioni note del codice (registrate come fatto)

Il codice le porta oggi; non sono regole da ereditare.
- `src/components/onboarding/steps/WelcomeStep.tsx` imposta il nome del prodotto "Dixit" in `incisa` a 60px: non è né un nome di parlante né un titolo di riunione.
- Terra di Siena (tinta 21) dista 12 gradi dal minio (tinta 9): rispetta bronzo e vinaccia, non il minio.
