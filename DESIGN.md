---
name: Dixit
description: "Assistente di riunioni locale, nel design system Dixit: antracite e bianco, petrolio per l'azione, verde per la tua voce."
colors:
  background: "#FFFFFF"
  foreground: "#2A2A2C"
  primary: "#1F8093"
  secondary: "#EEF0F2"
  muted: "#F4F5F6"
  muted-foreground: "#62656A"
  accent: "#EEF4F6"
  border: "#E2E6E9"
  input: "#D2D6DA"
  rail: "#F4F5F6"
  rail-active: "#E3EFF2"
  destructive: "#C3373C"
  live: "#D83B40"
  success: "#457D17"
  speaker-me: "#457D17"
  speaker-1: "#1A77AD"
  speaker-2: "#A9610A"
  speaker-3: "#A83E91"
  speaker-4: "#12817C"
  speaker-5: "#7350AF"
  speaker-6: "#AB4121"
  speaker-7: "#414EB4"
  speaker-other: "#6F7276"
  background-dark: "#2A2A2C"
  foreground-dark: "#F2F2F3"
  card-dark: "#343437"
  muted-foreground-dark: "#AAACB1"
  border-dark: "#404145"
  rail-dark: "#232325"
  rail-accent-dark: "#5DC3DA"
  primary-dark: "#30AAC5"
  destructive-dark: "#F06A6F"
  live-dark: "#FF5C61"
  success-dark: "#8DC828"
  speaker-me-dark: "#8DC828"
  speaker-1-dark: "#5BB7EC"
  speaker-2-dark: "#F2A74A"
  speaker-3-dark: "#E58ACA"
  speaker-4-dark: "#3BC9BD"
  speaker-5-dark: "#B99EEB"
  speaker-6-dark: "#EE8363"
  speaker-7-dark: "#919EF3"
  speaker-other-dark: "#9D9FA4"
typography:
  display:
    fontFamily: "Montserrat, 'Open Sans', sans-serif"
    fontSize: "24px"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "normal"
  headline:
    fontFamily: "Montserrat, 'Open Sans', sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "normal"
  title:
    fontFamily: "Montserrat, 'Open Sans', sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.43
    letterSpacing: "normal"
  speaker-name:
    fontFamily: "Montserrat, 'Open Sans', sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: "20px"
    letterSpacing: "normal"
  body:
    fontFamily: "'Open Sans', -apple-system, BlinkMacSystemFont, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.43
    letterSpacing: "normal"
  label:
    fontFamily: "'Open Sans', -apple-system, BlinkMacSystemFont, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.33
    letterSpacing: "normal"
  stamp:
    fontFamily: "'Open Sans', -apple-system, BlinkMacSystemFont, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "normal"
rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.background}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  button-rail-outline:
    backgroundColor: "transparent"
    textColor: "{colors.primary}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  button-live:
    backgroundColor: "{colors.live}"
    textColor: "{colors.background}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  button-destructive:
    backgroundColor: "{colors.destructive}"
    textColor: "{colors.background}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "0 12px"
  input:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.md}"
    height: "32px"
    padding: "4px 10px"
  state-stamp:
    backgroundColor: "transparent"
    textColor: "{colors.muted-foreground}"
    typography: "{typography.stamp}"
    rounded: "{rounded.sm}"
    height: "20px"
    padding: "0 6px"
  sidebar-row-active:
    backgroundColor: "{colors.rail-active}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.sm}"
    height: "28px"
    padding: "0 8px"
---

# Design System: Dixit

## Overview

**Creative North Star: "Il quaderno Dixit"**

Dixit veste il design system Dixit (template Dixit 2023): antracite e bianco come base, petrolio per l'unica azione che conta, verde Dixit per la voce dell'utente e per ciò che è andato bene. L'interfaccia è un documento di lavoro, non una vetrina: testo piccolo e leggibile (14px), righe divise da filetti sottili, nessuna card. Il colore compare dove significa qualcosa: chi parla, se si sta registrando, cosa richiede attenzione.

Light e dark seguono macOS (`prefers-color-scheme`); in scuro lo sfondo è l'antracite Dixit. La sidebar ha una superficie propria (`rail`), più chiara in light e più scura in dark, come nelle app macOS. Il marchio è una D a fumetto (coda in basso a sinistra) in `currentColor` nell'interfaccia; l'icona la mostra bianca su gradiente diagonale Dixit dal verde (alto a sinistra) al blu (basso a destra).

**Key Characteristics:**
- Un solo pulsante petrolio pieno per schermata; il resto è outline o ghost.
- Il colore è informazione: rosso = sta registrando o distrugge, verde = tu o esito positivo, un colore per ogni altro parlante.
- Montserrat per titoli e nomi, Open Sans per tutto il resto.
- Piatto: filetti da 1px, niente card, un'unica ombra per ciò che galleggia.
- Densità da strumento: controlli alti 32px (28px in sidebar), testo 12-14px.

