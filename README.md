# Chanson du *fenua*

Chanson du *fenua* is an application that allows you to sing and play songs with
their musical chords: a collection of traditional and contemporary Polynesian
songs, each with its lyrics, its chords and a transposition control, served in
French, Tahitian and English.

Every page is rendered on the server and compiled together with the stylesheet,
the fonts, the logos and the database engine into a single executable. There is
no client-side runtime, no separate database server to install, and no JavaScript
needed to read a page.

## Build and run

```bash
cargo build
topcoat asset bundle
./target/debug/chansondufenua
```

The server answers on <http://localhost:3000>. [CONTRIBUTING.md](./CONTRIBUTING.md)
has the requirements, the environment variables, the database, the translations
and the test suite.

## Contributing

Contributions are welcome: follow [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

This project is licensed under the [GPL 3.0 License](./LICENSE).
