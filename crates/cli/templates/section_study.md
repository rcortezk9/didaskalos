# Section study (argument walkthrough)

For a run of numbered sections of a primary text (a Father, a council,
a book of Scripture). One note per block, linked into a series. File
name: `<work_slug>_chNN_MM_study.md` (e.g. `on_the_incarnation_ch20_32_study.md`).
Save it with the book's other notes, e.g. `Orthodox/Book_Readings/<work_slug>/`.
Section studies are **not** added to lessons_index.md: call save_lesson
without index_row.

If the user's own `section-study` skill is installed (~/.claude/skills/section-study),
follow it instead of this template; this template mirrors its output.

```markdown
---
title: <Work>, Sections N–M — <Title of this block>
book: <Work> (<Author>)
translation: <translator, edition>
sections: N–M
source: "<the Raw/ text or scan this was built from, with book pages>"
reading_type: argument
created: <YYYY-MM-DD>
tags: [<author>, <topic>]
---

# <Work>, §§N–M: <Title>

> **Source note:** <which scan or text each section came from, any fixes, the reading type.>
>
> **Earlier blocks:** [[<previous_study>]] (<its theme>) · …

---

## 1. The Core Problem

<One paragraph: what the previous block left open, and the questions this block answers.>

---

## 2. The Argument, Step by Step

1. **§N — <what this step does>.** <The step, quoting the text in double quotes where it matters, with scripture references.>
   - <sub-objection or paradox, if any>
   ↓ *<the question the next step answers>*

2. **§N+1 — <…>.** …
   ↓ *<…>*

<one numbered step per section or move, each joined to the next by an italic connector question>

**Steps asserted rather than argued:**
- <a step the author assumes without defending, and why it matters>

**The whole chain in one line:** <step> → <step> → <step> → <conclusion>

## 3. Key Terms (As <Author> Uses Them)
- **<term>**: <meaning in this text, with §>

## 4. The Images

| Image | § | What it shows |
|---|---|---|
| <analogy> | <§> | <the point it carries> |

## 5. Key Quotes to Memorize
- "<quote>" (§N)

## 6. Common Misreadings
- **<misreading>**: <why the text does not say that, with §>

## 7. Does It Hold Up?
- **Strongest point:** <…>
- **Weakest point:** <…> (in Adler's terms: uninformed, misinformed, illogical or incomplete)
- **Open questions:** <…>
- **Verdict:** Agree / Suspend judgment, and why.

## 8. Where This Shows Up
- **In worship:** <troparia, feasts, services that use this teaching>
- **In later Fathers:** <who takes it up>

## 9. What Follows
- **For belief:** <…>
- **For practice:** <…>
- **Handed to the next block:** <the question §§M+1… takes up>

## 10. Review Questions
1. *(What is it about as a whole?)* <…>
2. *(A step)* <…>
3. *(A step)* <…>
4. *(A term)* <…>
5. *(Does it hold up?)* <…>
6. *(What follows?)* <…>

---

## Recall Key (Answers)
1. <answer with § citations>
…

---

## Not Sure / Check These
- <scan damage, references from memory, the note's own additions; mark the uncertain lines elsewhere with ⚠>

## Related
- Previous: [[<previous_study>]]
- First block: [[<first_study>]]
- Next: [[<next_study>]] (once written)
```

Rules:
- Quote the text word for word in double quotes; everything else is paraphrase.
- One step per section or move, in the text's order. Every step is joined by a connector question, so each step answers the one before it.
- Anything from memory (liturgical texts, later Fathers, verse numbers a scan garbled) is marked ⚠ and listed in Not Sure / Check These.