## Colors

Antracite e bianco, un petrolio, un verde e una scala di sette colori per le voci. I token con suffisso `-dark` sono i valori in modalità scura dello stesso ruolo; in CSS sono le stesse variabili ridefinite in `prefers-color-scheme: dark` (valori HSL in `globals.css`).

### Primary
- **Petrolio Dixit** (`primary`): l'azione principale della schermata, l'anello di focus, la selezione, i puntini e i timbri "da verificare". In scuro si schiarisce (`primary-dark`); in sidebar l'accento è `rail-accent-dark`.

### Secondary
- **Verde Dixit** (`success`, `speaker-me`): la voce dell'utente e gli stati positivi. In chiaro è scurito (`#457D17`) per reggere il contrasto su bianco; in scuro è il verde pieno (`success-dark`, circa #8DC928).

### Tertiary
- **Rosso registrazione** (`live`): solo per "sta registrando" (puntino, etichetta, pulsante di stop).
- **Rosso distruttivo** (`destructive`): solo per le azioni che distruggono.

### Speaker
- **Altri parlanti** (`speaker-1` ... `speaker-7`): blu, ambra, magenta, teal, viola, ruggine, indaco, assegnati per posizione nel roster ordinato per numero di turni (chi parla di più prende il primo colore), quindi in una riunione nessun colore si ripete prima dell'ottava voce. `speaker-other` (grigio) è per "Altri" e per i nomi sconosciuti. Se un nome non è nel roster, il colore deriva da un hash del nome.

### Neutral
- **Antracite e bianco** (`foreground`, `background`, `background-dark`, `foreground-dark`): testo e fondo; `card-dark` è la superficie scura rialzata (popover, card shadcn).
- **Grigi** (`secondary`, `muted`, `accent`, `muted-foreground`): fondi tenui, hover e testo secondario.
- **Filetti** (`border`, `input`, `border-dark`): divisori e contorni di campo.
- **Sidebar** (`rail`, `rail-active`, `rail-dark`): superficie e riga selezionata della sidebar.

### Named Rules
**The One Petrol Rule.** Un solo pulsante petrolio pieno per schermata, l'azione principale. Le azioni di avvio nella sidebar sono outline (testo e contorno `rail-accent`) o ghost.

**The Recording Red Rule.** Il rosso esiste solo per "sta registrando" e per le azioni distruttive. Mai per errori generici o decorazione.

**The Own Voice Rule.** Il verde è la voce dell'utente e gli esiti positivi; nessun altro parlante lo usa.

**The Roster Rule.** I colori degli altri parlanti si assegnano per numero di turni, non per ordine di arrivo.

## Typography

**Display Font:** Montserrat (600, 700; fallback Open Sans, sans-serif), self-hosted da next/font
**Body Font:** Open Sans (400, 600; fallback -apple-system, BlinkMacSystemFont, sans-serif)

**Character:** Montserrat dà ai titoli e ai nomi un tratto geometrico e fermo; Open Sans tiene il testo corrente neutro e leggibile a taglie piccole. Gli h1-h4 ereditano Montserrat da `globals.css`.

### Hierarchy
- **Display** (700, 24px, 1.25): titolo della riunione nel dettaglio.
- **Headline** (600, 20px, 1.4): titolo delle pagine (Impostazioni).
- **Title** (600, 14px): titolo di gruppo nelle impostazioni (`SettingsSection`).
- **Speaker name** (600, 13px, 20px, Montserrat): nome del parlante, nel suo colore, identico ovunque (`SpeakerName`).
- **Body** (400, 14px): testo corrente; la trascrizione corrente sale a 14px con interlinea 28px; blocchi di testo vuoto con `max-w-[60ch]`.
- **Label** (400/500, 12px): descrizioni, metadati, tempi (`tabular-nums`).
- **Stamp** (600 nominale, 11px): etichette di stato dentro il timbro. Nota: il codice chiede `font-medium`, ma Open Sans è caricata solo a 400 e 600.

### Named Rules
**The Names Only Rule.** Montserrat solo per titoli e nomi (parlanti, titoli). Mai per pulsanti, campi o testo corrente.

## Layout

Finestra desktop a tutta altezza (`h-screen`, body senza scroll: scorrono le aree interne). Sidebar a sinistra, 256px aperta e 64px chiusa (transizione di larghezza 300ms), poi il contenuto. Il dettaglio riunione è una colonna: intestazione (`px-3 pt-4 pb-3`), banda dei parlanti, poi la trascrizione con scorrimento proprio e filetto sopra. Il ritmo dello spazio è su base 4px: gap 6-8px dentro le righe, 12-16px tra i blocchi, righe di impostazione con `py-2.5`. I turni di conversazione hanno più aria sopra (16-20px) delle righe dello stesso turno (4px).

