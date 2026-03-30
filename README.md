# Next Gen OS

`Next Gen OS` (`ngos`) este un sistem de operare original, cu kernel propriu, arhitectura proprie si identitate proprie.

Nu este un derivat conceptual din Linux, Windows, Android sau alt sistem existent.

Starea corecta actuala este:

- arhitectural, `ngos` este original
- ca origine de implementare, workspace-ul nu este inca complet proprietar
- directia proiectului este tranzitia spre o baza complet proprietara

## Identitate

- kernel propriu
- ABI nativ propriu
- model intern propriu pentru procese, memorie, I/O, securitate, drivere si observabilitate
- compatibilitate externa doar ca strat separat, niciodata ca fundatie interna

## Ce nu este

- nu este un Linux nou
- nu este un Windows-like kernel
- nu este un macOS-like sistem
- nu este un toy OS
- nu este un demo kernel
- nu este un MVP minimalist gandit sa fie aruncat si rescris mai tarziu

## Principii

- `64-bit only`, fara compromisuri pentru `32-bit`
- subsisteme reale, nu mock-uri sau versiuni simbolice
- complexitatea necesara produsului final trebuie urmarita din prima
- compatibilitatea externa nu are voie sa dicteze arhitectura interna
- orice idee preluata din alte sisteme trebuie absorbita si redesenata in termenii `ngos`
- portarea nu este scop in sine; valoarea vine din integrare, modernizare si design mai bun
- nu exista obiectiv de suport pentru `macOS` sau tehnologii Apple care introduc risc juridic sau dependenta de ecosisteme inchise

## Directia de Dezvoltare Activa

Dezvoltarea activa a `ngos` se face in directia `nano-kernel`.

Asta inseamna:

- subsistemele noi si extinderile majore se introduc ca unitati semantice mici
- fiecare unitate noua trebuie sa aiba trigger explicit, autoritate ingusta si observabilitate clara
- dezvoltarea trebuie sa reduca responsabilitatile concentrate, nu sa le extinda
- orice front nou trebuie sa fie impins prin slice-uri verticale reale, nu prin manageri tot mai mari
- regula nano-semantica se aplica nu doar kernelului, ci si userland-ului, shell-ului, tooling-ului si oricarei suprafete noi de control sau dezvoltare
- un shell mare este acceptat numai daca ramane compus din agenti/moduluri semantice mici, nu daca revine la manageri interni opaci
- userland-ul `ngos` trebuie sa creasca ca ecosistem semantic coerent, nu ca acumulare de logica compactata intr-un singur bloc

Nu este acceptata dezvoltarea inapoi spre model monolitic intern, chiar daca suprafata externa ramane unitara.

## Contract de Executie

Executia in `ngos` se face pe fronturi complete, nu pe progres fragmentat.

Regula activa este:

- nu sunt valide livrarile de tip "am pregatit baza", "am pus hook-uri", "am adaugat structuri", "urmeaza sa implementez" sau "pot continua cu"
- orice front inceput trebuie impins pana la comportament real, integrare reala, observabilitate reala si demonstratie sau verificare reala
- o implementare nu este `done` daca nu produce efect vizibil in runtime, nu este integrata in fluxurile existente, nu poate fi observata si nu poate fi explicata cauzal
- gruparea muncii se face pe fluxuri complete si capabilitati complete, nu pe micro-pasi raportabili
- nu se accepta fragmentare artificiala doar pentru a raporta progres
- unde exista o simplificare rezonabila, ea trebuie sa fie functionala si reala, nu placeholder
- raportarea trebuie sa descrie doar frontul inchis, comportamentul nou, verificarea reala si gap-urile reale
- un front nu este `done` daca este demonstrat doar pe path-ul pozitiv
- pentru orice front declarat inchis, validarea trebuie sa includa:
  - path de succes
  - path de blocare, refuz sau eroare, daca subsistemul poate refuza
  - recovery sau reversibilitate, daca subsistemul permite revenirea
  - expunerea observabila a starii finale

