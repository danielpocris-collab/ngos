# NGOS Subsystem Maturity Matrix

## Authority

Acest document fixează nivelul de maturitate pentru subsistemele strategice din `ngos`.

Nu este document de marketing.
Este document de execuție și evaluare.

## Maturity Levels

### M0: Declared

- subsistemul există numai ca nume, intenție sau documentare parțială
- nu există closure reală

### M1: Local

- există logică și teste locale
- nu există integrare suficientă pe truth path

### M2: Integrated

- există integrare reală în fluxurile sistemului
- există observabilitate și suprafețe consumabile
- încă nu există dovadă completă pe `QEMU`

### M3: QEMU-Closed

- există:
  - logică reală
  - integrare reală
  - observabilitate
  - success path
  - refusal/error path
  - recovery/release path unde e relevant
  - dovadă pe `QEMU`

### M4: Hardware-Closed

- tot ce există în `M3`
- plus dovadă pe hardware fizic unde acel subsistem cere validare hardware reală

## Matrix

| Subsystem | Current Maturity | Reason |
| --- | --- | --- |
| Boot and diagnostics | M3 | path real și dovezi `QEMU` există |
| CPU/runtime bring-up | M2 | policy și lifecycle există, dar closure globală hardware/families nu e completă |
| Process model | M2 | model puternic și integrat, dar nu este documentat ca închis global pe `QEMU` pentru întreaga familie |
| Scheduler | M2 | policy, fairness și proof există, dar `per-CPU/SMP/balancing` rămân deschise |
| Capability model | M2 | verified-core și inspectability există, dar nu este declarat închis global |
| Domain/resource/contract model | M2 | obiectele și inspectability există, dar closure globală nu este declarată |
| VFS | M3 | documentat închis pe path-ul real `QEMU` |
| VM | M4 | documentat închis pe `QEMU` și hardware fizic |
| Eventing and waits | M2 | închis pe minimum truth path `kernel-core -> syscall surface -> user-runtime`; nu cere `QEMU`, deci nu se promovează la `M3` |
| Signal runtime | M2 | închis pe minimum truth path `kernel-core -> syscall surface -> observabilitate`; nu cere `QEMU`, deci nu se promovează la `M3` |
| Device runtime | M2 | logică și dovezi locale bune, dar closure globală real-path nu este încă declarată |
| Networking | M2 | integrat și observabil, dar closure globală real-path nu este încă declarată |
| Syscall surface | M2 | închis pe minimum truth path `kernel-core -> user-abi -> user-runtime`; nu cere `QEMU`, deci nu se promovează la `M3` |
| Observability / procfs | M2 | foarte puternică, dar neînchisă global ca subsistem separat |
| User ABI | M2 | consistent și testat, dar ca subsistem nu este declarat închis global |
| User runtime | M2 | integrat și real, dar nu este declarat închis global |
| Native control/userland | M2 | foarte puternic, dar încă prea concentrat și neînchis global |
| Host runtime / synthetic validation | Auxiliary | nu este subsistem de produs închis, ci instrument |

## Upgrade Rule

Un subsistem poate fi mutat la nivel superior numai dacă există dovadă reală corespunzătoare nivelului respectiv.

Nu este permisă promovarea pe baza:

- optimismului
- volumului de cod
- unei singure demonstrații fericite

## Downgrade Rule

Dacă un subsistem pretins închis:

- nu mai are dovadă reală
- sau pierde familii obligatorii
- sau s-a bazat numai pe host/synthetic path

atunci maturitatea lui trebuie coborâtă explicit.

## Use Rule

Acest document trebuie folosit pentru:

- prioritizare
- roadmap real
- evaluare sinceră a repo-ului
- alegerea următorului subsistem strategic