Nell'archivio della sidebar le righe tolgono dal titolo la coda data/ora automatica (formato `gg-mm-aaaa hh.mm`); un puntino da 6px in `rail-accent` segnala le riunioni con nomi da verificare.

## Elevation & Depth

Piatto per scelta: la profondità viene da toni (sidebar su `rail`, superficie scura `card-dark`) e da filetti da 1px. Esiste una sola ombra, per ciò che galleggia (toast, popover, finestre).

### Shadow Vocabulary
- **Float** (`box-shadow: 0 12px 32px -12px hsl(240 2% 17% / 0.25), 0 2px 6px -2px hsl(240 2% 17% / 0.10)`; in scuro nero a 0.55 e 0.35): token `--shadow-float`.

### Named Rules
**The Hairline Rule.** Le suddivisioni sono filetti da 1px (`border-border`, `divide-y`), mai riquadri riempiti. Niente card.

## Shapes

Angoli piccoli e sobri: base `--radius` 8px (`lg`), 6px sui pulsanti e campi (`md`), 4px su righe di sidebar, timbri e pulsanti piccoli (`sm`); `full` per puntini e luce di registrazione. La banda dei parlanti è una barra sottile (10px) con angoli `sm`, ogni turno un segmento nel colore del parlante. Il marchio è una D con la coda di un fumetto; l'icona è uno squircle.

## Components

### Buttons
- **Shape:** angolo 6px (`md`), altezza 32px (28px `sm`, 36px `lg`), testo 14px medio.
- **Primary:** petrolio pieno, testo bianco; hover a 90% di opacità. Uno per schermata.
- **Outline / Ghost:** outline con contorno `input`; ghost con velo `accent` all'hover. In sidebar, avvio in outline con accento `rail-accent`.
- **Live:** rosso registrazione, solo per fermare una registrazione in corso.
- **Destructive:** rosso distruttivo, solo per azioni distruttive.
- **Focus:** outline globale da 2px con offset 2px nel colore `ring` (petrolio); i componenti shadcn aggiungono anche `ring-1`.

### Inputs / Fields
- **Style:** contorno 1px `input`, fondo `background`, angolo 6px, altezza 32px, padding 10px. In sidebar il campo di ricerca usa `rail-active` e `rail-border`.
- **Focus:** anello `ring` da 1px; disabilitato a 50% di opacità.

### Navigation
- **Sidebar:** righe da 28px (voci archivio) e 32px (controlli) con angolo `sm`; la riga selezionata prende `rail-active` e peso medio; hover `rail-active`. Icone da 16px in `rail-muted`. Separatori `rail-border`.

### Impostazioni
- **SettingsSection e SettingRow:** titolo breve, descrizione opzionale, righe separate da filetti (`divide-y` più filetto sopra e sotto), etichetta e descrizione a sinistra, controllo a destra. Niente card.

### SpeakerName
Nome del parlante in Montserrat 600, 13px, nel colore assegnato da `speakerColor`; `title` con il nome completo; tronca se lungo. Le etichette provvisorie ("Parlante A", "Altri", "Io") passano da `speakerLabel`.

### SpeakerBand
Barra sottile con un segmento per turno su un'unica scala temporale, nomi sotto, un clic salta alla prima riga del turno (opacità 70% all'hover). Senza tempi, solo legenda.

### StateStamp
Timbro da 20px con contorno 1px, angolo `sm`, testo 11px: vocabolario fisso `recording` (rosso live), `verify` (petrolio), `processing` / `saved` (grigio), `cancelled` (grigio barrato). Ogni stato ha una sola etichetta e un solo colore ovunque.

### LiveIndicator
Puntino pieno da 8px più etichetta 12px, nel rosso `live` (`rail-live` in sidebar). Statico, senza pulsazione.

## Do's and Don'ts

### Do:
- **Do** usare un solo pulsante petrolio pieno per schermata; avvii in sidebar in outline o ghost.
- **Do** riservare il rosso a "sta registrando" e alle azioni distruttive.
- **Do** usare il verde per la voce dell'utente e per gli stati positivi: scurito in chiaro, #8DC928 pieno in scuro.
- **Do** assegnare i sette colori degli altri parlanti per numero di turni, tramite `speakerColor` e un roster ordinato.
- **Do** passare da `SpeakerName`, `StateStamp`, `LiveIndicator` invece di ricreare etichette.
- **Do** dividere con filetti da 1px; seguire macOS per light e dark.

### Don't:
- **Don't** usare Montserrat fuori da titoli e nomi.
- **Don't** introdurre card o riquadri riempiti per raggruppare contenuti.
- **Don't** usare il rosso o il verde come decorazione.
- **Don't** aggiungere ombre oltre `shadow-float`.
