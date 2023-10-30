use std::path::PathBuf;
use rspack_core::PluginExt;
use rspack_testing::{fixture,test_fixture_insta};
use plugin_manifest::ManifestPlugin;

#[fixture("tests/fixtures/*")]
fn manifest(fixture_path: PathBuf) {
  test_fixture_insta(
    &fixture_path,
    // TODO: check the specific file output.
    &|_| false,
    Box::new(|plugins, _| {
      plugins.push(ManifestPlugin::new().boxed())
    })
  );
}