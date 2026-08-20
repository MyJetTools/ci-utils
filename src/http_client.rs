/// Downloads `url` and returns the body as bytes.
///
/// Downloading is opt-in twice over, because each step costs dependencies a build script does
/// not need by default: `download-resource-by-http` brings in FlUrl plus a tokio runtime, and
/// `with-tls` adds the rustls stack on top of it. Without them a project still compiles proto
/// files - just from a folder instead of from a url.
///
/// Both missing-feature cases panic instead of being hidden behind `#[cfg]` on the function
/// itself: an unresolved-name error would point at the call site rather than at the feature
/// that has to be switched on, and the url is only known at build-script runtime anyway.
#[cfg(feature = "download-resource-by-http")]
pub(crate) fn download(url: &str) -> Vec<u8> {
    panic_if_https_without_tls(url);

    // build.rs is synchronous and FlUrl is async, so the runtime is created right here. A
    // build script downloads a handful of files, which is cheaper than the global state
    // sharing a single runtime between the calls would need.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build();

    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(err) => panic!(
            "Can not start a tokio runtime to download '{}'. Err: {:?}",
            url, err
        ),
    };

    runtime.block_on(download_async(url))
}

#[cfg(feature = "download-resource-by-http")]
async fn download_async(url: &str) -> Vec<u8> {
    // A build script makes one request and exits, so the connection pool would only keep a
    // background task and a socket alive after the file has already been written.
    let response = flurl::FlUrl::new(url)
        .do_not_reuse_connection()
        .get()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(err) => panic!("Failed to download '{}'. Err: {:?}", url, err),
    };

    let status_code = response.get_status_code();

    if !(200..300).contains(&status_code) {
        panic!(
            "Failed to download '{}'. Http Status is: {}",
            url, status_code
        );
    }

    match response.receive_body().await {
        Ok(body) => body,
        Err(err) => panic!("Failed to read the body of '{}'. Err: {:?}", url, err),
    }
}

/// FlUrl is compiled without a TLS provider here, so it would fail on an `https://` url at
/// connect time with an error about the provider. Saying which ci-utils feature is missing is
/// more useful than relaying that.
#[cfg(all(
    feature = "download-resource-by-http",
    not(feature = "with-tls")
))]
fn panic_if_https_without_tls(url: &str) {
    if is_https(url) {
        panic!(
            "Can not download '{}': ci-utils is compiled without the 'with-tls' feature, so https is not supported. Add features = [\"with-tls\"] to the ci-utils build dependency, or use an http:// url",
            url
        );
    }
}

#[cfg(all(feature = "download-resource-by-http", feature = "with-tls"))]
fn panic_if_https_without_tls(_url: &str) {}

#[cfg(not(feature = "download-resource-by-http"))]
pub(crate) fn download(url: &str) -> Vec<u8> {
    panic!(
        "Can not download '{}': ci-utils is compiled without the 'download-resource-by-http' feature, so resources can only be taken from a local folder. Add features = [\"download-resource-by-http\"] (plus \"with-tls\" for an https url) to the ci-utils build dependency, or point the builder at a folder",
        url
    );
}

/// The scheme is case insensitive, and `get` rather than slicing keeps a short or non-ascii
/// url from panicking on a byte boundary here instead of at the download.
#[cfg(all(
    feature = "download-resource-by-http",
    not(feature = "with-tls")
))]
fn is_https(url: &str) -> bool {
    url.get(..8)
        .map(|scheme| scheme.eq_ignore_ascii_case("https://"))
        .unwrap_or(false)
}
