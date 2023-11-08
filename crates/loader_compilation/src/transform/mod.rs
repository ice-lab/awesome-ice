use std::path::Path;
use anyhow::{Error, Context};
use either::Either;
use serde::{Deserialize, Serialize};
use swc_core::common::chain;
use swc_core::ecma::{
  transforms::base::pass::noop,
  visit::{as_folder, Fold, VisitMut, Visit},
  ast::Module,
};

mod keep_export;
mod remove_export;

use keep_export::keep_export;
use remove_export::remove_export;

macro_rules! either {
  ($config:expr, $f:expr) => {
    if let Some(config) = &$config {
      Either::Left($f(config))
    } else {
      Either::Right(noop())
    }
  };
  ($config:expr, $f:expr, $enabled:expr) => {
    if $enabled() {
      either!($config, $f)
    } else {
      Either::Right(noop())
    }
  };
}

// Only define the stuct which is used in the following function.
#[derive(Deserialize, Debug)]
struct NestedRoutesManifest {
  file: String,
  children: Option<Vec<NestedRoutesManifest>>,
}

fn get_routes_file(routes: Vec<NestedRoutesManifest>) -> Vec<String> {
  let mut result: Vec<String> = vec![];
  for route in routes {
    // Add default prefix of src/pages/ to the route file.
    let mut path_str = String::from("src/pages/");
    path_str.push_str(&route.file);

    result.push(path_str.to_string());

    if let Some(children) = route.children {
      result.append(&mut get_routes_file(children));
    }
  }
  result
}

fn parse_routes_config(c: String) -> Result<Vec<String>, Error> {
  let routes = serde_json::from_str(&c)?;
  Ok(get_routes_file(routes))
}

pub(crate) fn load_routes_config(path: &Path) -> Result<Vec<String>, Error> {
  let content = std::fs::read_to_string(path).context("failed to read routes config")?;
  parse_routes_config(content)
}

fn match_route_entry(resource_path: &Path, routes: &Vec<String>) -> bool {
  let resource_path_str = resource_path.to_str().unwrap();
  for route in routes {
    if resource_path_str.ends_with(&route.to_string()) {
      return true;
    }
  }
  false
}

fn match_app_entry(resource_path: &Path) -> bool {
  let resource_path_str = resource_path.to_str().unwrap();
  // File path ends with src/app.(ts|tsx|js|jsx)
  let regex_for_app = regex::Regex::new(r"src/app\.(ts|tsx|js|jsx)$").unwrap();
  regex_for_app.is_match(resource_path_str)
}

#[derive(Default, Debug, Clone)]
pub struct KeepExportOptions {
  pub export_names: Vec<String>,
}

#[derive(Default, Debug, Clone)]
pub struct RemoveExportOptions {
  pub remove_names: Vec<String>,
}

#[derive(Default, Debug)]
pub struct TransformFeatureOptions {
  pub keep_export: Option<KeepExportOptions>,
  pub remove_export: Option<RemoveExportOptions>,
}

pub(crate) fn transform<'a>(
  resource_path: &'a Path,
  context: &str,
  routes_config: &Vec<String>,
  feature_options: &TransformFeatureOptions,
) -> impl Fold + 'a {
  chain!(
    either!(feature_options.keep_export, |options: &KeepExportOptions| {
      let mut exports_name = options.export_names.clone();
      // Special case for app entry.
      // When keep pageConfig, we should also keep the default export of app entry.
      if match_app_entry(resource_path) && exports_name.contains(&String::from("pageConfig")) {
        exports_name.push(String::from("default"));
      }
      keep_export(exports_name)
    }, || {
      match_app_entry(resource_path) || match_route_entry(resource_path, routes_config)
    }),
    either!(feature_options.remove_export, |options: &RemoveExportOptions| {
      remove_export(options.remove_names.clone())
    }, || {
      // Remove export only work for app entry and route entry.
      match_app_entry(resource_path) || match_route_entry(resource_path, routes_config)
    }),
  )
}