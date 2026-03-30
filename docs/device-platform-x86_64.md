# x86_64 Device Platform Model

Acest document fixeaza modelul arhitectural pentru substratul de device-uri reale din `ngos`.
Este document de autoritate pentru:

- `platform-hal`
- `platform-x86_64`
- integrarea din `boot-x86_64`
- relatia cu `kernel-core`

Scopul lui nu este sa permita "primul driver", ci sa inchida modelul de platforma astfel incat drivere reale multiple sa poata fi adaugate fara redesign structural.

## Obiective

Substratul trebuie sa acopere cap-coada:

- discovery generic de bus si device
- config-space access
- lifecycle complet pentru BAR-uri
- mapare si unmap MMIO
- model unificat pentru interrupts
- model explicit pentru DMA
- ownership si lifecycle verificabile

Substratul nu are voie sa contina:

- API-uri create pentru un singur device
- acces direct la registre hardware din afara HAL/platform
- pseudo-DMA peste heap normal
- semantici implicite sau ascunse

## Separarea pe straturi

### `platform-hal`

`platform-hal` defineste contractele abstracte si tipurile stabile. Nu contine cod specific `x86_64`, `PCI`, `e1000`, `virtio` sau alt device concret.

Responsabilitati:

- modelul generic de bus/device/function/resource
- tipuri de config-space access
- tipuri si lifecycle pentru BAR-uri
- semantici pentru MMIO mapping
- semantici pentru interrupts
- semantici pentru DMA memory
- contracte pentru enumerare, claim/release si map/unmap

### `platform-x86_64`

`platform-x86_64` implementeaza contractele HAL pentru hardware `x86_64`.

Responsabilitati:

- scanare PCI prin config-space access
- decodare BAR
- mapare MMIO in spatiul kernel
- infrastructura de interrupt routing `x86_64`
- alocare si mapare DMA-capable memory
- snapshot si introspectie a platformei hardware

Nu contine logica de driver.

### `boot-x86_64`

`boot-x86_64` aduce platforma in starea in care device substrate-ul poate functiona.

Responsabilitati:

- initializare paging/heap/physical allocator
- initializare interrupt controller de baza
- initializare registry de interrupt dispatch
- activare platform services pentru enumerare/device access

Nu contine driver logic si nu expune shortcut-uri pentru un singur device.

### `kernel-core`

`kernel-core` consuma doar contractele HAL si obiectele rezultate din platform layer.

Responsabilitati:

- modelul intern `ngos` de devices/resources/contracts
- lifecycle de driver si binding
- folosirea bufferelor DMA si a resurselor MMIO/interrupt prin contractele HAL

`kernel-core` nu are voie sa faca:

- config-space reads/writes direct
- MMIO map direct
- interrupt routing direct
- alocare DMA direct in afara HAL

## Modelul generic de discovery

Discovery-ul este generic si nu presupune PCI ca singur bus.

Entitati:

- `BusKind`
  - `Pci`
  - `PlatformMmio`
  - `Virtual`
- `DeviceLocator`
  - identitate stabila a unui device in cadrul platformei
- `BusAddress`
  - adresa specifica bus-ului
  - pentru PCI: `segment/bus/device/function`
- `DeviceClass`
  - clasa functionala generica vazuta de kernel
  - `Network`, `Storage`, `Display`, `Bridge`, `Input`, `Other`
- `DeviceIdentity`
  - vendor/device/subsystem/class/revision
- `DeviceRecord`
  - locator
  - bus kind
  - address
  - identity
  - lista de resources
  - interrupt capabilities

Modelul de enumerare:

1. platforma descopera bus-uri
2. fiecare bus produce `DeviceRecord`
3. fiecare `DeviceRecord` este stabil in durata de viata a boot-ului
4. driver model-ul din kernel consuma `DeviceRecord`, nu scaneaza hardware-ul direct

Pentru PCI, `DeviceRecord` este alimentat de scanarea `segment/bus/device/function`, dar contractul ramane generic.

## Config-space access

Config-space access este definit generic prin:

- `ConfigSpaceKind`
  - `Pci`
  - extensibil pentru alte bus-uri
- `ConfigWidth`
  - `U8`, `U16`, `U32`
- `ConfigOffset`
  - offset in config space

Contract:

- read/write explicit pe `DeviceLocator`
- validare de aliniere si width
- erori explicite pentru acces invalid
- fara expunere a detaliilor mecanismului de acces catre kernel

Pentru PCI, implementarea concreta poate folosi:

- legacy config IO ports
- ECAM/MMCONFIG

Dar semantica expusa ramane una singura.

## BAR model

BAR-ul este tratat ca resursa distincta, cu lifecycle separat.

Tipuri:

- `BarKind`
  - `Memory32`
  - `Memory64`
  - `IoPort`
