> [!NOTE]
> Any use of AI is extremely limited to completely nonexistent in this repository, outside from the following _potential_ parts:
> - Tests
> - CI/CD
> - Documentation
> 
> and of course... rubber ducking :)

# Piecewise
Game engine with board game virtualization in mind!

## Development

### Environment Set up

* Follow the instructions in the [Tauri docs](https://v2.tauri.app/start/prerequisites/#system-dependencies)
* Make sure your Rust toolset is up to date: `rustup update stable`
* `cargo install tauri-cli --version "^2.0.0" --locked`
* Download all npm dependencies: `npm install`
* Install the VSCode `Live Server` extension or equivalent (used to dev test `wisdom` results)

### Run app in development mode
`cd piecewise/src-tauri; cargo tauri dev`

### Compile `wisdom` YAML for dev testing (VSCode)
1. Create a game config in `wisdom/test.yaml`
1. Run `wisdom/src/main.rs`
1. Right click `wisdom/dist/index.html` and select `Open with Live Server`
