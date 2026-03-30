# Graphics Render Engine Plan

## Scope

Acest document fixează planul de execuție pentru subsistemul `graphics/render engine`.

Nu este un roadmap general al OS-ului.
Este documentul de execuție pentru frontul mare de rendering cerut acum.

Conform regulilor repo-ului:

- `graphics/render engine` nu poate fi redus la UI polish
- nu poate fi închis prin `host-runtime` only
- nu poate fi tratat ca demo renderer
- trebuie împins pe truth path-ul real:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`
  - hardware real

## Current State

`Subsystem graphics/render engine is not yet closed.`

Ce există acum:

- prezentare de frame reală pe path-ul GPU curent
- `FrameScript` ca limbaj 2D proprietar
- primitive:
  - `clear`
  - `line`
  - `rect`
  - `sprite`
  - `blit`
  - `gradient-rect`
  - `rounded-rect`
  - `shadow-rect`
- fronturi reale care folosesc suprafața:
  - boot desktop
  - `aura-browser`
  - game graphics lane

Ce lipsește încă:

- scene model real
- compositor real
- effect pipeline real
- 3D engine real
- observabilitate și closure cap-coadă pentru aceste familii

## Families

Familiile în scope pentru `graphics/render engine` sunt:

- `render_command_agent`
- `frame_script_agent`
- `surface_composition_agent`
- `window_compositor_agent`
- `effect_pipeline_agent`
- `scene_graph_agent`
- `camera_agent`
- `mesh_material_agent`
- `lighting_agent`
- `temporal_animation_agent`
- `presentation_agent`
- `render_observability_agent`

## Execution Order

Ordinea corectă de execuție este:

1. `Render Command Model`
2. `2D Composition Engine`
3. `Effects Pipeline`
4. `Scene Graph`
5. `3D Render Engine`
6. `QEMU Proof`
7. `Hardware Proof`

## Phase 1: Render Command Model

### Objective

Limbajul de randare trebuie să înceteze să fie doar un translator de primitive plate și să devină un contract semantic stabil pentru rendering.

### Deliveries

- extinderea `DrawOp` și `FrameScript` cu primitive și passes reale
- separarea dintre:
  - geometry ops
  - composition ops
  - effect ops
  - presentation ops
- validare și encoding pentru noile familii
- observabilitate pentru op classes, op count, frame passes și profile

### Closure Requirements

- noile comenzi produc efect real în runtime
- sunt expuse prin API-ul real existent
- sunt observabile în payload și în runtime reports
- au refusal path pentru argumente invalide

## Phase 2: 2D Composition Engine

### Objective

UI-ul trebuie să fie compus de un motor de suprafețe, nu doar de liste de dreptunghiuri.

### Deliveries

- model de suprafețe/ferestre
- stacking order real
- clipping și occlusion
- chrome semantic
- rounded windows și shadows tratate ca primitive de compoziție
- separarea dintre:
  - background
  - panels
  - windows
  - dock/taskbar
  - overlays

### Closure Requirements

- compoziția este deterministă și observabilă
- ordinea de stack este inspectabilă
- refusal path există pentru surface contract invalid
- final state este expus după present

## Phase 3: Effects Pipeline

### Objective

Efectele premium trebuie mutate din simulări vizuale în passes reale.

### Deliveries

- gradient pipeline real
- shadow pipeline real
- translucency model real
- backdrop/frosted layer model
- temporal accent/effect state

### Closure Requirements

- efectele au semantică proprie, nu hacks locale
- sunt compuse prin pipeline, nu ad-hoc în fiecare app
- există refusal/error path pentru effect configuration invalidă

## Phase 4: Scene Graph

### Objective

Pentru 3D real, UI și scenele au nevoie de o structură semantică de scenă.

### Deliveries

- noduri de scenă
- transform hierarchy
- camere
- view/projection semantics
- scene submission contract

### Closure Requirements

- scenele pot fi construite, mutate, inspectate și prezentate
- state-ul scenei este observabil
- failure path există pentru graph invalid

## Phase 5: 3D Render Engine

### Objective

Acesta este punctul în care sistemul devine motor 3D real, nu doar 2D premium.

### Deliveries

- mesh submission
- material model
- light model
- camera-driven frame generation
- render passes pentru geometry, lighting și present

### Closure Requirements

- un front 3D real rulează cap-coadă
- mesh/material/light/camera sunt toate observabile
- refusal path pentru asset/material invalid
- final frame state și pass results sunt inspectabile

## Phase 6: QEMU Proof

### Objective

`QEMU` este primul truth surface acceptabil pentru subsistem.

### Required Proofs

- 2D composition proof
- effects proof
- scene proof
- 3D proof
- refusal/error proof
- recovery/release proof unde subsistemul permite

### Done Means

- `graphics/render engine` are execuție observabilă pe `QEMU`, nu doar compilează

## Phase 7: Hardware Proof

### Objective

Subsystemul nu este global închis fără frontul hardware real.

### Required Proofs

- present real pe hardware
- stability proof
- final observable state după present
- degradare/refusal controlată dacă hardware path nu poate satisface cererea

### Done Means

- `graphics/render engine` este dovedit pe hardware real, nu doar pe `QEMU`

## Non-Goals During This Front

În timpul acestui front nu trebuie acceptate ca substitut:

- mai multe mockups HTML
- showcase-only windows
- “fake 3D” din multe dreptunghiuri fără subsistem nou
- un renderer separat care ocolește stack-ul `ngos`
- un “temporary engine” care va fi rescris ulterior

## Immediate Next Work

Ordinea imediată corectă este:

1. închiderea `Render Command Model` ca families noi
2. `2D Composition Engine` real peste primitivele noi
3. abia după aceea `Scene Graph`
4. apoi `3D Render Engine`
