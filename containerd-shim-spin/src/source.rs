use std::{fs::File, io::Write, path::PathBuf};

use anyhow::{Context, Result};
use containerd_shim_wasm::sandbox::context::RuntimeContext;
use log::info;
use oci_spec::image::MediaType;
use spin_app::locked::LockedApp;
use spin_loader::{cache::Cache, FilesMountStrategy};

use crate::{constants, utils::handle_archive_layer};

#[derive(Clone)]
pub enum Source {
    File(PathBuf),
    OciSpin,
    OciWkg(PathBuf),
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::File(path) => write!(f, "File({})", path.display()),
            Source::OciSpin => write!(f, "OciSpin"),
            Source::OciWkg(path) => write!(f, "OciWkg({})", path.display()),
        }
    }
}

impl Source {
    pub(crate) async fn from_ctx(ctx: &impl RuntimeContext, cache: &Cache) -> Result<Self> {
        match ctx.entrypoint().source {
            containerd_shim_wasm::sandbox::context::Source::File(_) => {
                Ok(Source::File(constants::SPIN_MANIFEST_FILE_PATH.into()))
            }
            containerd_shim_wasm::sandbox::context::Source::Oci(layers) => {
                info!(" >>> configuring spin oci application {}", layers.len());

                for layer in layers {
                    log::debug!("<<< layer config: {:?}", layer.config);
                }

                for artifact in layers {
                    match artifact.config.media_type() {
                        MediaType::Other(name)
                            if name == spin_oci::client::SPIN_APPLICATION_MEDIA_TYPE =>
                        {
                            let path = PathBuf::from("/spin.json");
                            log::info!("writing spin oci config to {path:?}");
                            File::create(&path)
                                .context("failed to create spin.json")?
                                .write_all(&artifact.layer)
                                .context("failed to write spin.json")?;
                        }
                        MediaType::Other(name) if name == constants::OCI_LAYER_MEDIA_TYPE_WASM => {
                            log::info!(
                                "<<< writing wasm artifact with length {:?} config to cache, near {:?}",
                                artifact.layer.len(),
                                cache.manifests_dir()
                            );
                            cache
                                .write_wasm(&artifact.layer, &artifact.config.digest())
                                .await?;
                        }
                        MediaType::Other(name)
                            if name == constants::OCI_LAYER_MEDIA_TYPE_WASM_WKG =>
                        {
                            log::info!(
                                "<<< writing wasm package with length {:?} config to cache, near {:?}",
                                artifact.layer.len(),
                                cache.manifests_dir()
                            );
                            cache
                                .write_wasm(&artifact.layer, &artifact.config.digest())
                                .await?;
                            if layers.len() != 1 {
                                anyhow::bail!(
                                    "expected single layer in OCI package, found {} layers",
                                    layers.len()
                                );
                            }
                            return Ok(Source::OciWkg(cache.wasm_path(artifact.config.digest())));
                        }
                        MediaType::Other(name) if name == spin_oci::client::DATA_MEDIATYPE => {
                            log::debug!(
                                "<<< writing data layer to cache, near {:?}",
                                cache.manifests_dir()
                            );
                            cache
                                .write_data(&artifact.layer, &artifact.config.digest())
                                .await?;
                        }
                        MediaType::Other(name) if name == spin_oci::client::ARCHIVE_MEDIATYPE => {
                            log::debug!(
                                "<<< writing archive layer and unpacking contents to cache, near {:?}",
                                cache.manifests_dir()
                            );
                            handle_archive_layer(cache, &artifact.layer, &artifact.config.digest())
                                .await
                                .context("unable to unpack archive layer")?;
                        }
                        _ => {
                            log::debug!(
                                "<<< unknown media type {:?}",
                                artifact.config.media_type()
                            );
                        }
                    }
                }
                Ok(Source::OciSpin)
            }
        }
    }

    pub(crate) async fn to_locked_app(&self, cache: &Cache) -> Result<LockedApp> {
        let locked_app = match self {
            Source::File(source) => {
                // TODO: This should be configurable, see https://github.com/deislabs/containerd-wasm-shims/issues/166
                // TODO: ^^ Move aforementioned issue to this repo
                let files_mount_strategy = FilesMountStrategy::Direct;
                spin_loader::from_file(&source, files_mount_strategy, None).await
            }
            Source::OciSpin => {
                let working_dir = PathBuf::from("/");
                let loader = spin_oci::OciLoader::new(working_dir);

                let locked_content = tokio::fs::read("/spin.json")
                    .await
                    .context("failed to read from \"/spin.json\"")?;
                let mut locked_app = LockedApp::from_json(&locked_content)
                    .context("failed to decode locked app from \"/spin.json\"")?;
                for component in &mut locked_app.components {
                    loader
                        .resolve_component_content_refs(component, cache)
                        .await
                        .with_context(|| {
                            format!("failed to resolve content for component {:?}", component.id)
                        })?;
                }
                Ok(locked_app)
            }
            Source::OciWkg(wasm_path) => spin_loader::from_wasm_file(&wasm_path)
                .await
                .with_context(|| format!("Failed to load component from {wasm_path:?}")),
        }?;
        Ok(locked_app)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        str::FromStr as _,
    };

