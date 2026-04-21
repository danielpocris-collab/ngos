# `ngos` Shell Best-In-Class Roadmap

Acest document sintetizează repere externe relevante pentru construirea unui shell de top și le traduce în cerințe compatibile cu direcția `ngos`.

Surse consultate:

- fish design principles: https://fishshell.com/docs/current/design.html
- Nushell philosophy: https://www.nushell.sh/contributor-book/philosophy_0_80.html
- Nushell types / pipelines: https://www.nushell.sh/book/types_of_data.html
- Elvish unique semantics: https://elv.sh/learn/unique-semantics.html
- PowerShell pipeline overview: https://learn.microsoft.com/en-us/training/modules/understand-windows-powershell-pipeline/
- YSH error handling: https://www.oilshell.org/release/0.24.0/doc/error-handling.html
- Formal shell semantics (`Smoosh`): https://arxiv.org/abs/1907.05308
- NaSh guardrails for natural-language shell: https://arxiv.org/abs/2506.13028

## Verdict Strategic

Dacă vrem cel mai bun shell posibil pentru `ngos`, nu trebuie să copiem un shell existent.
Trebuie să combinăm explicit:

- UX bun by default ca `fish`
- pipelines structurate ca `Nushell` și `PowerShell`
- semantici prudente și explicite ca `Elvish`
- error handling modern ca `YSH`
- guardrails serioase pentru orice strat NL/AI, cum sugerează `NaSh`
- o matrice de comportament testabilă, nu doar convenții informale, cum motivează cercetarea pe semantici formale pentru shell

## Ce Face Un Shell Excelent

### 1. Este bun din prima, fără configurare grea

Lecția din `fish`:

- shell-ul trebuie să fie prietenos implicit
- puterea și ușurința nu trebuie tratate ca un tradeoff inevitabil
- prea multe opțiuni de configurare sunt adesea simptomul unui design care mută responsabilitatea pe utilizator

Traducere pentru `ngos`:

- completările, erorile, history-ul și help-ul trebuie să fie bune fără setup manual
- comportamentul important nu trebuie ascuns în toggles obscure
- configurarea trebuie să existe, dar să nu fie metoda principală de a obține un shell utilizabil

### 2. Datele nu trebuie tratate doar ca șiruri

Lecția din `Nushell`, `PowerShell` și `Elvish`:

- pipeline-urile structurate reduc parsarea fragilă de text
- shell-ul devine mai sigur când poate transmite obiecte, tabele, liste și record-uri
- interoperabilitatea cu tool-uri text rămâne importantă, dar nu trebuie să fie singurul model de date

Traducere pentru `ngos`:

- shell-ul trebuie să aibă un pipeline semantic nativ pentru obiecte `ngos`
- comenzi precum `ps`, `mounts`, `resources`, `contracts`, `queues`, `signals`, `fdinfo`, `stat`, `statfs` trebuie să poată produce și consuma record-uri tipate
- output-ul text trebuie să fie view-ul, nu adevărul intern

### 3. Erorile trebuie să fie previzibile și explicabile

Lecția din `YSH` și `Elvish`:

- shell-urile clasice au multe surprize periculoase în jurul exit status și control flow
- un shell bun nu ar trebui să piardă tăcut erori sau să le mascheze prin contexte speciale
- tratamentul explicit al erorilor este mai bun decât convențiile fragile

Traducere pentru `ngos`:

- eroarea trebuie să aibă tip semantic, nu doar cod numeric
- comanda eșuată trebuie să spună: ce a refuzat, de ce, pe ce obiect și dacă există recovery
- `if`, workflow-urile și pipeline-urile nu trebuie să ascundă eșecul implicit
- trebuie să existe un model clar pentru:
  - `success`
  - `refusal`
  - `recoverable failure`
  - `fatal failure`

### 4. Shell-ul trebuie să fie bun atât interactiv, cât și ca limbaj

Lecția din `Nushell`:

- un shell mare trebuie să rămână și limbaj de scripting coerent
- modularitatea, lizibilitatea și compoziția nu sunt opționale

Traducere pentru `ngos`:

- același model semantic trebuie să funcționeze pentru:
  - comandă interactivă
  - script
  - workflow agent
  - proof runner
- shell-ul nu trebuie să aibă o lume pentru interactive și alta complet diferită pentru scripturi

### 5. Trebuie să existe guardrails pentru AI / natural language

