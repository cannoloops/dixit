# Meeting Summary Templates

This directory contains template definitions for meeting summary generation.

## Available Templates

### 1. `daily_standup.json`
Time-boxed daily updates template designed for engineering/product teams.

**Sections:**
- Date
- Attendees
- Yesterday (completed work)
- Today (planned work)
- Blockers
- Notes

### 2. `standard_meeting.json`
General-purpose meeting notes template focusing on key outcomes and actions.

**Sections:**
- Summary
- Key Decisions
- Action Items
- Discussion Highlights

### 3. `sal.json`, `analisi.json`, `sal_analisi.json` (Dixit)
Picked automatically with the `auto` template id: a short LLM pass classifies the call
(`sal`, `analisi`, `mix`, `altro`, see `src/summary/classification.rs`) and saves the type on
the meeting (`meetings.meeting_type`). `altro` uses `standard_meeting.json`.

- **SAL**: summary, participants, activity status (done / in progress / blocked), decisions, action items, risks, next steps
- **Analisi**: context, participants, topics, requirements and rules, decisions, open questions, next steps
- **SAL + Analisi**: the SAL minutes plus the analysis sections

Templates are written in English (the summary is generated in English and then translated);
`name_it` and `description_it` are the Italian labels shown in the app. Write instructions
that never ask for facts the call may not contain: anything not stated is "Not stated".

## Template Structure

Each template JSON file follows this schema:

```json
{
  "name": "Template Name",
  "description": "Brief description of the template's purpose",
  "sections": [
    {
      "title": "Section Title",
      "instruction": "Instructions for the LLM on what to extract/include",
      "format": "paragraph|list|string",
      "item_format": "Optional: Markdown table format for list items"
    }
  ]
}
```

## Custom Templates

Users can add custom templates to the application data directory:

- **macOS**: `~/Library/Application Support/Meetily/templates/`
- **Windows**: `%APPDATA%\Meetily\templates\`
- **Linux**: `~/.config/Meetily/templates/`

Custom templates override built-in templates with the same filename.

## Template Fields

### Root Level
- `name` (required): Display name for the template
- `description` (required): Brief explanation of the template's use case
- `sections` (required): Array of section definitions
- `name_it`, `description_it` (optional): Italian name and description for the UI

### Section Object
- `title` (required): Section heading text
- `instruction` (required): LLM guidance for this section
- `format` (required): One of `"paragraph"`, `"list"`, or `"string"`
- `item_format` (optional): Markdown formatting hint for list items (e.g., table structure)
- `example_item_format` (optional): Alternative formatting hint

## Usage in Code

Templates are loaded using the `templates` module:

```rust
use crate::summary::templates;

// Get a specific template
let template = templates::get_template("daily_standup")?;

// List available templates
let available = templates::list_templates();

// Validate custom template JSON
let custom_json = std::fs::read_to_string("custom.json")?;
let validated = templates::validate_template(&custom_json)?;
```
