# `ngos` Shell Language Specification

Acest document definește ținta completă pentru limbajul de shell și scripting `ngos`.
Nu este un set minim.
Este specificația de referință pentru un shell care vrea să fie excelent atât interactiv, cât și ca limbaj.

## Obiectiv

`ngos shell` trebuie să fie:

- un shell interactiv foarte bun
- un limbaj de scripting serios
- un control plane semantic pentru subsistemele `ngos`
- o suprafață sigură pentru refusal, recovery și observabilitate

## Principii

- datele nu sunt doar text
- erorile nu sunt doar coduri de ieșire
- interactive și scripting folosesc același model semantic
- pipeline-urile transportă valori, nu doar linii
- renderizarea este separată de valoarea semantică
- efectele trebuie să fie observabile și explicabile

## Modelul De Valoare

Toate expresiile produc un `ShellValue`.

## Tipuri Primitive

- `Null`
- `Bool`
- `Int`
- `Float`
- `Decimal`
- `String`
- `Bytes`
- `Char`
- `Size`
- `Duration`
- `Timestamp`
- `ExitStatus`

## Tipuri Compuse

- `List<T>`
- `Set<T>`
- `Map<K, V>`
- `Record`
- `Tuple`
- `Range`
- `Option<T>`
- `Result<T, E>`
- `Stream<T>`
- `Table<Row>`

## Tipuri De Program

- `Command`
- `Closure`
- `Function`
- `Module`
- `Pattern`
- `Regex`
- `Glob`

## Tipuri Shell / OS De Prim Rang

- `Path`
- `Inode`
- `Fd`
- `Pid`
- `Tid`
- `Mount`
- `MountId`
- `PathStat`
- `FsStat`
- `Process`
- `ProcessState`
- `Job`
- `Signal`
- `SignalSet`
- `Queue`
- `QueueEvent`
- `Watch`
- `Resource`
- `ResourceClaim`
- `Contract`
- `Domain`
- `Capability`
- `Policy`
- `Label`
- `VmRegion`
- `Device`
- `Driver`
- `Socket`
- `Interface`
- `Route`
- `GpuBuffer`
- `AudioBuffer`
- `InputEvent`
- `CompatHandle`
- `WasmModule`
- `WorkflowRun`
- `TraceRecord`

## Tipuri De Eroare

Erorile sunt și ele valori.

`ShellError` trebuie să includă:

- `kind`
- `message`
- `errno`
- `path`
- `object_kind`
- `object_id`
- `recoverable`
- `hint`
- `cause`
- `trace`

Categorii de bază:

- `ParseError`
- `TypeError`
- `NameError`
- `PathError`
- `PermissionError`
- `AccessError`
- `ConflictError`
- `BusyError`
- `NotFoundError`
- `ValidationError`
- `RuntimeError`
- `IoError`
- `WorkflowError`
- `ExternalError`

## Literali

Suport complet pentru:

- numere întregi
- numere flotante
- zecimale exacte
- string-uri
- bytes
- liste
- tuple
- record-uri
- intervale
- regex
- glob
- path literals

Exemple conceptuale:

```ngsh
42
3.14
12.50d
"hello"
b"abc"
[1, 2, 3]
(1, "x", true)
{ pid: 1, name: "init" }
1..10
re"[a-z]+"
glob"/vfs/**/*.txt"
path"/vfs/bin/app"
```

## Variabile Și Binding

Binding-urile implicite sunt imutabile.

Forme:

- `let`
- `mut`
- `const`
- `export`

Exemple:

```ngsh
let pid = 1
mut count = 0
const root = path"/vfs"
export PATH = [path"/bin", path"/usr/bin"]
```

## Destructuring

Suport pentru:

- list destructuring
- tuple destructuring
- record destructuring

Exemple:

```ngsh
let [a, b, c] = [1, 2, 3]
let (pid, state) = (1, "Running")
let { pid, name } = ps | first
```

