# `ngos` Shell For Coding Specification

Acest document definește cum trebuie să arate `ngos shell` dacă vrem să fie excelent pentru coding și development, nu doar pentru operare de sistem.

## Obiectiv

Shell-ul trebuie să fie capabil să devină unul dintre cele mai bune medii de lucru pentru coding în ecosistemul `ngos`, prin:

- navigare rapidă în workspace
- build și test workflows curate
- diagnostică structurată
- compoziție semantică a task-urilor de dezvoltare
- explainability pentru cod, patch-uri și failures

## Principiu

Nu vrem un shell care doar pornește tool-uri externe.
Vrem un shell care înțelege proiectul, build-ul, testele și artefactele ca obiecte semantice.

## Tipuri De Prim Rang Pentru Coding

Pe lângă tipurile generale din
[ngos-shell-language-spec.md](C:/Users/pocri/OneDrive/Desktop/experiment/docs/ngos-shell-language-spec.md),
pentru coding shell-ul trebuie să aibă:

- `Workspace`
- `Project`
- `Package`
- `ModuleRef`
- `SourceFile`
- `Symbol`
- `Reference`
- `Location`
- `Span`
- `Diagnostic`
- `DiagnosticSet`
- `BuildTarget`
- `BuildArtifact`
- `TestCase`
- `TestSuite`
- `Benchmark`
- `Patch`
- `Diff`
- `ChangeSet`
- `SearchResult`
- `Traceback`
- `TaskRun`
- `CommandPlan`

## Command Families Pentru Coding

### Workspace Navigation

- `workspace`
- `project`
- `packages`
- `modules`
- `files`
- `symbols`
- `refs`
- `outline`

Exemple conceptuale:

```ngsh
workspace info
packages | where name ~= "boot"
symbols --file boot-x86_64/src/user_syscall.rs
refs BootVfs.rename
outline userland-native/src/lib.rs
```

### Search Semantics

Nu doar `grep`.
Shell-ul trebuie să distingă:

- text search
- symbol search
- path search
- diagnostic search

Comenzi:

- `find text`
- `find symbol`
- `find path`
- `find diagnostic`

### Build Model

Build-ul nu trebuie văzut doar ca output brut de terminal.

Tipuri:

- `BuildTarget`
- `BuildArtifact`
- `Diagnostic`

Comenzi:

- `build`
- `build plan`
- `build artifacts`
- `build diagnostics`

Exemple:

```ngsh
build workspace
build package ngos-boot-x86_64
build diagnostics | where severity == "error"
```

### Test Model

Testele trebuie expuse semantic:

- `test list`
- `test run`
- `test failures`
- `test explain`
- `test watch`

Exemple:

```ngsh
test list --package ngos-userland-native
test run native_shell_runs_vfs_smoke_command_and_reports_vfs_markers
test failures | select package name reason
```

### Bench And Perf

Comenzi:

- `bench`
- `bench compare`
- `perf trace`
- `perf regressions`

### Diff And Patch

Shell-ul trebuie să știe patch-uri și schimbări.

Comenzi:

- `diff`
- `patch preview`
- `changes`
- `changes explain`

Exemple:

```ngsh
diff working
changes | where file ~= "user_syscall"
patch preview last
```

## Diagnostică Structurată

Unul dintre cele mai importante avantaje.

Un `Diagnostic` trebuie să includă:

- `severity`
- `message`
- `code`
- `file`
- `span`
- `related`
- `suggestion`
- `tool`

Shell-ul trebuie să poată:

- filtra diagnostice
- grupa pe fișier sau severitate
- deschide contextul
- explica sugestii

Exemple:

```ngsh
build diagnostics | where severity == "error"
build diagnostics | group-by file
diagnostic explain 12
```

## Error UX Pentru Coding

Shell-ul trebuie să fie mai bun decât terminalele clasice la feedback de dezvoltare.

Pentru un eșec de build/test trebuie să poată spune:

- ce a eșuat
- unde
- dacă este reproducibil
- care e frontiera minimă relevantă
- care e următorul pas rezonabil

