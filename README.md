# ci-utils

## Cargo features

Both are off by default: a project which only generates CI files, compiles a proto folder lying
next to it or concatenates css does not pay for an http client.

- **`download-resource-by-http`** - taking a resource from a url at all: `download_file`,
  `sync_and_build_proto_file` and `ProtoFileBuilder::new("http://...")`. Brings in FlUrl and the
  tokio runtime the synchronous `build.rs` blocks on;
- **`with-tls`** - the same over `https://`. Implies the feature above and adds the rustls stack on
  FlUrl's pure Rust provider (`with-rust-tls`), so a build script needs no C toolchain - at the
  cost of building on x86_64 and aarch64 only.

```toml
[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "<tag carrying the feature>", features = [
    "with-tls",
] }
```

A url used without the matching feature is not silently skipped - the build script panics naming
the feature to switch on, so the outcome is a red build instead of a proto file compiled from a
stale local copy.

## Use cases

### Generate Dockerfile + GitHub Actions in `build.rs`

> **`CiGenerator` is for single-repo services only** (one service = one GitHub repo). It generates
> `Dockerfile` and `.github/workflows/release.yaml` in the repo root, so it does not apply in a
> monorepo holding many services: there both workflow files are written by hand -
> `release-{service-name}.yaml` (triggered by the `{service-name}-*` tag) and
> `build-{service-name}-docker.yaml` (`workflow_dispatch`, bakes the builder image). Templates and
> the load-bearing rules are in the app-bootstrap guide, section CI / GitHub Actions. In a monorepo
> `build.rs` is still needed for `ProtoFileBuilder` and `CssCompiler` - just without `CiGenerator`.

```rust
use ci_utils::ci_generator::{CiGenerator, DockerFileType};

fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()          // Dockerfile + release.yml
        .with_ff_mpeg()              // optional ffmpeg layer + workflow step
        .add_docker_copy_file("./Rocket.toml", "./Rocket.toml")
        .generate_github_ci_file()
        .with_ci_test()              // optional: only if project has unit tests
        .build();
}
```
Always pass `env!("CARGO_PKG_NAME")` to `CiGenerator::new` in `build.rs` so generated names match the crate.

**Note:** Only add `.with_ci_test()` if the project has at least one unit test (`#[test]`). Skip it for projects without tests.

For Dioxus web builds (release-dioxus.yaml + Dioxus Dockerfile):
```rust
CiGenerator::new(env!("CARGO_PKG_NAME"))
    .as_dioxus_fullstack_service()              // Dockerfile + release.yaml
    .set_docker_container_name("myjettools/dioxus-docker:0.x.y") // optional override
    .generate_github_ci_file()
    // .with_ci_test()                          // optional: only if project has unit tests
    .build();
```
## Compile-time secrets

A secret which must be baked into the binary during `cargo build --release` on the GitHub runner
and must **not** exist at runtime.

One declaration does both things: teaches the workflow to export the secret and bakes the exported
value into the binary.

```rust
// build.rs
fn main() {
    CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .add_compile_time_secret("ENCRYPTION_KEY")   // GitHub secret name == env var name
        // .add_compile_time_secret_as("ENCRYPTION_KEY", "MESH_KEY") // secrets.ENCRYPTION_KEY -> MESH_KEY
        .generate_github_ci_file()
        .build();
}
```

Can be called several times - one call per secret.

### Reading the injected value

`option_env!` is the only thing needed on the reading side: it is expanded into `Some("...")` when
the value was injected into the build and into `None` when it was not.

```rust
// src/compile_time_secrets.rs
const ENCRYPTION_KEY: Option<&'static str> = option_env!("ENCRYPTION_KEY");

pub fn get_encryption_key() -> &'static str {
    match ENCRYPTION_KEY {
        Some(value) => value,
        // a local debug build never has the production secret - let the developer run the app
        #[cfg(debug_assertions)]
        None => "dev-only-key-never-used-in-release",
        // a release binary without the key is broken - say it loudly and immediately
        #[cfg(not(debug_assertions))]
        None => panic!(
            "The binary is compiled without the ENCRYPTION_KEY compile time secret. Check that the secret exists in the Github repository settings and rebuild the tag"
        ),
    }
}
```

```rust
// main.rs - touch every compile time secret while starting up, so a broken binary dies
// at the start and not in the middle of the first request which needs the key
fn main() {
    let _ = crate::compile_time_secrets::get_encryption_key();
    ...
}
```

Rules for the reading side:

- never log the value and never put it into a settings/health/debug endpoint - it is exactly as
  secret as it was in the GitHub settings;
- keep it behind a function like above instead of spreading `option_env!` over the code: the
  `None` case then has one single behaviour;