## Modelul De Execuție

Unități de execuție:

- expresie
- comandă builtin
- comandă externă
- pipeline
- bloc
- workflow

Fiecare unitate produce:

- valoare
- status
- eventual efecte
- eventual error record

## Evaluare

Reguli:

- evaluare stânga-dreapta
- side effects doar unde sunt explicite
- scurtcircuit pentru booleans și guard-uri
- eșecul nu este ascuns implicit

## Pipeline Semantic

Pipeline-ul transmite `ShellValue`, nu doar text.

Moduri:

- `semantic pipeline`
- `text pipeline`
- `bytes pipeline`

Reguli:

- builtins `ngos` consumă semantic dacă pot
- comenzi externe consumă text sau bytes prin adaptori expliciți
- renderizarea text nu pierde valoarea internă decât dacă utilizatorul cere

Exemple:

```ngsh
mounts | where propagation_mode == "shared"
ps | select pid name state
resources | where state == "queued" | count
stat path"/vfs/bin/app" | get inode
```

## Adaptori De Pipeline

Conversii explicite:

- `as text`
- `as bytes`
- `as table`
- `as json`
- `as record`
- `render`

Exemple:

```ngsh
mounts | as table
ps | as json
cat path"/vfs/bin/app" | as bytes
```

## Sintaxa De Comandă

Forme principale:

- apel simplu
- apel cu named args
- apel cu flags
- apel cu subcommand
- pipeline
- bloc

Exemple:

```ngsh
stat /vfs/bin/app
mounts --view table
resource claim --contract 12
ps | where state == "Running"
```

## Control Flow

Forme necesare:

- `if`
- `else`
- `match`
- `for`
- `while`
- `loop`
- `break`
- `continue`
- `return`

Exemple:

```ngsh
if (exists /vfs/bin/app) {
  stat /vfs/bin/app
} else {
  echo "missing"
}

match (statfs /vfs).read_only {
  true => echo "ro"
  false => echo "rw"
}
```

## Funcții

Forme:

- funcții declarate
- closures
- named parameters
- optional parameters
- typed parameters
- typed returns

Exemplu:

```ngsh
fn mount_is_shared(m: Mount) -> Bool {
  m.propagation_mode == "shared"
}

let only_shared = |items| {
  items | where propagation_mode == "shared"
}
```

## Module

Suport pentru:

- `module`
- `use`
- `pub`
- namespace explicit

Exemplu:

```ngsh
module storage {
  pub fn summary() {
    mounts | where kind == "storage"
  }
}

use storage summary
```

## Pattern Matching

Trebuie să funcționeze pe:

- primitive
- tuple
- list
- record
- enum-like records
- error kinds

Exemplu:

```ngsh
match (show last-error) {
  Error { kind: "PermissionError", path } => echo $"permission denied on ($path)"
  Error { kind } => echo $"error: ($kind)"
  _ => echo "ok"
}
```

## Error Handling

Mecanisme:

- `try`
- `catch`
- `finally`
- `defer`
- `?`
- `ensure`

Exemple:

```ngsh
let info = inspect_mount /vfs/mount-shared?

try {
  unmount /vfs/mount-shared
} catch err {
  explain err
} finally {
  echo "done"
}
```

## Refusal Semantics

Un shell bun trebuie să facă distincție între:

- command failed because of bug
- command refused because policy forbids it
- command blocked because object state conflicts
- command incomplete because resource unavailable

Acestea trebuie reflectate în `ShellError.kind` și în `ExitStatus`.

## Exit Status

Nu folosim doar `0` sau `1`.

`ExitStatus` trebuie să distingă:

- `Success`
- `Failure`
- `Refused`
- `Blocked`
- `Conflict`
- `NotFound`
- `Timeout`
- `Canceled`

Pentru compat extern, se poate mapa la cod numeric, dar shell-ul intern trebuie să păstreze forma semantică.

