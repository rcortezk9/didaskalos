# Search results: keyword-only vs hybrid (phase 1b)

Vault: `~/Documents/Notes/Theology` (481 notes, 9,421 chunks). Command per
question: `dsk search -n 5 <question>`. "Before" is keyword-only (commit
e8d459e). "After" is keyword + meaning (BGE small, 384 dims) merged by
reciprocal rank fusion (k=60), with the layer, section, hub, title and link
weights kept.

## Verdict

- **#7 (one true church): fixed for two of three.** "Chapter 1: The One, Holy,
  Catholic and Apostolic Church" is #1 and "Orthodoxy IS the TRUE CHURCH" is #4
  (before, the top 5 were all "True Israel" notes). `concept_church_history_and_apostolicity`
  is still not in the top 5.
- **#4 (how Orthodoxy differs): one of two.** "Thinking Orthodox, Ch. 1 — The
  Distinctiveness of Orthodox Christianity" is now #2. "7 Orthodox Beliefs That
  Shock Protestants and Catholics" is still not in the top 12.
- **The other 8: the same leading notes, with some churn in places 3-5.**
  - #2 (icons, first century) is a small step back: "7 Orthodox Beliefs That
    Shock..." drops from #3 to #10, while a daily reading and
    "Orthodox Catechesis" enter the top 5.
  - #9 (forgiveness): "Thinking Orthodox, Ch. 3", the St. John of Kronstadt note
    and the raw transcript leave the top 5; "On Resistance to Evil by Force",
    a daily reading and "Apologetic Disposition" enter. The top 2 are unchanged.
  - #1, #3, #5, #6, #8, #10: the top 2 are the same notes (sometimes swapped); see the lists below.
- Scores are on a new scale (reciprocal rank fusion), so compare ranks, not numbers.
- Cost: `dsk index` now embeds all 9,421 chunks once (about 25 minutes on 10
  cores). A search takes about 0.4 s.

## What is PSA and why is it not part of the Orthodox belief?

**Before (keyword-only)**

1. Punishment vs. Rescue: An Orthodox Analysis of the Atonement  (analysis, 73.9)
2. Ten Reasons Not to Convert to Orthodoxy — Response  (analysis, 43.2)
3. UOC 10 — Orthodox vs Protestant: What the Early Church ACTUALLY Believed  (wiki-source, 31.9)
4. Fr. Stephen (De Young) — Why Protestants Leave Protestantism  (lesson, 28.7)
5. Catechumen Hub — Orthodox Reception Preparation  (wiki-concept, 27.4)

**After (hybrid)**

1. Punishment vs. Rescue: An Orthodox Analysis of the Atonement  (analysis, 58.2)
2. Ten Reasons Not to Convert to Orthodoxy — Response  (analysis, 33.6)
3. Catechumen Hub — Orthodox Reception Preparation  (wiki-concept, 26.4)
4. Thinking Orthodox, Ch. 3 — Phronema in the Western Christian Tradition  (lesson, 25.6)
5. UOC 10 — Orthodox vs Protestant: What the Early Church ACTUALLY Believed  (wiki-source, 23.7)

## Where icons venerated in the first century church?

**Before (keyword-only)**

1. UOC 11 — Are Orthodox Icons Idolatry? What the Bible ACTUALLY Says  (wiki-source, 35.6)
2. Jonathan Pageau on Icon Veneration: Manifestation, Hierarchy, and the Flattening of the Sacred  (wiki-source, 33.8)
3. 7 Orthodox Beliefs That Shock Protestants and Catholics  (wiki-source, 25.3)
4. Study Guide: What the Orthodox Church Teaches About Icons  (book, 24.3)
5. WORST Anti-Orthodox Slop... (NeedGod.net) — Kyle  (wiki-source, 22.5)

**After (hybrid)**

1. UOC 11 — Are Orthodox Icons Idolatry? What the Bible ACTUALLY Says  (wiki-source, 70.7)
2. Jonathan Pageau on Icon Veneration: Manifestation, Hierarchy, and the Flattening of the Sacred  (wiki-source, 67.3)
3. Study Guide: What the Orthodox Church Teaches About Icons  (book, 52.7)
4. Orthodox Daily Reading — 2026-10-11  (daily, 42.3)
5. Orthodox Catechesis  (wiki-concept, 42.0)

## Can we find in the bible where icon veneration was scriptural?

**Before (keyword-only)**

