---
description: Quiz yourself on notes due for review, and schedule each one's next review
argument-hint: "[optional: a note or topic to review now]"
allowed-tools: mcp__didaskalos__due, mcp__didaskalos__record_review, mcp__didaskalos__read_note, mcp__didaskalos__search, mcp__didaskalos__lessons
---

Run a review session. $ARGUMENTS

## 1. Pick what to review

- If I named a note or topic above, find that note (`lessons`, then
  `search`) and review it, whether or not it is due.
- Otherwise call `due`. Review the due notes in the order listed. Tell me
  up front how many are due. If none are due, say when the next one is and
  offer two or three of the never-reviewed lessons; start with the one I
  pick (or the first, if I just say go).

## 2. Quiz me on one note at a time

Read the whole note with `read_note`. Build 4 to 6 questions from the
note's own material, in this order of preference:

1. its `Review Questions` with the `Recall Key` answers;
2. its `Reflection Questions`;
3. its Core Claims, Key Terminology, argument steps and Key Scripture Index.

Mix the kinds: recall a term or quote, rebuild a step of the argument, say
why something follows, and one "how would you answer someone who says…"
question that applies it. Ask **one question at a time and wait for my
answer.** Never show the answer before I have tried.

After each answer, reply in two or three lines: what I got right, what I
missed or got wrong, and the note's own wording for it with its citation
`[[slug#Heading]]`. If I say "skip" or "I don't know", give the answer
and count it as missed.

## 3. Grade and schedule

When the questions for a note are done, propose a grade with one line of
reasoning:

- **again**: missed most of it;
- **hard**: got the gist but with real gaps;
- **good**: the main points, small slips;
- **easy**: everything, quickly.

If I accept or give a different grade, call `record_review` with it and
tell me the next review date. Then move to the next note, or stop if I
say so.

## 4. Close

End with one short summary: notes reviewed, their grades, the points I
missed most (as links to reread), and when the next review is due.
