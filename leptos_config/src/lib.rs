use std::{env::VarError, str::FromStr};
use typed_builder::TypedBuilder;

#[derive(Default, TypedBuilder, Clone)]
pub struct RenderOptions {
    #[builder(setter(into))]
    pub pkg_path: String,
    #[builder(setter(into))]
    pub environment: RustEnv,
    #[builder(setter(strip_option), default)]
    pub reload_port: Option<u32>,
}

impl RenderOptions {
    /// Creates a hidden file at ./.leptos_toml so cargo-leptos can monitor settings
    pub fn write_to_file(&self) {
        use std::fs;
        let options = format!(
            r#"render_options: {{
    pkg_path {}
    environment {:?}
    reload_port {:?}
}}
"#,
            self.pkg_path, self.environment, self.reload_port
        );
        fs::write("./.leptos.kdl", options).expect("Unable to write file");
    }
}
#[derive(Default, Debug, Clone)]
pub enum RustEnv {
    #[default]
    PROD,
    DEV,
}

impl FromStr for RustEnv {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let sanitized = input.to_lowercase();
        match sanitized.as_ref() {
            "dev" => Ok(Self::DEV),
            "development" => Ok(Self::DEV),
            "prod" => Ok(Self::PROD),
            "production" => Ok(Self::PROD),
            _ => Ok(Self::PROD),
        }
    }
}

impl From<&str> for RustEnv {
    fn from(str: &str) -> Self {
        let sanitized = str.to_lowercase();
        match sanitized.as_str() {
            "dev" => Self::DEV,
            "development" => Self::DEV,
            "prod" => Self::PROD,
            "production" => Self::PROD,
            _ => {
                panic!("Environment var is not recognized. Maybe try `dev` or `prod`")
            }
        }
    }
}
impl From<&Result<String, VarError>> for RustEnv {
    fn from(input: &Result<String, VarError>) -> Self {
        match input {
            Ok(str) => {
                let sanitized = str.to_lowercase();
                match sanitized.as_ref() {
                    "dev" => Self::DEV,
                    "development" => Self::DEV,
                    "prod" => Self::PROD,
                    "production" => Self::PROD,
                    _ => {
                        panic!("Environment var is not recognized. Maybe try `dev` or `prod`")
                    }
                }
            }
            Err(_) => Self::PROD,
        }
    }
}
