---
paths:
  - "docs/**"
  - "ARCHITECTURE.md"
  - "README.md"
---

# Documentation, ADRs and spikes

- Project documentation is written in French: factual tone, short sentences, no filler.
- `ARCHITECTURE.md` describes the target; if the code diverges, fix the document or write an ADR, not both versions in parallel. Section §16 separates "verified" from "hypothesis": move there whatever a spike validated.
- ADR: `docs/adr/NNNN-slug.md`, continuous numbering, template `0000-template.md`. An accepted ADR is never rewritten: create one that supersedes it and update the old status (`Remplacé par ADR-NNNN`). Use `/new-adr`.
- Spikes: `docs/spikes/X.Y-slug.md` with throwaway code in `spikes/<name>/` (outside the workspace). A spike records the question, method, hardware measured, quantified result and conclusion.
- Diagrams are Mermaid.