- `BarFlags`
  - `prefetchable`
  - `cacheable`
  - `read_only`

Structuri:

- `BarId`
  - identitate stabila in cadrul unui `DeviceLocator`
- `BarInfo`
  - `id`
  - `kind`
  - `base`
  - `size`
  - `flags`

Lifecycle:

1. `Discovered`
2. `Claimed`
3. `Mapped`
4. `Unmapped`
5. `Released`

Reguli:

- BAR-ul nu poate fi mapat fara `claim`
- BAR-ul mapat nu poate fi claim-uit din nou
- un BAR poate avea mai multe mapari doar daca semantica o permite explicit; implicit mapping-ul este exclusiv
- BAR `IoPort` nu trece prin MMIO mapper

## MMIO mapping

MMIO este model separat de BAR, pentru ca aceeasi infrastructura trebuie sa suporte si device-uri `PlatformMmio`.

Tipuri:

- `MmioRegionId`
- `MmioPermissions`
  - `read`
  - `write`
- `MmioCachePolicy`
  - `Uncacheable`
  - `WriteCombining`
  - `WriteBack` doar cand hardware-ul o permite explicit
- `MmioMapping`
  - region id
  - virtual base
  - physical base
  - len
  - perms
  - cache policy

Reguli:

- maparea creeaza un handle explicit
- unmap elibereaza handle-ul si invalideaza accesul
- accesul la registre se face doar prin handle valid
- niciun driver nu pastreaza raw pointer in afara mapping lifecycle-ului

## Interrupt model

Interrupt-ul este modelat ca resursa routabila cu ownership explicit.

Tipuri:

- `InterruptKind`
  - `LegacyLine`
  - `Msi`
  - `Msix`
- `InterruptVector`
  - vector CPU instalat
- `InterruptRoute`
  - descrierea rutei reale
- `InterruptHandle`
  - ownership explicit al inregistrarii
- `InterruptTrigger`
  - `Edge`, `Level`
- `InterruptPolarity`
  - `High`, `Low`

Lifecycle:

1. `Discovered`
2. `Allocated`
3. `Registered`
4. `Enabled`
5. `Disabled`
6. `Released`

Contract:

- un device poate expune mai multe capability-uri de interrupt
- driverul cere o capacitate compatibila
- platforma aloca ruta reala si intoarce un `InterruptHandle`
- handlerul este inregistrat prin handle
- `acknowledge` este operatie explicita
- `enable/disable` sunt explicite

Semantica:

- `LegacyLine` trece prin PIC/IOAPIC routing
- `Msi` si `Msix` sunt tratate ca acelasi contract HAL cu capabilitati diferite
- kernelul nu vede detaliile LAPIC/MSI message format direct

## DMA model

DMA nu este heap normal si nu este doar "memorie fizica".

Tipuri:

- `DmaBufferId`
- `DmaDirection`
  - `ToDevice`
  - `FromDevice`
  - `Bidirectional`
- `DmaCoherency`
  - `Coherent`
  - `NonCoherent`
- `DmaConstraints`
  - alignment
  - max_address_bits
  - segment_boundary
  - contiguous
- `DmaBuffer`
  - id
  - cpu_virtual
  - device_address
  - len
  - direction
  - coherency

Lifecycle:

1. `Allocated`
2. `MappedForDevice`
3. `OwnedByCpu`
4. `OwnedByDevice`
5. `Released`

Sincronizare:

- `prepare_for_device`
  - flush/sync CPU writes
  - transfer ownership catre device
- `complete_from_device`
  - sync/invalidate dupa DMA de la device
  - transfer ownership catre CPU

Reguli:

- bufferul DMA nu poate fi accesat de CPU in mod arbitrar cand ownership-ul este la device
- bufferul DMA nu poate fi folosit dupa `release`
- bufferul DMA are constrangeri explicite, nu implicite
- `device_address` nu este egal conceptual cu `physical address`, chiar daca implementarea `x86_64` poate folosi identitatea fizica in prima faza

## Relatia DMA cu `NetworkBuffer`

`NetworkBuffer` ramane obiectul logic din `kernel-core` pentru datapath.

Maparea corecta este:

- `NetworkBuffer`
  - lifecycle logic in networking subsystem
- `DmaBuffer`
  - backing memory pentru descriptor/ring payload

Un `NetworkBuffer` poate referi un `DmaBufferId`, dar nu il inlocuieste.

Regula:

- lifecycle-ul `NetworkBuffer` si lifecycle-ul `DmaBuffer` trebuie sa se inchida impreuna la completion
- niciun driver nu are voie sa creeze cale alternativa care sare peste `NetworkBuffer`

## Ownership si lifecycle

### BAR

- owner initial: platform registry
- owner dupa claim: driver runtime
- owner dupa release: platform registry

Invariant:

