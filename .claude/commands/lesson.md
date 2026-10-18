---
description: Build a full lesson in your vault's format, from your notes or from a source, and register it in lessons_index
argument-hint: <topic, or a chapter/transcript file to study>
allowed-tools: Read, mcp__didaskalos__search, mcp__didaskalos__read_note, mcp__didaskalos__neighbors, mcp__didaskalos__lessons, mcp__didaskalos__check_citations, mcp__didaskalos__lesson_format, mcp__didaskalos__save_lesson
---

Build a lesson on: $ARGUMENTS

## 1. Decide the kind

- **Chapter lesson** when I hand you one source to study: a chapter, a
  file (PDF, `.txt` scan or transcript in `Raw/`), a video or a talk.
- **Synthesis lesson** when I name a topic or question ("theosis", "why
  icons are not idols"), or a book whose chapter lessons already exist.

If it could be either, choose synthesis for a topic and chapter for a
file. Call `lesson_format` with that kind and follow the template and its
rules exactly: headings, order, frontmatter, quote marks, *(paraphrase)*
marks and the index row.

## 2. Check what exists

Call `lessons` with a few words from the topic or source. If a lesson on
the same thing is already there, tell me its link and ask whether to
improve that one or write a new one; stop there until I answer. For a
chapter lesson, find the previous chapter's lesson so the Chapter
Synthesis can connect to it.

## 3. Gather

- **Synthesis:** search several times with different wording (the
  question, its key terms, the objections to it). Read the strongest
  notes in full with `read_note`, and follow `neighbors` from the Wiki
  concept pages, since my links show what I connected myself. Prefer Wiki
  concept pages and lessons, then book analyses, then raw notes.
- **Chapter:** read the whole source (use Read for a file path). Quote
  only what the source says word for word. Then `search` for notes that
  already treat the chapter's themes, to link from the synthesis.

## 4. Write

- Every claim in a synthesis lesson comes from a note and carries a
  `[[slug#Heading]]` citation, cited once after the run of sentences it
  supports. A step the notes do not cover is marked ⚠ and listed under
  `## Not Sure / Check These`.
- In a chapter lesson the source is the authority: quotes word for word,
  everything else marked as paraphrase where the template says to.
- Write for a student who knows the basics: plain words, an analogy
  where it helps, no padding.
- File name and folder as the template says. Use today's date.

## 5. Check and save

1. Run `check_citations` on the draft and fix every missing citation.
2. Call `save_lesson` with the path, the whole note, and the index row
   (date, title, source, a one-sentence hook that states the thesis).
   Never pass `overwrite: true` unless I asked to replace that lesson.
3. Tell me in a few lines: the link to the new note, its thesis, how many
   notes it drew on, and anything in Not Sure / Check These.