Lecția din `NaSh`:

- un shell care acceptă limbaj natural nu poate trata modelele ca executor implicit de încredere
- utilizatorul trebuie ajutat să înțeleagă, confirme și recupereze după erori ale modelului

Traducere pentru `ngos`:

- dacă introducem vreodată suprafață NL/AI, ea trebuie să fie separată clar de shell commands
- nu trebuie să existe autodetecție ambiguă între comandă și prompt AI
- orice plan AI trebuie să arate:
  - comanda propusă
  - obiectele afectate
  - riscul
  - refusal-urile posibile
  - cerința de confirmare pentru mutații cu impact

## Modelul Recomandat Pentru `ngos`

### P1. Shell semantic dual-surface

Shell-ul trebuie să aibă două moduri de ieșire pentru aceeași comandă:

- `semantic output`
- `rendered text output`

Exemplu conceptual:

- `mounts` produce listă de record-uri
- `mounts | where propagation_mode == "shared"` filtrează semantic
- `mounts --view table` doar schimbă renderizarea

### P2. Command grammar mică, compoziție mare

Nu urmărim o gramatică gigantică și opacă.
Mai bine:

- comenzi mici
- blocuri/pipe-uri/guard-uri clare
- workflow-uri compuse din agenți expliciți

### P3. Obiecte shell de prim rang

Shell-ul `ngos` ar trebui să aibă tipuri native cel puțin pentru:

- `Path`
- `Fd`
- `Pid`
- `Mount`
- `Resource`
- `Contract`
- `Queue`
- `Signal`
- `Stat`
- `ErrorRecord`

### P4. Refusal-first UX

Shell-ul trebuie să exceleze la operații refuzate, nu doar la happy path.

Mesajul ideal:

- spune obiectul
- spune regula
- spune motivul
- spune ce poți face mai departe

### P5. Help și introspecție integrate în runtime

Cel mai bun shell nu trimite utilizatorul în afara lui pentru a înțelege ce se întâmplă.

Trebuie să existe:

- `help <command>`
- `describe <command>`
- `why-failed`
- `show last-error`
- `explain <object>`

## Ce Ar Trebui Să Evităm

- compatibilitate POSIX mare ca obiectiv dominant
- shell monolitic care înghite toată logica în `lib.rs`
- text parsing ca mecanism principal între comenzi
- mod AI ambiguizat cu comanda normală
- opțiuni multe care repară UX-ul prost în loc să-l înlocuiască
- script semantics diferite de interactive semantics

## Backlog Prioritizat

### Nivel 1: fundație

- model unificat de `ShellValue`
- model unificat de `ShellError`
- pipeline semantic între builtins
- rendereri text separați de valori

### Nivel 2: UX mare

- completări contextuale reale
- help semantic și `describe`
- history cu structură, nu doar linii brute
- refusal messages și recovery hints standardizate

### Nivel 3: scripting și workflow

- blocuri și control flow cu erori predictibile
- module mici pentru comenzi
- workflow agents compozabili și observabili

### Nivel 4: introspecție și ops

- explicații pentru procese, mount-uri, fd-uri și resurse
- tracing local din shell pentru ultima mutație semantică
- replay / explain pentru workflow-uri

### Nivel 5: AI/NL, doar cu guardrails

- mod separat explicit
- plan preview
- confirmare pe mutații
- politică de risc
- jurnal de acțiuni AI

## Standardul De Excelență Pentru `ngos`

Vom putea spune că avem un shell excepțional când:

- este mai ușor de folosit decât shell-urile clasice
- este mai sigur decât shell-urile clasice
- este mai expresiv pe obiectele `ngos` decât un shell generic
- este bun atât interactiv, cât și pentru scripting și proof orchestration
- tratează refusal, recovery și observability ca funcții de bază, nu ca extensii
- orice strat AI este subordonat shell-ului semantic, nu invers

## Concluzie

Cel mai bun shell posibil pentru `ngos` nu înseamnă:

- `fish clone`
- `Nushell clone`
- `PowerShell clone`
- `AI shell first`

Înseamnă:

- UX bun implicit
- obiecte și pipeline-uri semantice
- error model explicit
- workflow orchestration curată
- introspecție profundă
- guardrails stricte

Formula scurtă:

`fish UX + Nushell/PowerShell data model + Elvish/YSH semantics + NaSh guardrails`, traduse original în termenii `ngos`.
