# Next Gen OS

`Next Gen OS` (`ngos`) este un proiect original de sistem de operare, cu kernel propriu, ABI propriu si arhitectura interna proprie.

Proiectul nu urmareste sa fie un derivat conceptual din Linux, Windows, Android sau alt sistem existent. Compatibilitatea externa este tratata ca strat separat, nu ca fundatie interna.

## Directie

- kernel propriu si model intern propriu pentru procese, memorie, I/O, securitate si observabilitate
- ABI nativ propriu
- arhitectura orientata spre subsisteme semantice mici si fronturi verticale reale
- tranzitie activa spre o baza de implementare complet proprietara

## Principii

- `64-bit only`
- subsisteme reale, nu mock-uri sau suprafete simbolice
- compatibilitatea externa nu dicteaza arhitectura interna
- designul intern este definit in termenii `ngos`
- noile implementari trebuie scrise direct pentru `ngos`, nu portate mecanic din alte sisteme

## Workspace

Workspace-ul actual include fundatia principala a proiectului:

- `kernel-core`
- `platform-hal`
- `platform-host-runtime`
- `platform-x86_64`
- `user-abi`
- `user-runtime`
- `userland-native`

## Rulare

Pentru a porni runtime-ul curent:

```bash
cargo run -p ngos-host-runtime
```

Pentru build complet:

```bash
cargo build --workspace
```

Pentru testare:

```bash
cargo test --workspace
```

## Licenta si Contributii

Acest repository este public pentru vizibilitate, evaluare si referinta. Termenii de utilizare sunt definiti in [LICENSE](LICENSE), iar termenii pentru contributii sunt definiti in [CONTRIBUTING.md](CONTRIBUTING.md).

## Status

Din punct de vedere arhitectural, `ngos` este original. Din punct de vedere al originii complete a implementarii, proiectul este in tranzitie spre o baza complet proprietara.