## Code Explainability

Comenzi dorite:

- `explain symbol`
- `explain file`
- `explain error`
- `explain diff`
- `explain test-failure`

Exemple:

```ngsh
explain symbol BootVfs.rename
explain file boot-x86_64/src/user_syscall.rs
explain test-failure last
```

## Coding Workflows

Shell-ul trebuie să poată modela workflows frecvente:

- edit -> build -> test -> explain
- search -> patch -> verify
- repro -> isolate -> confirm -> close

Exemplu conceptual:

```ngsh
workflow "tighten-vfs-front" {
  step search { find symbol BootVfs.rename }
  step build { build package ngos-boot-x86_64 }
  step test { test run boot_vfs_*rename* }
  step proof { run qemu-proof vfs }
}
```

## Live Dev Features

Pentru a fi foarte bun pe coding, shell-ul trebuie să aibă:

- `watch build`
- `watch test`
- `watch diagnostics`
- `watch file`

Exemple:

```ngsh
watch test --package ngos-userland-native
watch build --workspace
```

## Safe Automation

Shell-ul de coding nu trebuie să fie doar puternic, ci sigur.

Necesită:

- patch preview înainte de aplicare
- command plan pentru mutații mari
- rollback hooks unde e posibil
- explainability pentru comenzi generate sau compuse

## Integration With External Tooling

Comenzile externe rămân utile:

- `cargo`
- `git`
- `rg`
- `qemu proof scripts`

Dar shell-ul trebuie să le ridice semantic:

- parsează diagnostice
- parsează test failure
- parsează artefacte
- parsează status de proof

Nu vrem doar:

- shell care rulează `cargo test`

Vrem:

- shell care înțelege ce a ieșit din `cargo test`

## AI For Coding

Dacă vom adăuga AI pentru coding, trebuie să fie strict subordonat shell-ului semantic.

Comenzi conceptuale:

- `ai code-plan`
- `ai explain-error`
- `ai draft-patch`
- `ai review-diff`

Guardrails:

- nu aplică direct fără preview
- explică fișierele atinse
- explică riscul
- arată difful
- cere confirmare pentru mutații reale

## Ce Ar Face Shell-ul Cu Adevărat Excepțional Pentru Coding

### 1. Navigare semantică

Mai bun decât text search brut:

- symbol aware
- module aware
- package aware

### 2. Diagnostică tratată ca date

Mai bun decât compilatoare care scuipă doar text:

- filtrare
- grouping
- explain
- actionability

### 3. Workflows directe

Mai bun decât secvențe manuale repetate:

- build/test/proof orchestration
- failure localization
- reproducibility helpers

### 4. Explainability nativă

Mai bun decât “a citit logul și atât”:

- explain error
- explain diff
- explain subsystem state

### 5. Obiecte `ngos`

Aici `ngos shell` poate depăși shell-urile generale:

- `mount`
- `fd`
- `resource`
- `contract`
- `queue`
- `signal`
- `workflow`

## Primele Fronturi De Implementare Pentru Coding

### Front 1

`build diagnostics` și `test failures` ca obiecte semantice

### Front 2

`find symbol`, `refs`, `outline`

### Front 3

`changes`, `diff`, `patch preview`

### Front 4

`explain error`, `explain test-failure`, `explain diff`

### Front 5

`watch build`, `watch test`, `watch diagnostics`

## Verdict

`ngos shell` poate deveni unul dintre cele mai bune shell-uri pentru coding dacă:

- tratează development-ul ca subsistem de prim rang
- nu rămâne la model text-only
- transformă build/test/diagnostic/diff în obiecte
- face explainability și workflows native

Dacă facem doar:

- prompt frumos
- completări
- wrappere peste `cargo`

nu va fi cel mai bun.

Dacă facem:

- workspace semantics
- diagnostics as data
- coding workflows
- explainability
- guardrails

atunci are șanse reale să fie excepțional în ecosistemul `ngos`.
