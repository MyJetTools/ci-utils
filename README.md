# ci-utils

Utility crate for build-time helpers: generate CI/Docker assets, sync/compile protos, and merge CSS/JS.

## Install

Add as a build-time helper (recommended):

```toml
[build-dependencies]
ci-utils = { version = "0.1.3", git = "https://github.com/myjettools/ci-utils.git" }
```

Then call it from `build.rs` so assets are generated during `cargo build`/`cargo test`.

```rust
// build.rs
use ci_utils::ci_generator::{CiGenerator, DockerFileType};

fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        .with_ci_test()
        .build();
}
```

## Use cases

### Generate Dockerfile + GitHub Actions in `build.rs`
```rust
use ci_utils::ci_generator::{CiGenerator, DockerFileType};

fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()          // Dockerfile + release.yml + test.yml
        .with_ff_mpeg()              // optional ffmpeg layer + workflow step
        .add_docker_copy_file("./Rocket.toml", "./Rocket.toml")
        .generate_github_ci_file()
        .with_ci_test()
        .build();
}
```
Always pass `env!("CARGO_PKG_NAME")` to `CiGenerator::new` in `build.rs` so generated names match the crate.

For Dioxus web builds (release-dioxus.yaml + Dioxus Dockerfile):
```rust
CiGenerator::new(env!("CARGO_PKG_NAME"))
    .set_docker_container_name("myjettools/dioxus-docker:0.7.0")
    .generate_github_ci_file()
    .build();
```

## Proto utilities

```rust
use ci_utils::ProtoFileBuilder;

fn main() {
    ProtoFileBuilder::new("https://example.com/protos")
        // optional: .skip_syncing() to reuse existing proto files in ./proto
        .sync_and_build("my.api.proto");
}
```

- Downloads `my.api.proto` into `./proto` (unless `skip_syncing`), then compiles it via `tonic_prost_build` with `--experimental_allow_proto3_optional`.
- You can also call `ci_utils::sync_and_build_proto_file(url, name)` or `ci_utils::compile_protos(path)` directly.

## File helpers

### Download any text file
```rust
ci_utils::download_file("https://example.com/file.txt", "local.txt");
```

### CSS concatenation
```rust
use ci_utils::css::CssCompiler;

CssCompiler::new("static/css")
    .add_file("reset.css")
    .add_file("app.css")
    .compile("public/app.css");
```
Reads each file in order and rewrites the output only when content changes.

### JS merge (strip leading `//` comments)
```rust
use ci_utils::js::merge_js_files;

merge_js_files(&["vendor.js", "app.js"], "public/app.js");
```
Reads from `JavaScript/<file>` and prefixes each chunk with the file name.

## Embedded templates

- `release.yml` and `test.yml` GitHub Actions workflows (versions injected via `CHECKOUT_VERSION` and `RUST_TOOLCHAIN_VERSION` constants).
- `release-dioxus.yaml` for Dioxus web bundles.
- `ffmpeg.yaml` snippet that is inserted into the release workflow when `with_ff_mpeg()` is enabled.

## Tips

- Generated workflow files are overwritten when `build()` runs; commit them if you want them versioned.
- Use it from `build.rs` so CI configs/Dockerfile/proto outputs stay in sync during regular `cargo build` or `cargo test` runs.
