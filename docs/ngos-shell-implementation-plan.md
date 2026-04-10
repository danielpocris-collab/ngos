# `ngos` Shell Implementation Plan

Acest document transformă direcția din
[ngos-shell-best-in-class-roadmap.md](C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-shell-best-in-class-roadmap.md)
în fronturi executabile.

## Obiectiv

Să construim un shell `ngos` care este:

- excelent interactiv
- sigur semantic
- bun pentru scripting
- capabil de orchestration pentru subsistemele `ngos`
- observabil și explicabil

## Principiu De Execuție

Nu implementăm un shell nou prin rewrite complet.
Îl împingem înainte prin fronturi verticale reale în `userland-native`, fiecare cu:

- logică reală
- integrare în shell-ul existent
- output observabil
- refusal path
- recovery sau cleanup unde are sens
- test local și dovadă `QEMU` când frontul devine strategic

## Nivel 1: Valori Și Pipeline Semantic

### 1.1 `ShellValue`

Introducem un model unificat de valori de shell.

Tipuri minime:

- `Null`
- `Bool`
- `Int`
- `String`
- `Bytes`
- `List`
- `Record`
- `Path`
- `Error`

Front închis doar dacă:

- builtins noi pot produce `ShellValue`
- pipeline-ul poate transporta valori fără reserializare text
- renderizarea text rămâne compatibilă pentru comenzi interactive

### 1.2 separare `value` vs `view`

Aceeași comandă trebuie să poată:

- produce valoare semantică
- produce text randat

Primele comenzi candidate:

- `ps`
- `stat`
- `statfs`
- `mounts`
- `resources`
- `contracts`

### 1.3 filtre și select semantic

Adăugăm compoziție de bază peste valori:

- `where`
- `select`
- `count`
- `first`
- `get`

## Nivel 2: Error Model Și Refusal UX

### 2.1 `ShellError`

Introducem un tip unificat de eroare:

- `kind`
- `message`
- `path/object`
- `errno` sau motiv semantic
- `recoverable`
- `hint`

### 2.2 refusal-first reporting

Toate builtins majore trebuie să raporteze:

- ce a fost refuzat
- de ce
- pe ce obiect
- dacă există pas de recovery

Fronturi candidate:

- `open`
- `read`
- `write`
- `rename`
- `unlink`
- `mount`
- `unmount`
- `signal`
- `resource/contract ops`

### 2.3 stare explicită a ultimei erori

Adăugăm:

- `show last-error`
- `why-failed`

## Nivel 3: UX Interactiv Best-In-Class

### 3.1 completări contextuale

Completările trebuie să știe:

- tipul comenzii
- path-uri
- obiecte `ngos`
- mount-uri
- procese
- resurse
- contracte

### 3.2 help semantic

Adăugăm:

- `help <command>`
- `describe <command>`
- `examples <command>`

### 3.3 history structurat

History-ul nu trebuie să fie doar text brut.

Trebuie să păstreze:

- comanda
- status
- timestamp
- obiectele afectate când sunt cunoscute

## Nivel 4: Scripting Și Workflow

### 4.1 limbaj de script coerent

Stabilim un subset clar pentru:

- variabile
- pipe
- guard-uri
- blocuri
- control flow minim

### 4.2 workflow agents ca primitive de shell

Workflow-urile existente trebuie împinse spre un model uniform:

- intrări clare
- ieșiri clare
- refusal path
- cleanup path

### 4.3 replay / explain

Adăugăm posibilitatea de a explica:

- ce a făcut un workflow
- unde a eșuat
- ce mutații a produs

## Nivel 5: Introspecție Și Operare

### 5.1 explain-object

Comenzi candidate:

- `explain path <x>`
- `explain fd <x>`
- `explain pid <x>`
- `explain mount <x>`
- `explain resource <x>`

### 5.2 shell observability

Trebuie să existe:

- explain pentru ultima mutație
- explain pentru ultima refusal
- status pentru workflow-uri active

### 5.3 diagnostic views

Shell-ul trebuie să poată randa curat:

- queue state
- watch state
- pending signals
- mount propagation
- descriptor state

## Nivel 6: AI / NL Cu Guardrails

Acest nivel este permis doar după ce shell-ul semantic de bază este bun.

### 6.1 mod separat

Nu amestecăm comanda normală cu NL.

Exemplu conceptual:

- `ai plan "show me blocked mounts"`
- nu interpretare ambiguă a oricărei linii ca prompt AI

### 6.2 plan preview

AI trebuie să producă:

- plan
- comenzi propuse
- obiecte afectate
- risc

### 6.3 confirmare și jurnal

Pentru mutații:

- preview
- confirmare explicită
- jurnal al acțiunilor AI

## Ordine Recomandată

1. `ShellValue`
2. `ShellError`
3. `value/view split` pentru builtins de introspecție
4. filtre semantice mici
5. help / describe / last-error
6. completări contextuale
7. scripting model minim coerent
8. workflow explain / replay
9. shell observability dedicated proof
10. AI mode separat, doar după restul

## Primele Fronturi Concret Recomandate

### Front A

`ps`, `mounts`, `resources`, `contracts` să producă record-uri semantice, cu render text separat

### Front B

`ShellError` + `show last-error` + refusal hints

### Front C

`where/select/get` peste rezultate semantice

### Front D

`help/describe/examples` pentru builtins majore

### Front E

proof dedicat `shell` pe `QEMU`

## Definiția De Excelență

Shell-ul va fi într-o zonă “best-in-class” pentru `ngos` când:

- utilizatorul poate opera subsistemele majore fără parsare fragilă de text
- refusal-urile sunt mai explicabile decât în shell-urile clasice
- workflow-urile sunt observabile și reproductibile
- interactiv și scripting folosesc același model semantic
- calea reală `QEMU` are proof dedicat de shell