### Clauza de Scope

Termenii de subsistem folositi in cerinta sunt autoritari si nu pot fi ingustati unilateral.

Exemple:

- `inchide VM` inseamna inchide subsistemul VM ca ansamblu
- `inchide VFS` inseamna inchide VFS cap-coada
- `inchide networking` inseamna inchide networking ca subsistem

Nu este permisa reformularea lor ca:

- frontul lucrat in acest ciclu
- sub-frontul curent
- calea urmarita aici

Un subsistem este considerat inchis numai daca toate familiile lui relevante au fost:

1. implementate si validate cap-coada, sau
2. declarate explicit `out-of-scope` de utilizator inainte de executie

Daca subsistemul mare nu este inca inchis, formularea corecta este doar:

- `Subsistemul <nume> NU este inca inchis.`

Nu este permisă prezentarea unui `done` local ca si cum ar fi `done` global.

### Clauza Anti-Oprire Prematura

Când cerința este de forma `nu te opri pana nu inchizi X`, atunci:

- fiecare raspuns intermediar este doar progres partial
- niciun raspuns intermediar nu are voie sa contina concluzii precum:
  - `front inchis`
  - `nu mai exista gap`
  - `este complet`
  - sau echivalente

decât dacă `X` este într-adevăr închis complet.

Pentru o cerinta precum `inchide VM`, sunt interzise formulări ca:

- `frontul VM lucrat aici este inchis`
- `nu mai exista gap-ul urmarit`
- `ce a ramas sunt alte fronturi VM`

cât timp subsistemul VM nu este închis cap-coadă.

### Hard Scope Law

Când cerința este `inchide X`, `X` înseamnă subsistemul complet, nu sub-frontul ales local.

Este interzis:

- să reduci scope-ul la `frontul lucrat acum`
- să declari `done` local ca substitut pentru `done` global
- să te oprești cât timp mai există familii relevante din același subsistem
- să reclasifici restul ca `alte fronturi` doar ca să justifici oprirea

Regula absolută este:

Dacă mai există familii relevante neînchise din `X`, atunci singura formulare corectă este:

- `Subsistemul X NU este încă închis.`

Apoi trebuie:

- să enumeri ce este închis
- să enumeri ce este încă deschis
- să continui execuția pe ce a rămas

Până la închiderea completă a lui `X`, sunt interzise expresii precum:

- `front închis`
- `nu mai există gap`
- `ce a rămas sunt alte fronturi`
- `am închis ce am urmărit aici`

### Completitudinea Prematura Inseamna Executie Esuata

Orice raspuns care declara completitudine inainte de inchiderea intregului subsistem cerut este invalid si trebuie tratat ca executie esuata.

Regula ramane valabila chiar daca modificarile tehnice facute pana in acel punct sunt bune in sine.

Progresul tehnic poate fi real, dar executia este esuata daca este prezentat ca inchidere de subsistem inainte de inchiderea completa a subsistemului.

## Regula Absoluta

Regula urmatoare este litera de lege si trebuie respectata indiferent de situatie:

- toate regulile din acest document sunt obligatorii pentru orice agent, LLM, contributor, script sau automatizare care opereaza in repo
- aceste reguli au caracter normativ, nu orientativ
- nu se accepta derogari implicite, reinterpretari convenabile sau exceptii motivate de viteza, comoditate, testare, bootstrap, demo sau preferinta locala de implementare
- daca exista conflict intre o decizie locala si regulile proiectului, regulile proiectului prevaleaza
- orice agent care actioneaza in repo trebuie sa trateze aceste reguli ca litera de lege

- nu se folosesc `mock`, `demo`, `minimal`, `toy`, `showcase`, `MVP`, `sample`, `example-only` sau echivalente mascate ca directie de proiect
- nu se introduc subsisteme, binare, crate-uri, backend-uri, API-uri, fluxuri sau path-uri a caror identitate sau justificare este una provizorie, demonstrativa sau simbolica
- nu se accepta exceptii motivate prin testare, prezentare, viteza de iteratie, bootstrap sau validare locala daca rezultatul introduce o componenta care contrazice regula de mai sus
- daca este nevoie de verificare, aceasta trebuie sa serveasca implementarea reala si sa nu redefineasca produsul sub forme reduse sau temporare
- orice componenta existenta care contrazice aceasta regula trebuie tratata ca datorie tehnica de corectat, redenumit, absorbit sau eliminat

