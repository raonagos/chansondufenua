# Contributing to Chanson du *fenua*

Thank you for considering contributing to **Chanson du fenua** ! We welcome contributions from everyone. Here are some guidelines to help you get started :

## Ways to Contribute

- **Bug Reports** : If you find a bug, please report it using the issue tracker. Provide as much detail as possible to help us reproduce and fix the issue.
- **Feature Requests** : Have an idea for a new feature? Submit a feature request and describe the problem you're trying to solve.
- **Code Contributions** : We welcome pull requests! Please follow the guidelines below to ensure a smooth process.
- **Documentation** : Improvements to documentation are always appreciated. If you find any gaps or unclear sections, feel free to submit updates.

## Getting Started

1. Clone this repository : `gh repo clone raonagos/chansondufenua`.
2. Navigate to the project directory: `cd chansondufenua`.
3. Install a chromium engine.
4. Install the dependencies: `cargo-leptos build`.

## Usage

1. Make sure you have [rust](https://www.rust-lang.org/learn/get-started), [cargo-leptos](https://github.com/leptos-rs/cargo-leptos?tab=readme-ov-file#getting-started), `wasm32-unknown-unknown` target and chromium engine installed on your machine.
2. Fill the [.env](env.example).
3. Run your favorite database provider (*[surrealdb](https://surrealdb.com/install) was implemented*).
4. Run the application with the following command: `cargo-leptos serve`.
5. Open your browser and go to the URL: `http://localhost:3000`.

## Contribute

1. **Create a new branch** :
```bash
git checkout -b feature/your-feature-name
```
2. **Make Your Changes** : Implement your feature or bug fix. Make sure to follow the coding standards and write tests if applicable.
3. **Commit your changes** :
```bash
git add .
git commit -m "Describe your changes"
```
4. **Push your fork** :
```bash
git push origin feature/your-feature-name
```
5. **Open a PR** : Go to the [upstream repository](https://github.com/raonagos/chansondufenua) and open a pull request with a clear title and description or via cli :
```bash
gh pr create
```

## Code Style and Standards

- Follow the existing code style and conventions.
- Write clear and concise commit messages.
- Include tests for new features and bug fixes.
- Ensure your code passes all existing tests.

## Reporting Issues

- Provide a clear and descriptive title.
- Describe the expected behavior and the actual behavior.
- Include steps to reproduce the issue.
- Mention your operating system and any other relevant details.

## Notes

The project aims to adopt a hexagonal architecture by separating interchangeable parts into distinct layers. Using Cargo's workspace feature, we define these layers within the workspace and strive to minimize interdependencies among them. The primary layers are :

- api : manages the API endpoints and communication
- core : provides essential utilities and services used across the application
- domain : contains the core business logic and rules
- database : handles database interactions and data persistence.

Additionally, there are sub-layers like `server`, `web`, and `app`, which are more tightly integrated with the `core` layer. These components work together to deliver the application's functionality, while the other layers can evolve more independently due to their reduced dependency on the core.