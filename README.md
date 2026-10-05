# Chanson du *fenua*

Chanson du *fenua* is an application that allows you to sing and play songs with their musical chords. Immerse yourself in the musical richness of Polynesia with our collection of traditional and contemporary songs. Improve your musical skills while exploring the culture of the *fenua* through music.

## Features

- Access a vast collection of Polynesian songs with their musical chords.
- Learn to sing and play songs by following the musical chords.
- Explore different music genres, from traditional chants to popular songs.

## Getting started

```bash
cargo build && topcoat asset bundle && ./target/debug/chansondufenua
```

The site is one server-rendered binary with an embedded SQLite database; the [contribution guide](./CONTRIBUTING.md) covers the toolchain, the three variables, and why that middle command is not optional.

## Contributing

Contributions to the Chanson du *fenua* application are welcome ! If you want to add new features, fix bugs, or improve the documentation, feel free to submit a pull request. Follow the [contribution guide](./CONTRIBUTING.md) to contribute and also the [notes](./CONTRIBUTING.md#notes).

## Notes

Chanson du *fenua* was born out of a simple yet relatable experience: the joy of singing together during gatherings. Whether it's a casual get-together or a festive celebration, singing is a universal language that brings people together. However, we've all been there—mid-song, and suddenly, the lyrics escape us (that was me xD). This project aims to solve that by providing a platform where you can always find the lyrics to your favorite songs.

But why stop at just lyrics ? Many of us are also eager to learn or improve our skills on instruments like the ukulele. So, we thought, why not combine the lyrics with musical notes ? This way, you can sing along while also learning to play the melody.

Chanson du *fenua* serves as your go-to resource for both singing and playing music. Whether you're a seasoned singer or just starting your musical journey, this site is here to help you enjoy the rich musical culture of Polynesia and beyond.

### Why Rust ?

The first two versions of this project were built using PHP. While they functioned well, there were noticeable inefficiencies in memory usage and data storage. Instead of scaling up the server's physical capacity, we decided to optimize resource usage by switching to Rust. This transition allowed us to maintain performance while being more resource-efficient.

- **Performance** : Let's face it, Rust is blazing fast. Everyone knows it, and we wanted a piece of that action on the server side.
- **Eco-Friendly** : Rust is greener than a salad on Earth Day (or thousand?). It's less hungry for memory and energy, making it the eco-warrior of programming languages as C/C++.
- **Type Safety** : Rust's type system is like a strict teacher who makes sure you write correct code. No more sloppy mistakes !
- **Compiler** : Cargo...cargo !
- **One binary** : Pages, stylesheet, fonts, logos and the database engine are compiled into a single executable. There is no client-side runtime and no separate database server to install — see the [contribution guide](./CONTRIBUTING.md) for the three commands that build and run it.
- **Fun** : Rust is just plain fun to code in.

### Agent readiness

The web is read by more than browsers now, and the site is built to be legible to the ones that do not run JavaScript. The clearest example is the song list. Version 3 rendered each row as a click handler — the link had no destination:

```html
<tr class="tab-row" role="button" tabindex="0" onclick="window.location='/himene/7114wvk91gffr2bj6wza'">
  <td class="tab-cell"><a href class="tab-link">Āhani e</a></td>
```

The 3.x table was also filled in by the browser, so a client without a JavaScript engine read *"Chargement…"* and no songs at all. Now the same page arrives complete, with real destinations:

```html
<tr>
  <td><a href="/himene/7114wvk91gffr2bj6wza">Āhani e</a></td>
```

Every song is followable, keyboard-reachable, middle-clickable and copyable. Alongside that, the site serves `robots.txt`, two XML sitemaps, an `llms.txt`, RFC 8288 `Link` headers, a Markdown version of any song when asked (`Accept: text/markdown`), and a small read-only JSON API — so a song can be read as a page, as Markdown, or as JSON, and every form is discoverable from the others.

## License

This project is licensed under the [GPL 3.0 License](./LICENSE).
