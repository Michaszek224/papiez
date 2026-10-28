# papiez

Wpisujesz `papiez`, a litery zlatują się na ekran i układają w Jana Pawła II z ASCII.
Każde uruchomienie wygląda inaczej. Dowolny klawisz — litery się rozlatują,
a terminal wraca do stanu sprzed uruchomienia.

```sh
cargo install --path .
papiez
```

## Jak to działa

- **Portret (`pope.rs`)** — ASCII-art generowany ze zdjęcia: kadr na twarz,
  wyostrzenie, wyrównanie histogramu i losowa rampa znaków (` .:-=+*#%@` itp.),
  pokolorowany kolorami zdjęcia.
- **Nalot (`morph.rs`)** — każde miejsce w portrecie dostaje literę ze słów
  „JAN PAWEŁ II”, „BARKA”, „2137”… lecącą po krzywej Béziera; tuż przed
  lądowaniem zamienia się w znak ASCII. Losowane przy każdym uruchomieniu:
  skąd litery przylatują (zza krawędzi, deszcz z góry, wybuch z miejsca kursora,
  materializacja w powietrzu, wir), kolejność zapełniania i krzywizna torów.
- **Rozlot** — nowy, losowy plan nalotu odtworzony od końca.
- **Ekran** — alternatywny bufor terminala, więc po wyjściu nic nie ginie.
  Rysowane są tylko zmienione komórki, w trybie synchronized output.

## Zdjęcie

`assets/jp2.jpg` — [Pope John Paul II smile.jpg](https://commons.wikimedia.org/wiki/File:Pope_John_Paul_II_smile.jpg),
Wikimedia Commons, licencja CC BY 3.0 PL.