- if the secret is a hex/base64 payload rather than a string, decode it once into a
  `std::sync::LazyLock<Vec<u8>>` built on top of the same function.

Checklist for the whole thing to work:

1. `build.rs` belongs to **the same crate** which reads the value: `option_env!` is expanded while
   that crate is being compiled, and `cargo:rustc-env` only applies to the crate owning the
   `build.rs`. Putting `option_env!` into a dependency crate silently yields `None`.
2. The regenerated `.github/workflows/release.yaml` has to be committed.
3. **A human has to create the secret** in the GitHub repo: `Settings -> Secrets and variables ->
   Actions -> New repository secret`, named exactly as in `add_compile_time_secret`. There is no
   way to do it from the code.
4. The code must handle `None` - that is the normal state of every local developer build.

### Declared but not injected

A secret which is declared in `build.rs` but has no value in the environment of the build is
reported, and the severity depends on where the build happens:

- **locally** - a `cargo:warning` in the console, the build goes on and `option_env!` returns
  `None`. A developer has to be able to check that the project compiles at all without having the
  production secrets;
- **during the release build on the GitHub runner** (`GITHUB_ACTIONS=true` and the build is
  triggered by a tag) - `build.rs` fails, so the `Build` step of the workflow goes red. A released
  binary with a missing key is broken anyway and it is much cheaper to see it in the workflow than
  at runtime. This is exactly what happens when the secret is not created in the repo settings:
  GitHub expands an unknown `${{ secrets.X }}` into an empty string instead of failing.

The test workflow generated by `.with_ci_test()` also runs on the runner but is triggered by a
branch push, not by a tag, so it does not need the secrets and is not affected.

The only thing it changes in the generated `.github/workflows/release.yaml` is the `Build` step:

```yaml
      - name: Build
        run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          export ENCRYPTION_KEY="${{ secrets.ENCRYPTION_KEY }}"
          cargo build --release
```

**At runtime the env variable does not exist - by design.** The `export` is done inside the `run:`
block of the build step, so the value lives only in the shell of that step. It is deliberately
**not**:

- an `ARG`/`ENV` in the generated `Dockerfile`;
- a `--build-arg` of `docker build`;
- a job/workflow level `env:` entry.

The binary is compiled on the runner and only copied into the image, so neither the variable nor a
layer holding it ends up in the container - `docker exec`, `env` and `docker inspect` show nothing.

**Do not "fix" a compile time secret by adding it to the runtime environment.** If the value is
missing in the container it means the GitHub secret is missing or the tag was built before the
secret was added - rebuild, do not add the variable to `docker-compose.yaml`, to the service
settings or to the `Dockerfile`. Doing that puts the value back into `docker inspect` and defeats
the whole point of the feature.

Note on the threat model: the value is invisible to anything looking at the container environment,
but it is still a string inside the executable, so `strings` over the binary pulled out of the
image shows it. Compile time secrets protect against a leak through the environment, not against
someone who already has the binary.

Under the hood `build()` prints `cargo:rustc-env=<NAME>=<value>` and
`cargo:rerun-if-env-changed=<NAME>` for every declared secret. The value itself is never logged.

To bake a value without declaring anything in the workflow - it is baked when the variable is set
and quietly skipped when it is not - call `ci_utils::bake_compile_time_secret("NAME")` or
`ci_utils::bake_compile_time_secrets(&["A", "B"])` directly.

> **Inside a monorepo builder container the guarantee does not hold.** A monorepo release builds the
> service with a `docker run ... cargo build --release` step, and the container does not inherit the
> environment of the runner: neither the secret itself nor `GITHUB_ACTIONS=true` gets in. The check
> which fails the release build is keyed exactly on `GITHUB_ACTIONS`, so it does not fire -
> `build.rs` prints a `cargo:warning`, `option_env!` returns `None`, the workflow stays green and a
> binary without the key goes to production. If a monorepo service uses `bake_compile_time_secret` /
> `bake_compile_time_secrets`, every secret has to be passed into `docker run` explicitly via
> `-e NAME="${{ secrets.NAME }}"`, plus `-e GITHUB_ACTIONS=true` so that a missing secret paints the
> step red again. `-e` sets the environment of a single container run and never ends up in the image
> layers - this is not the same mistake as `--build-arg`.

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
- A url source needs the `download-resource-by-http` feature, and `with-tls` on top of it for
  `https://`. A local folder and a relative `../proto-files` path need neither - they are read
  straight from the disk.

## File helpers

### Download any text file
```rust
ci_utils::download_file("https://example.com/file.txt", "local.txt");
```
Needs `download-resource-by-http`, plus `with-tls` for an `https://` url.

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