1. UOC 11 — Are Orthodox Icons Idolatry? What the Bible ACTUALLY Says  (wiki-source, 40.0)
2. Do Orthodox Read the Bible? — Jay Dyer & Tucker Carlson  (wiki-source, 36.6)
3. Jonathan Pageau on Icon Veneration: Manifestation, Hierarchy, and the Flattening of the Sacred  (wiki-source, 35.7)
4. 7 Orthodox Beliefs That Shock Protestants and Catholics  (wiki-source, 25.5)
5. WORST Anti-Orthodox Slop... (NeedGod.net) — Kyle  (wiki-source, 23.6)

**After (hybrid)**

1. Jonathan Pageau on Icon Veneration: Manifestation, Hierarchy, and the Flattening of the Sacred  (wiki-source, 79.8)
2. UOC 11 — Are Orthodox Icons Idolatry? What the Bible ACTUALLY Says  (wiki-source, 74.3)
3. Do Orthodox Read the Bible? — Jay Dyer & Tucker Carlson  (wiki-source, 63.0)
4. WORST Anti-Orthodox Slop... (NeedGod.net) — Kyle  (wiki-source, 40.5)
5. Elevator Statements — Common Objections, Quick Answers  (analysis, 38.7)

## What makes the Orthodox church different from other christian religions?

**Before (keyword-only)**

1. Orthodoxy and the Religion of the Future — Part 1: Confronting Non-Christian Religions (Preface–Ch. 3)  (book, 28.6)
2. Orthodoxy and the Religion of the Future — Study Index  (book, 16.8)
3. The Religion of the Apostles — Chapter 1: Divine Father and Son  (book, 13.9)
4. The Religion of the Apostles — Chapter 7: Atonement for the Whole World  (book, 13.6)
5. The Religion of the Apostles — Chapter 5: The Saints in Glory  (book, 12.9)

**After (hybrid)**

1. Orthodoxy and the Religion of the Future — Part 1: Confronting Non-Christian Religions (Preface–Ch. 3)  (book, 38.5)
2. Thinking Orthodox, Ch. 1 — The Distinctiveness of Orthodox Christianity  (lesson, 37.7)
3. Lost Christianities (Ehrman) — Orthodox Response Comparison  (wiki-concept, 31.2)
4. Orthodoxy and the Religion of the Future — Study Index  (book, 28.7)
5. Catholic Orthodox vs Early Church Analysis - Eucharist Section  (analysis, 28.3)

## What is sola scriptura and why don't the Orthodox church follow it?

**Before (keyword-only)**

1. Sola Scriptura — Orthodox Critique  (wiki-concept, 76.6)
2. Orthodox Teaching on Sola Scriptura: An Integrated Lesson  (lesson, 71.8)
3. Ultimate Destruction of Sola Scriptura — Alex Sorin Opening & Closing Statements  (wiki-source, 62.8)
4. Orthodox vs. Protestant Live Debate: Sola Scriptura, the Eucharist, the Masoretic Text, and the Three-Tier Office — Complete Analysis  (analysis, 60.1)
5. Holy Scripture and Holy Tradition: An Orthodox Priest on Sola Scriptura and the Apostolic Oral Deposit — Complete Analysis  (analysis, 58.6)

**After (hybrid)**

1. Sola Scriptura — Orthodox Critique  (wiki-concept, 107.1)
2. Orthodox Teaching on Sola Scriptura: An Integrated Lesson  (lesson, 102.1)
3. Orthodox vs. Protestant Live Debate: Sola Scriptura, the Eucharist, the Masoretic Text, and the Three-Tier Office — Complete Analysis  (analysis, 77.6)
4. Holy Scripture and Holy Tradition: An Orthodox Priest on Sola Scriptura and the Apostolic Oral Deposit — Complete Analysis  (analysis, 77.1)
5. Ultimate Destruction of Sola Scriptura — Alex Sorin Opening & Closing Statements  (wiki-source, 65.3)

## Explain the difference between ancestral sin and inherited sin? What does the Orthodox church follow?

**Before (keyword-only)**

1. Ancestral Sin  (wiki-concept, 40.6)
2. UOC 10 — Orthodox vs Protestant: What the Early Church ACTUALLY Believed  (wiki-source, 26.9)
3. Defending Mary from Protestant Slander: Infallibility, the Queen Mother, and the Grammar of Marian Language — Alex Sorin  (analysis, 20.2)
4. Defending Mary from Protestant Slander — Alex Sorin  (wiki-source, 18.1)
5. Ten Reasons Not to Convert to Orthodoxy — Response  (analysis, 17.9)

**After (hybrid)**

1. Ancestral Sin  (wiki-concept, 84.4)
2. UOC 10 — Orthodox vs Protestant: What the Early Church ACTUALLY Believed  (wiki-source, 59.4)
3. Ten Reasons Not to Convert to Orthodoxy — Response  (analysis, 48.6)
4. Defending Mary from Protestant Slander — Alex Sorin  (wiki-source, 43.5)
5. Defending Mary from Protestant Slander: Infallibility, the Queen Mother, and the Grammar of Marian Language — Alex Sorin  (analysis, 39.4)

