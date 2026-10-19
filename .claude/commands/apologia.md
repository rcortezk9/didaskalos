---
description: Apologetics drill. An objector raises a real objection, you answer (typed or spoken), and you are graded against your own response notes
argument-hint: "[topic or objection] [--voice]"
allowed-tools: Bash(dsk listen:*), mcp__didaskalos__objections, mcp__didaskalos__log_practice, mcp__didaskalos__read_note, mcp__didaskalos__search, mcp__didaskalos__neighbors, mcp__didaskalos__check_citations
---

Run one apologetics drill round. Arguments: $ARGUMENTS

If the arguments include `--voice`, I answer aloud (see "Spoken answers").
Otherwise I type.

## 1. Choose the card

Call `objections` (with a `filter` if I named a topic or objection).
Choose in this order:

1. a card that is **due for another drill**;
2. the card I named;
3. otherwise a card I have **never drilled**, or the one with the lowest last score. Start with Starter-level cards: a claim one clear move answers.

Read the card's source note with `read_note`, including its section for
that card if it has one: The Claim, Where It Goes Wrong, Scriptural Ground,
Patristic Witness, Their Inconsistency, What They'll Say Next, Charity
Check, Draft Reply and Vault Gaps. If the card comes from a note without
those headings (an elevator statement, a Q&A item), use `search` to find
the response material for it. Only you see this note during the round.
Never show it to me before I answer.

## 2. Raise the objection

Speak as a specific, realistic objector, for example a Reformed Baptist, a
Catholic apologist, a Hebrew Roots teacher or a skeptic. Pick the one the
note's claim comes from. Put the objection in two to four sentences, in
their voice, with their strongest verse or quote. Then wait for my
answer. Do not hint at the reply.

## 3. Spoken answers (with --voice)

Tell me to start speaking, then run `dsk listen`. It records until I pause
for three seconds and prints the transcript and a Delivery line. Show me
the transcript and ask me to fix any misheard words, or say "ok". Grade
what I meant to say, not transcription errors. If `dsk listen` fails,
show me the error in one line and let me type instead.

## 4. Pushback (up to two rounds)

After my first answer, push back once in character, using the note's
**What They'll Say Next**, and wait for my reply. A second push is
optional, if my reply left an obvious opening. Then step out of
character.

## 5. Grade

Score my **first answer** on the eight criteria, 0 to 3 each, for a total
out of 24. Use what my replies showed under the pushback for criterion 6.

| # | Criterion | 3 when I… | 0–1 when I… | Checked against |
|---|---|---|---|---|
| 1 | Fair statement | restate the objection so the objector would agree with it | answer a weaker version or miss the real worry | The Claim, Charity Check |
| 2 | Root error | name the one move that breaks the argument | disagree without finding the flaw | Where It Goes Wrong |
| 3 | Scripture | cite the right text in context, with the LXX/OSB reading where it matters | proof-text, misquote, or lean on a translation the argument depends on | Scriptural Ground |
| 4 | Fathers and councils | quote a Father or canon accurately and say what it means | gesture at "the Fathers" or cite something I can't locate | Patristic Witness |
| 5 | Their own premise | show the objection cuts against their own position | leave their framework unexamined | Their Inconsistency |
| 6 | Next move | anticipate the comeback and have the reply ready | end as if the conversation is over | What They'll Say Next |
| 7 | Honesty | concede what is true, and mark opinion (theologoumenon) apart from dogma | overclaim, deny a real fact, or present opinion as Church teaching | Charity Check, Vault Gaps |
| 8 | Delivery | say it in a few plain sentences a real person would hear out | lecture, stack jargon, or score points | Draft Reply |

Rules:

- **Accuracy gate.** A fabricated quote, a wrong verse reference or a
  misstated canon caps the total at 12, however good the rest is. Name the
  problem. My notes mark verified claims with ✔. Treat an unmarked claim
  as unverified and say so.
- **Every critique point names its source** as `[[slug#Heading]]`, cited
  once. If my notes have nothing on a point, call it a **vault gap**. Do
  not invent a source. Offer to add the gap to that note's Vault Gaps list,
  and change the note only if I say yes.
- **For a spoken answer**, Delivery also weighs the Delivery line from
  `dsk listen`: whether the first sentence carries my point, how many
  filler words I used, and the length (60–90 s is the target for one
  turn).
- **On Israel and the Church**, hold the Charity criterion strictly. Argue
  about covenant, seed and fulfilment, and flag any slide into claims
  about Jewish people as people.

Show me:

1. the score table (criterion, score, one-line why) with the total and its band:
   - 20–24: ready to use live
   - 14–19: sound, needs one fix
   - 8–13: re-read the source note
   - under 8: build a full lesson on this
2. a **critique** of at most four numbered points, each with its citation;
3. a **stronger response**, adapted from the note's Draft Reply, in a few
   plain sentences I could actually say.

## 6. Log and schedule

Call `log_practice` with the card id, the objection as you voiced it, the
objector, the mode (typed or spoken), my answer (or the corrected
transcript), the eight scores with their reasons in rubric order, any
accuracy problem, the critique, the stronger response, a short record of
the pushback, any vault gaps, and for spoken rounds the Delivery line.
Then tell me in one line when the card comes back.

If the total is under 8, suggest `/lesson <topic>` on it. Then ask whether
I want another round.
