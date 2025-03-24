# Chanson du *fenua*

Chanson du *fenua* is an application that allows you to sing and play songs with their musical chords. Immerse yourself in the musical richness of Polynesia with our collection of traditional and contemporary songs. Improve your musical skills while exploring the culture of the *fenua* through music.

## Features

- Access a vast collection of Polynesian songs with their musical chords.
- Learn to sing and play songs by following the musical chords.
- Explore different music genres, from traditional chants to popular songs.

## Contributing

Contributions to the Chanson du *fenua* application are welcome ! If you want to add new features, fix bugs, or improve the documentation, feel free to submit a pull request. Follow the [contribution guide](./CONTRIBUTING.md) to contribute and also the [notes](./CONTRIBUTING.md#notes).

## Notes

Chanson du *fenua* was born out of a simple yet relatable experience: the joy of singing together during gatherings. Whether it's a casual get-together or a festive celebration, singing is a universal language that brings people together. However, we've all been there—mid-song, and suddenly, the lyrics escape us (that was me xD). This project aims to solve that by providing a platform where you can always find the lyrics to your favorite songs.

But why stop at just lyrics ? Many of us are also eager to learn or improve our skills on instruments like the ukulele. So, we thought, why not combine the lyrics with musical notes ? This way, you can sing along while also learning to play the melody.

Chanson du *fenua* serves as your go-to resource for both singing and playing music. Whether you're a seasoned singer or just starting your musical journey, this site is here to help you enjoy the rich musical culture of Polynesia and beyond.

### Why Rust and wasm ?

The first two versions of this project were built using PHP. While they functioned well, there were noticeable inefficiencies in memory usage and data storage. Instead of scaling up the server's physical capacity, we decided to optimize resource usage by switching to Rust and WASM. This transition allowed us to maintain performance while being more resource-efficient.

- **Performance** : Let's face it, Rust is blazing fast. Everyone knows it, and we wanted a piece of that action on the server side.
- **Eco-Friendly** : Rust is greener than a salad on Earth Day (or thousand?). It's less hungry for memory and energy, making it the eco-warrior of programming languages as C/C++.
- **Type Safety** : Rust's type system is like a strict teacher who makes sure you write correct code. No more sloppy mistakes !
- **Compiler** : Cargo...cargo !
- **Multi Platform** : Thanks to WebAssembly (WASM), our app can run anywhere, and browsers are becoming its best friends.
- **Fun** : Rust is just plain fun to code in.

## License

This project is licensed under the [GPL 3.0 License](./LICENSE).