- un BAR nu este simultan `Claimed` de doi consumatori

### Interrupt

- owner initial: platform interrupt allocator
- owner dupa registration: driver runtime
- acknowledge si disable sunt valide doar pentru owner-ul curent

Invariant:

- un `InterruptHandle` are un singur owner activ

### DMA buffer

- owner initial: DMA allocator
- owner operational: CPU sau device, niciodata ambele logic simultan
- release il intoarce allocatorului

Invariant:

- niciun `DmaBufferId` nu apare simultan in doua stari active incompatibile

## Invarianturi globale

Sistemul trebuie sa apere explicit:

- fiecare `DeviceLocator` este unic in snapshot-ul de platforma
- fiecare `BarId` este unic in cadrul device-ului
- fiecare mapping MMIO activ are region valid si interval neambiguu
- niciun mapping MMIO nu ramane activ dupa `unmap`
- fiecare `InterruptHandle` are route determinista si owner unic
- niciun interrupt nu poate fi ack-uit de alt owner
- memoria DMA nu este confundata cu heap-ul general
- niciun `DmaBuffer` nu este accesat fara mapping valid
- niciun `DmaBuffer` nu este simultan `OwnedByCpu` si `OwnedByDevice`
- toate operatiile hardware trec prin contractele HAL, nu prin bypass local

## Implementarea concreta `x86_64`

### PCI enumeration

Prima implementare concreta foloseste PCI ca bus real initial.

Model:

- scanare pe `segment 0`
- `bus 0..=255`
- `device 0..=31`
- `function 0..=7` doar cand header-ul indica multifunction
- pentru fiecare functie valida:
  - vendor/device/class/subclass/progif/revision
  - command/status
  - header type
  - BAR decode
  - interrupt pin/line

Rezultatul este `DeviceRecord { bus_kind: Pci, ... }`.

### BAR decode

Pentru fiecare BAR:

- se citeste valoarea
- se scrie `all ones`
- se citeste size mask
- se restaureaza valoarea initiala
- se determina:
  - tipul
  - baza
  - dimensiunea
  - flags

### MMIO mapping pe `x86_64`

MMIO mapping foloseste paging-ul existent si direct map-ul doar ca mecanism intern, nu ca API expus.

Regula:

- driverul primeste `MmioMappingHandle`
- nu primeste "foloseste direct map la adresa fizica"

### Interrupt routing pe `x86_64`

Prima implementare trebuie sa sustina:

- `LegacyLine` prin IOAPIC/LAPIC path cand este disponibil
- `Msi` acolo unde device-ul si platforma permit

Structura:

- interrupt registry in boot/platform
- vector allocator separat
- dispatcher generic care routeaza `vector -> InterruptHandle -> callback`

`PIC` ramane doar pentru bootstrap si fallback controlat al frontului timpuriu, nu modelul principal pentru device-uri PCI reale.

### DMA pe `x86_64`

Prima implementare trateaza `device_address` ca adresa fizica mapabila de device in absenta IOMMU.

Dar contractul HAL ramane compatibil cu IOMMU ulterior:

- `device_address` este tip separat
- maparea DMA este operatie explicita
- sync/ownership raman explicite

## Relatia cu driverele

Un driver real trebuie sa poata face doar urmatoarele:

1. cere lista de device-uri si filtreaza dupa identitate/capability
2. claim resources:
   - BAR
   - interrupt
   - DMA buffers
3. mapeaza BAR MMIO
4. programeaza hardware prin handle-ul MMIO
5. opereaza rings/queues folosind `DmaBuffer`
6. completeaza lifecycle-ul in `kernel-core`

Nu trebuie sa fie nevoie de:

- schimbare de semantica in HAL
- API nou pentru un alt tip de device
- acces direct la config-space sau paging intern

## Probe obligatorii pentru validare

Substratul este considerat validat doar cand exista probe reale pentru:

- enumerare completa PCI in QEMU
- listare de `DeviceRecord` multiple
- BAR decode corect si mapping MMIO valid
- registration/enable/ack de interrupt pe ruta reala
- alocare DMA si tranzitie de ownership CPU/device/CPU
- utilizare concurenta a:
  - mai multor device-uri
  - mai multor interrupts
  - mai multor DMA buffers

## Criteriul de stabilitate arhitecturala

Modelul este considerat suficient de inchis daca un driver:

- NIC
- storage controller
- alt device PCI MMIO

poate fi inceput fara schimbari structurale in:

- modelul de discovery
- modelul BAR
- modelul MMIO
- modelul interrupt
- modelul DMA

Schimbarile admise ulterior sunt:

- implementari noi
- optimizari
- suport pentru bus-uri noi

Nu sunt admise ca necesitate:

- redefinirea lifecycle-ului
- inlocuirea ownership model
- rescrierea contractelor de baza