    use containerd_shim_wasm::sandbox::context::{Entrypoint, RuntimeContext, WasmLayer};
    use oci_spec::image::{Descriptor, Digest, MediaType};
    use spin_loader::cache::Cache;
    use tempfile::{tempdir, TempDir};

    use super::*;
    use crate::constants;

    // A digest shared across helpers; only one wasm is written per cache in each test so
    // collisions are not a concern.
    const TEST_DIGEST: &str =
        "sha256:6c3c624b58dbbcd3c0dd82b4c53f04194d1247c6eebdaab7c610cf7d66709b3b";

    fn make_layer(media_type: &str, data: Vec<u8>) -> WasmLayer {
        WasmLayer {
            layer: data,
            config: Descriptor::new(
                MediaType::Other(media_type.to_string()),
                1024,
                Digest::from_str(TEST_DIGEST).unwrap(),
            ),
        }
    }

    async fn make_cache() -> (Cache, TempDir) {
        let dir = tempdir().unwrap();
        let cache = Cache::new(Some(dir.path().to_path_buf()))
            .await
            .expect("failed to create cache");
        (cache, dir)
    }

    // ── Mock RuntimeContext implementations ──────────────────────────────────

    struct MockFileContext;

    impl RuntimeContext for MockFileContext {
        fn args(&self) -> &[String] {
            &[]
        }
        fn envs(&self) -> &[String] {
            &[]
        }
        fn entrypoint(&self) -> Entrypoint<'_> {
            Entrypoint {
                func: "_start".to_string(),
                name: None,
                arg0: None,
                source: containerd_shim_wasm::sandbox::context::Source::File(PathBuf::from(
                    "/some/module.wasm",
                )),
            }
        }
    }

    struct MockOciContext {
        layers: Vec<WasmLayer>,
    }

    impl RuntimeContext for MockOciContext {
        fn args(&self) -> &[String] {
            &[]
        }
        fn envs(&self) -> &[String] {
            &[]
        }
        fn entrypoint(&self) -> Entrypoint<'_> {
            Entrypoint {
                func: "_start".to_string(),
                name: None,
                arg0: None,
                source: containerd_shim_wasm::sandbox::context::Source::Oci(&self.layers),
            }
        }
    }

    // ── from_ctx tests ───────────────────────────────────────────────────────

    /// A File source always resolves to the well-known spin.toml manifest path,
    /// regardless of what path the container spec provided.
    #[tokio::test]
    async fn from_ctx_file_source_returns_spin_manifest_path() {
        let ctx = MockFileContext;
        let (cache, _dir) = make_cache().await;

        let source = Source::from_ctx(&ctx, &cache)
            .await
            .expect("from_ctx failed");

        assert!(
            matches!(source.clone(), Source::File(path) if path == Path::new(constants::SPIN_MANIFEST_FILE_PATH)),
            "expected Source::File({:?}), got {:?}",
            constants::SPIN_MANIFEST_FILE_PATH,
            source
        );
    }

    /// An OCI image with no layers resolves to OciSpin (the loop body is simply
    /// never entered).
    #[tokio::test]
    async fn from_ctx_oci_empty_layers_returns_oci_spin() {
        let ctx = MockOciContext { layers: vec![] };
        let (cache, _dir) = make_cache().await;

        let source = Source::from_ctx(&ctx, &cache)
            .await
            .expect("from_ctx failed");

        assert!(matches!(source, Source::OciSpin));
    }

    /// Layers with unrecognised media types are silently skipped and the result
    /// is still OciSpin.
    #[tokio::test]
    async fn from_ctx_oci_unknown_media_type_returns_oci_spin() {
        let ctx = MockOciContext {
            layers: vec![make_layer("application/unknown+type", vec![])],
        };
        let (cache, _dir) = make_cache().await;

        let source = Source::from_ctx(&ctx, &cache)
            .await
            .expect("from_ctx failed");

        assert!(matches!(source, Source::OciSpin));
    }

    /// A standard Wasm layer (`application/vnd.wasm.content.layer.v1+wasm`) is
    /// written to cache and the source resolves to OciSpin.
    #[tokio::test]
    async fn from_ctx_oci_wasm_layer_writes_to_cache_and_returns_oci_spin() {
        let ctx = MockOciContext {
            layers: vec![make_layer(
                constants::OCI_LAYER_MEDIA_TYPE_WASM,
                // Minimal valid wasm binary header (magic + version)
                vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00],
            )],
        };
        let (cache, _dir) = make_cache().await;

        let source = Source::from_ctx(&ctx, &cache)
            .await
            .expect("from_ctx failed");

        assert!(matches!(source, Source::OciSpin));
        // Check that it was written to cache
        let expected_path = cache.wasm_path(Digest::from_str(TEST_DIGEST).unwrap());
        assert!(
            expected_path.exists(),
            "wasm not written to cache at expected path {:?}",
            expected_path
        );
    }

    /// A data layer (`application/vnd.spin.content.bytes.v1`) is written to
    /// cache and the source resolves to OciSpin.
    #[tokio::test]
    async fn from_ctx_oci_data_layer_writes_to_cache_and_returns_oci_spin() {
        let ctx = MockOciContext {
            layers: vec![make_layer(spin_oci::client::DATA_MEDIATYPE, vec![])],
        };
        let (cache, _dir) = make_cache().await;

        let source = Source::from_ctx(&ctx, &cache)
            .await
            .expect("from_ctx failed");

        assert!(matches!(source, Source::OciSpin));
    }

    /// A single `application/wasm` (wkg) layer is written to cache and the
    /// source resolves to `OciWkg` pointing at the cached file.
    #[tokio::test]
    async fn from_ctx_oci_wkg_single_layer_returns_oci_wkg_in_cache() {
        let ctx = MockOciContext {
            layers: vec![make_layer(
                constants::OCI_LAYER_MEDIA_TYPE_WASM_WKG,
                vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00],
            )],
        };
        let (cache, dir) = make_cache().await;

        let source = Source::from_ctx(&ctx, &cache)
            .await
            .expect("from_ctx failed");

        let Source::OciWkg(wasm_path) = source else {
            panic!("expected Source::OciWkg, got {:?}", source);
        };
        assert!(
            wasm_path.exists(),
            "wasm not written to cache at {wasm_path:?}"
        );
        assert!(
            wasm_path.starts_with(dir.path()),
            "wasm path {wasm_path:?} is not inside cache dir {:?}",
            dir.path()
        );
    }

    /// More than one `application/wasm` (wkg) layer is not a valid OCI package
    /// – the shim should return an error.
    #[tokio::test]
    async fn from_ctx_oci_wkg_multiple_layers_returns_error() {
        let layer = make_layer(
            constants::OCI_LAYER_MEDIA_TYPE_WASM_WKG,
            vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00],
        );
        let ctx = MockOciContext {
            layers: vec![layer.clone(), layer],
        };
        let (cache, _dir) = make_cache().await;

        let result = Source::from_ctx(&ctx, &cache).await;

        assert!(result.is_err(), "expected an error for multiple wkg layers");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("expected single layer"),
            "unexpected error message: {err}"
        );
    }

    // ── to_locked_app tests ──────────────────────────────────────────────────

    /// Missing manifest file produces a load error.
    #[tokio::test]
    async fn to_locked_app_file_not_found_returns_error() {
        let source = Source::File(PathBuf::from("/nonexistent/spin.toml"));
        let (cache, _dir) = make_cache().await;

        let result = source.to_locked_app(&cache).await;

        assert!(result.is_err(), "expected an error for missing spin.toml");
    }

    /// If `/spin.json` is absent the OciSpin path must return an error.
    /// The test skips if `/spin.json` happens to exist in the test environment.
    #[tokio::test]
    async fn to_locked_app_oci_spin_without_spin_json_returns_error() {
        if std::path::Path::new("/spin.json").exists() {
            // /spin.json exists in this environment; skip to avoid a false result.
            return;
        }
        let source = Source::OciSpin;
        let (cache, _dir) = make_cache().await;

        let result = source.to_locked_app(&cache).await;

        assert!(
            result.is_err(),
            "expected an error when /spin.json is absent"
        );
    }

    /// A valid wasm module written to a temp file can be loaded through the
    /// OciWkg path and produces a LockedApp.
    #[tokio::test]
    async fn to_locked_app_oci_wkg_valid_wasm_module_returns_locked_app() {
        let wasm = wat::parse_str("(module)").expect("failed to parse wasm module");
        let dir = tempdir().unwrap();
        let wasm_path = dir.path().join("test.wasm");
        std::fs::write(&wasm_path, &wasm).unwrap();

        let source = Source::OciWkg(wasm_path.clone());
        let (cache, _cache_dir) = make_cache().await;

        let result = source.to_locked_app(&cache).await;
        assert!(result.is_ok(), "to_locked_app failed: {:?}", result.err());
        // Assert that the LockedApp contains one component whose source URI
        // points at the wasm file (spin_loader encodes it as a file:// URI).
        let locked_app = result.unwrap();
        assert_eq!(locked_app.components.len(), 1);
        let component = &locked_app.components[0];
        let expected_uri = format!("file://{}", wasm_path.to_str().unwrap());
        assert_eq!(
            component.source.content.source.clone().unwrap(),
            expected_uri
        );
    }
}
