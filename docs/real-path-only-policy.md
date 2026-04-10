## Real Path Only Policy

Această regulă este explicită pentru execuția de acum înainte în `ngos`.

### Regula de bază

De acum înainte, implementarea strategică se face doar pe path-ul real al sistemului:

- `boot-x86_64`
- `platform-x86_64`
- `kernel-core`
- `user-runtime`
- `userland-native`
- `QEMU`

### Ce înseamnă concret

- Nu închidem subsisteme pe `host-runtime`.
- Nu tratăm path-urile sintetice sau model-only ca destinație de produs.
- Nu declarăm un subsistem închis dacă nu există comportament real, observabil și verificat pe path-ul real.
- `QEMU` este primul truth surface acceptat pentru closure.
- `hardware fizic` rămâne pasul ulterior, când intră în scope, dar nu schimbă regula că implementarea trebuie să fie deja pe path-ul real.

### Ce este permis

- Validarea auxiliară locală este permisă doar dacă accelerează implementarea reală.
- Orice helper host-side, synthetic sau tooling-side trebuie să rămână subordonat path-ului real.
- Dacă un comportament este testat mai întâi auxiliar, el trebuie împins imediat în:
  - `boot-x86_64`
  - `platform-x86_64`
  - `kernel-core`
  - `user-runtime`
  - `userland-native`
  - `QEMU`

### Ce este interzis

- Să folosim `host-runtime` ca punct final de closure.
- Să construim fronturi importante doar ca demo, simulare sau validare sintetică.
- Să raportăm completion globală pentru un subsistem care nu a trecut prin path-ul real.
- Să consumăm bugetul principal de dezvoltare pe path-uri auxiliare în locul path-ului real.

### Regula de execuție

Pentru subsistemele strategice, ordinea de acceptare este:

1. implementare pe path-ul real
2. integrare end-to-end
3. observabilitate reală
4. success path
5. refusal/error path
6. recovery/release path
7. dovadă pe `QEMU`

Fără această secvență, subsystemul nu este considerat închis.

### Formulare scurtă

De acum înainte:

- construim direct pentru path-ul real
- validăm pe `QEMU`
- nu închidem nimic strategic doar pe host sau pe simulări