## Compatibilitate

Compatibilitatea este un obiectiv pragmatic, nu identitatea sistemului.

Directia compatibilitatii externe este:

- aplicatii native `ngos`
- compatibilitate selectiva cu userland Linux
- compatibilitate selectiva cu aplicatii si modele de executie Windows
- suport Android doar in masura in care poate fi obtinut prin straturi curate si sustenabile juridic

Regula centrala:

- daca o suprafata de compatibilitate degradeaza coerenta, siguranta sau directia arhitecturala a nucleului, acea suprafata trebuie limitata, izolata sau eliminata

## Fara Demo Surface

Proiectul nu urmareste:

- binare, rapoarte sau path-uri adaugate doar pentru prezentare
- API-uri temporare create doar ca sa "arate progres"
- subsisteme reduse intentionat doar pentru demo
- lucru orientat spre showcase in loc de integrare reala
- `mock`, `minimal`, `toy`, `sample` sau alte forme mascate ale aceluiasi compromis

Fiecare pas trebuie sa lase in urma un subsistem mai puternic, mai testabil si mai apropiat de forma finala, nu doar un artefact usor de demonstrat.

## Tinte Tehnice

- arhitecturi suportate: `x86_64`, `aarch64`
- moduri de rulare:
  - `host-runtime`
  - `kernel`
- toolchain: Rust + LLVM
- kernel core fara dependenta directa de OS-ul gazda
- backend-uri de platforma separate de logica nucleului

## Structura Logica

- `kernel-core`
  - obiecte kernel, procese, scheduler, VM, handles, sync, eventing, syscall surface
- `platform-hal`
  - contracte de platforma: CPU, memorie, mapari, bootstrap, traps
- `platform-host-runtime`
  - backend de dezvoltare si validare pe host
- `platform-x86_64`
  - fundatie pentru target kernel real `x86_64`
- `user-abi`
- `user-runtime`
- `tooling`

## Reguli de Constructie

- designul intern se defineste in termenii `ngos`, nu in termenii altor sisteme
- orice strat de compatibilitate trebuie sa mapeze spre modelul intern, nu sa il inlocuiasca
- nu se accepta compromisuri arhitecturale doar pentru paritate superficiala cu alte OS-uri
- nu se accepta portari line-by-line daca ele conserva bagaj istoric fara valoare
- nu se accepta lucru orientat spre minimalism strategic sau "merge acum, refacem mai tarziu"
- interdictia asupra `mock` / `demo` / `minimal` / `toy` / `showcase` are prioritate peste orice argument tactic sau contextual

## Fundatia Actuala

Repo-ul actual defineste:

- nucleul in dezvoltare (`kernel-core`)
- contractele de platforma (`platform-hal`)
- backend-ul de gazda (`platform-host-runtime`)
- fundatia de boot si platforma pentru `x86_64`
- straturi de compatibilitate care trebuie sa ramana separate de identitatea interna a sistemului

Aceasta fundatie nu defineste limita proiectului. Ea este doar baza pentru un sistem de operare complet, profund si original.

## Tranzitie Proprietara

Obiectivul este un `ngos` complet proprietar in implementare, nu doar in directie arhitecturala.

Reguli:

- nu se mai introduc portari noi din surse de OS straine
- noile implementari trebuie scrise direct in termenii `ngos`
- compatibilitatea externa ramane permisa doar ca adaptor, nu ca sursa de implementare importata

Documentul de lucru pentru aceasta tranzitie este:

- `docs/proprietary-transition.md`

## Pornire Rapida

```powershell
cargo run -p ngos-host-runtime
```