## Jobs, Async Și Await

Funcționalități:

- `spawn`
- `job list`
- `job wait`
- `await`
- `cancel`
- `timeout`

Exemplu:

```ngsh
let job = spawn { workflow run "mirror-tree /vfs/a /vfs/b 8" }
await job
```

## Workflow Blocks

Shell-ul trebuie să aibă un model dedicat pentru workflow-uri reale.

Formă conceptuală:

```ngsh
workflow "vfs-upgrade" {
  step create_root { mkdir /vfs/new }
  step mirror { mirror-tree /vfs/live /vfs/new 8 }
  step switch { mount switch /vfs/live /vfs/new }
  recover {
    unmount /vfs/new
  }
}
```

## Explainability

Comenzi necesare:

- `help`
- `describe`
- `examples`
- `show last-error`
- `why-failed`
- `explain`
- `trace last`

Exemple:

```ngsh
describe mounts
why-failed
explain mount /vfs/mount-shared
trace last
```

## Interactive UX

Shell-ul trebuie să includă:

- completări contextuale
- syntax highlighting
- hints
- history structurat
- help inline
- preview pentru operații periculoase

## Quoting Și Interpolation

Reguli:

- fără word splitting implicit periculos
- interpolarea este explicită și predictibilă
- bytes și text rămân distincte

Forme:

- string simplu
- string raw
- string interpolat
- bytes literal

## External Commands

Comenzile externe sunt suportate, dar tratate ca frontieră de tip.

Reguli:

- input extern primește text sau bytes prin adaptori
- output extern este capturat ca `Text`, `Bytes` sau `ExternalRecord`
- shell-ul nu se degradează intern la model text-only doar pentru compatibilitate

## Standard Library De Shell

Module minime dorite:

- `core`
- `fs`
- `proc`
- `mount`
- `resource`
- `contract`
- `queue`
- `signal`
- `net`
- `gpu`
- `audio`
- `input`
- `compat`
- `workflow`
- `render`
- `debug`

## Builtins Majore

Builtin-uri strategice:

- `ps`
- `jobs`
- `env`
- `history`
- `aliases`
- `vars`
- `stat`
- `statfs`
- `list`
- `open`
- `read`
- `write`
- `append`
- `truncate`
- `mkdir`
- `mkfile`
- `symlink`
- `link`
- `rename`
- `unlink`
- `mounts`
- `mount`
- `unmount`
- `resources`
- `contracts`
- `domains`
- `queues`
- `signals`
- `watch`
- `workflow`
- `explain`

## Compatibilitate

Compatibilitatea POSIX nu este obiectivul central.

Compat poate exista:

- la nivel de command names familiare
- la nivel de adaptori pentru tool-uri externe
- la nivel de script import limitat

Dar nu trebuie să dicteze:

- modelul de tipuri
- modelul de erori
- modelul de quoting
- semantica pipeline-urilor

## AI / Natural Language

Dacă este introdus, trebuie să fie mod separat:

- `ai plan`
- `ai explain`
- `ai draft`

Nu:

- auto-executare ambiguă
- transformarea oricărei linii în prompt AI

Guardrails obligatorii:

- preview
- confirmare pentru mutații
- jurnal
- risc și policy check

## Definiția De Excelență

Specificația este atinsă când shell-ul:

- poate opera subsistemele `ngos` prin obiecte semantice
- poate face scripting fără fragilitate de text
- are erori mai clare decât shell-urile clasice
- oferă introspecție și explain native
- păstrează aceeași semantică între interactive și script

## Concluzie

`ngos shell` nu trebuie proiectat ca:

- shell text-only
- limbaj de scripting separat de interactive mode
- suprafață de compatibilitate dominantă

Trebuie proiectat ca:

- runtime semantic interactiv
- limbaj de control pentru OS-ul `ngos`
- suprafață sigură, explicabilă și foarte expresivă
