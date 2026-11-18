---
name: new-adr
description: Creates a new numbered ADR in docs/adr/ from the template and updates links (ARCHITECTURE.md, superseded ADR). Use when an architecture decision is made or changes.
---

# Create an ADR

Argument: short title of the decision.

ADRs are written in French (the template headings are French); this file is in English.

1. List `docs/adr/` and compute the next number (4 digits, template excluded).
2. Read `docs/adr/0000-template.md`, `ARCHITECTURE.md` §1 (constraints C1 to C7), §14 (planned ADR table) and §16 (hypotheses), plus related ADRs.
3. Create `docs/adr/NNNN-french-slug.md` following the template exactly: status `Proposé` (or `Accepté` if the user says so), today's date, context citing the constraints, precise and verifiable decision, rejected alternatives with reasons, consequences (positive, negative, risks and follow-up).
4. If the ADR supersedes another: set the old one's status to `Remplacé par ADR-NNNN` and change nothing else.
5. Add the reference in `ARCHITECTURE.md` (§14 or §16) when relevant.
6. Do not commit. Summarize in 3 lines what was decided and what remains hypothetical.

Never invent a measurement or test result: any unverified claim is marked as a hypothesis.
