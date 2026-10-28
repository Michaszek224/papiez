# papiez

Wpisujesz `papiez`, a litery zlatują się na ekran i układają w Jana Pawła II z ASCII.
Każde uruchomienie wygląda inaczej. Dowolny klawisz — litery się rozlatują,
a terminal wraca do stanu sprzed uruchomienia.

```sh
cargo install --path .
papiez
```

## Jak to działa

- **Portret (`pope.rs`)** — ASCII-art na cały terminal. Z czterech zdjęć wybierane
  jest to, którego kadr z papieżem najlepiej pasuje do proporcji okna (szerokie →
  rozłożone ręce, kwadratowe → uśmiech, wąskie → cała postać); przy remisie losowo.
  Dalej: wyostrzenie, wyrównanie histogramu, wygaszenie tła wokół papieża i losowa
  rampa znaków (` .:-=+*#%@` itp.), pokolorowane kolorami zdjęcia.
- **Nalot (`morph.rs`)** — każde miejsce w portrecie dostaje literę ze słów
  „JAN PAWEŁ II”, „BARKA”, „2137”… lecącą po krzywej Béziera; tuż przed
  lądowaniem zamienia się w znak ASCII. Losowane przy każdym uruchomieniu:
  skąd litery przylatują (zza krawędzi, deszcz z góry, wybuch z miejsca kursora,
  materializacja w powietrzu, wir), kolejność zapełniania i krzywizna torów.
- **Rozlot** — nowy, losowy plan nalotu odtworzony od końca.
- **Ekran** — alternatywny bufor terminala, więc po wyjściu nic nie ginie.
  Rysowane są tylko zmienione komórki, w trybie synchronized output.

## Zdjęcia

Wszystkie z Wikimedia Commons:

| plik | źródło | licencja |
|---|---|---|
| `assets/jp2-wave.jpg` | [Portrait of the Pope John Paul II.JPG](https://commons.wikimedia.org/wiki/File:Portrait_of_the_Pope_John_Paul_II.JPG) | domena publiczna |
| `assets/jp2-smile.jpg` | [Pope John Paul II smile.jpg](https://commons.wikimedia.org/wiki/File:Pope_John_Paul_II_smile.jpg) | CC BY 3.0 PL |
| `assets/jp2-tall.jpg` | [Pope John Paul II 1984 portrait.jpg](https://commons.wikimedia.org/wiki/File:Pope_John_Paul_II_1984_portrait.jpg) | Attribution |
| `assets/jp2-full.jpg` | [Pope John Paul II (1987).jpg](https://commons.wikimedia.org/wiki/File:Pope_John_Paul_II_(1987).jpg) — Reagan White House Photographs | domena publiczna |