## Why does the Orthodox church believe that it is the one true church?

**Before (keyword-only)**

1. True Israel and Ecclesiology  (wiki-concept, 14.0)
2. Who Are the True Jews? The Seed of Abraham and the Inheritance of the Church  (analysis, 13.6)
3. Who Are the True Israelites? - A Biblical Challenge to Christian Zionism  (analysis, 12.4)
4. Who Are the TRUE Jews in the New Testament Bible? - Sam Shamoun  (analysis, 12.2)
5. Neo Judaizers Are The Biggest Threat To The Church Today  (analysis, 11.2)

**After (hybrid)**

1. Chapter 1: The One, Holy, Catholic and Apostolic Church  (book, 42.0)
2. Orthodox Catechesis  (wiki-concept, 34.4)
3. Thinking Orthodox — Chapter 10: Characteristics of the True Orthodox Theologian  (book, 33.9)
4. Orthodoxy IS the TRUE CHURCH  (wiki-source, 32.2)
5. Orthodoxy IS the TRUE CHURCH  (raw, 29.3)

## Explain Theosis and how to achieve it?

**Before (keyword-only)**

1. Experiences of Theosis — Theoria, the Uncreated Light, and the Communion of Saints  (book, 18.3)
2. Theosis is Possible Through the Uncreated Energies of God  (book, 17.0)
3. Study Guide: What the Orthodox Church Teaches About Icons  (book, 15.8)
4. Theosis — The True Purpose of Human Life  (book, 15.7)
5. Qualifications for Theosis — Humility, Asceticism, and the Holy Mysteries  (book, 15.5)

**After (hybrid)**

1. Theosis is Possible Through the Uncreated Energies of God  (book, 56.3)
2. Experiences of Theosis — Theoria, the Uncreated Light, and the Communion of Saints  (book, 55.7)
3. Theosis — The True Purpose of Human Life  (book, 49.9)
4. Qualifications for Theosis — Humility, Asceticism, and the Holy Mysteries  (book, 43.2)
5. Punishment vs. Rescue: An Orthodox Analysis of the Atonement  (analysis, 38.1)

## Why should we forgive and not depend on the other person's apology?

**Before (keyword-only)**

1. According to Christ — Elder Simeon on Treating Enemies  (lesson, 27.1)
2. Forgiveness & Reconciliation — Research Note (Orthodox & Biblical)  (analysis, 13.4)
3. Thinking Orthodox, Ch. 3 — Phronema in the Western Christian Tradition  (lesson, 10.1)
4. The Purpose of Life | St. John of Kronstadt  (wiki-source, 9.6)
5. according to christ  (raw, 8.8)

**After (hybrid)**

1. According to Christ — Elder Simeon on Treating Enemies  (lesson, 53.5)
2. Forgiveness & Reconciliation — Research Note (Orthodox & Biblical)  (analysis, 41.7)
3. On Resistance to Evil by Force — Chapter 15: On the Limits of Love  (book, 35.6)
4. Orthodox Daily Reading — 2026-04-01  (daily, 29.9)
5. Apologetic Disposition — Order Before Argument  (wiki-concept, 27.6)

## Lay out the arguments that St. Anthanasius makes for why the Incarnation was so important.

**Before (keyword-only)**

1. On the Incarnation, Sections 33–40 — Refutation of the Jews  (book, 16.8)
2. On the Incarnation, Sections 20–32 — The Death of Christ and the Resurrection of the Body  (book, 15.3)
3. On the Incarnation, Sections 11–19 — The Divine Dilemma of Knowledge and Ignorance  (book, 14.7)
4. On the Incarnation, Sections 1–10 — The Divine Dilemma of Life and Death  (book, 14.6)
5. On the Incarnation, Sections 41–57 — Refutation of the Gentiles and Conclusion  (book, 14.3)

**After (hybrid)**

1. On the Incarnation, Sections 11–19 — The Divine Dilemma of Knowledge and Ignorance  (book, 44.2)
2. On the Incarnation, Sections 1–10 — The Divine Dilemma of Life and Death  (book, 43.9)
3. On the Incarnation, Sections 20–32 — The Death of Christ and the Resurrection of the Body  (book, 43.1)
4. On the Incarnation, Sections 33–40 — Refutation of the Jews  (book, 42.5)
5. On the Incarnation, Sections 41–57 — Refutation of the Gentiles and Conclusion  (book, 42.1)

