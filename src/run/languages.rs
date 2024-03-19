use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
/// Specifies a configuration for a language.
pub struct LanguageConfig {
    /// Name of the language
    pub name: String,
    #[serde(rename(serialize = "tablerIcon"))]
    /// Name of the icon for the language in [tabler icons](https://tabler.io/icons)
    pub tabler_icon: String,
    #[serde(rename(serialize = "monacoContribution"))]
    /// Name of the monaco contribution for the language
    pub monaco_contribution: String,
    #[serde(rename(serialize = "defaultCode"))]
    /// Default code to show in the editor
    pub default_code: String,
    #[serde(rename(serialize = "fileName"))]
    /// Name of the file to save user submitted code to
    pub file_name: String,
    #[serde(skip_serializing)]
    /// Command to compile the program.
    pub compile_cmd: String,
    #[serde(skip_serializing)]
    /// Command to run the program. This will be passed the case's input in stdin
    pub run_cmd: String,
}

#[derive(Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct RunConfig {
    /// Languages that are supported by the runner
    pub languages: HashMap<String, LanguageConfig>,
    /// Default language to use
    pub default_language: String,
}
