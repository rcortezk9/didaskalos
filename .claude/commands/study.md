---
description: Walk through an argument one step at a time, from your section-study notes, then test whether it holds up
argument-hint: "<text or block, e.g. 'On the Incarnation 20-32'>, or a source file to build a new study from"
allowed-tools: Read, mcp__didaskalos__search, mcp__didaskalos__read_note, mcp__didaskalos__neighbors, mcp__didaskalos__due, mcp__didaskalos__record_review, mcp__didaskalos__lesson_format, mcp__didaskalos__save_lesson, mcp__didaskalos__check_citations
---

Study session: $ARGUMENTS

The aim is to rebuild the author's argument myself, step by step, not to
read a summary of it.

## 1. Find the study note

Search for the matching `_study` note (they are named like
`on_the_incarnation_ch20_32_study`). If I named no text, call `due` and use
the first due `_study` note. If none is due, use the next block after the
last one I studied in a series (follow the note's `Related` links with
`neighbors`).

**If no study note exists for the text:** say so, then build one before
walking through it.
- If the `section-study` skill is installed, use it.
- Otherwise call `lesson_format` with kind `section-study`, read the
  source with Read (the `Raw/` text or scan), write the note in that
  format, run `check_citations` on it, and save it with `save_lesson`.
  Pass no `index_row`, because section studies stay out of
  lessons_index.

Read the whole study note with `read_note`. Keep it to yourself during
the walkthrough.

## 2. Set the problem

Give me **The Core Problem** in two or three sentences: what the last
block left open, and the question this block answers. Then ask me what I
expect the author's first move to be. One or two sentences is enough.

## 3. Walk the chain, one step at a time

For each numbered step in **The Argument, Step by Step**:

1. Ask the step's **connector question**, the italic `↓` line before it.
   For the first step, ask what the author must establish first.
2. **Wait for my answer.** Never reveal the step before I have tried.
3. Then show the step in two or three lines, quoting the text where the
   note quotes it, with its § number. Say plainly how my answer compared:
   got it, partly, or missed it, and what I missed.
4. Keep a private tally of steps I got, partly got and missed.

If I say "skip", show the step and count it as missed. If I say "faster",
take two steps per question. At the end of the chain, give me **The whole
chain in one line** and ask me to say it back in my own words first.

## 4. Does it hold up?

Ask me three things, one at a time, waiting each time:
1. the strongest step;
2. the weakest step, and whether it is uninformed, misinformed, illogical
   or incomplete (Adler's terms);
3. my verdict: agree, or suspend judgment.

After each answer, compare it with the note's **Does It Hold Up?** and
**Steps asserted rather than argued**, and cite them as
`[[slug#7. Does It Hold Up?]]`. Disagreeing with the note is fine if I
argue for it. Say whether my reasoning is sound, not whether I matched.

## 5. Review questions

Ask the note's **Review Questions** one at a time, and check each answer
against the **Recall Key**, as `/review` does. Point out anything I
couldn't have known from the text that sits under **Not Sure / Check
These** or carries ⚠, so I don't learn it as certain.

## 6. Schedule and close

Propose a grade from the walkthrough tally and the review answers:
- **again:** missed most steps;
- **hard:** the shape but not the moves;
- **good:** most steps, with small slips;
- **easy:** the whole chain, cleanly.

Call `record_review` with the grade I accept. End with:
- the steps I missed, as links to reread;
- the question this block hands to the next one (**What Follows**);
- the next block's link, if there is one.
