---
description: Answer a theology question from your own vault notes, with citations
argument-hint: <question>
allowed-tools: mcp__didaskalos__search, mcp__didaskalos__read_note, mcp__didaskalos__neighbors, mcp__didaskalos__lessons, mcp__didaskalos__check_citations
---

Answer this question from my theology vault: $ARGUMENTS

Work this way:

1. **Search.** Call `search` with the question as asked. If the question has
   parts (a "what" and a "why", or two things to compare), search each part
   with its own wording too. If a section that matters is cut off (ends in
   "…"), open it with `read_note` and its heading.
2. **Follow my links when the answer needs them.** Notes listed under
   "Linked from these results" are ones I connected myself; read the ones
   whose titles bear on the question.
3. **Write the answer** in this shape:
   - **The short answer:** two or three sentences that answer directly.
   - **From your notes:** the explanation, built only from what the notes
     say. Cite with the exact form `search` printed: `[[slug#Heading]]`.
     Cite a source once, at the end of the sentences it supports, not
     after each one: when several sentences in a row come from the same
     section, put one citation after the last of them. Cite again only when
     the source changes, and always right after a direct quote. Mark quotes with quotation marks and
     copy them word for word; everything else is paraphrase and must stay
     faithful to the note. When notes disagree or a note marks something as
     uncertain, say so.
   - **Beyond your notes:** only if the notes leave a real gap. Clearly
     label anything here as general knowledge, not from the vault, and keep
     it short. Leave the section out when the notes cover the question.
   - **Read next:** two to four notes worth opening, as `[[slug]]` links,
     with a few words on why.
4. **Check before sending.** Run `check_citations` on the draft. Fix or
   remove every citation it reports missing, then give the final answer.

If the vault has nothing on the question, say so plainly, and offer what
general knowledge can say under "Beyond your notes".

Keep the tone of a teacher explaining to a student who already knows the
basics: plain words, short paragraphs, an analogy where it helps. Write in
the language of the question.
