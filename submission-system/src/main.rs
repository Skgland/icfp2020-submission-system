use tempdir::TempDir;
use std::path::{PathBuf};
use git2::{Repository, BranchType};

use serde::Serialize;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{Display, Debug};
use serde::export::Formatter;
use git2::build::RepoBuilder;
use std::thread::sleep;
use std::time::Duration;


#[derive(Serialize, Deserialize)]
struct ConfigFile {
    repos: Vec<RepoSettings>,
    debug: bool
}

#[derive(Serialize, Deserialize)]
struct RepoSettings {
    clone_url: String,
    pull_key: String,
}

struct TestResult {}

fn main() -> Result<(), SetupError> {
    let config_content = std::fs::read_to_string("repositories.ron")?;

    //let str = ron::ser::to_string(&ConfigFile{repos:vec![RepoSettings{clone_url:"Test".into(), pull_key:"Test".into()}]})?;
    //println!("{}", str);

    let config: ConfigFile = ron::de::from_str(&config_content)?;


    for repo in config.repos.iter() {
        test(repo)?;
    }

    println!("Hello, world!");

    Ok(())
}
macro_rules! impl_from_for {
    ($from:ty => $to:ty as $var:ident) => {
        impl From<$from> for $to {
            fn from(error: $from) -> Self {
                <$to>::$var(error)
            }
        }
    };
}


#[derive(Debug)]
enum SetupError {
    GitError(git2::Error),
    IOError(std::io::Error),
    RonError(ron::Error),
    Utf8Erro(std::string::FromUtf8Error),
    ContainerBuildFailed,
}

impl_from_for!(git2::Error => SetupError as GitError);
impl_from_for!(std::io::Error => SetupError as IOError);
impl_from_for!(ron::Error => SetupError as RonError);
impl_from_for!(std::string::FromUtf8Error => SetupError as Utf8Erro);


impl Error for SetupError {}

impl Display for SetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

fn test(repo: &RepoSettings) -> Result<TestResult, SetupError> {
    let dir = TempDir::new("submission")?;

    let mut reo_builder = RepoBuilder::new();
    let repo = reo_builder.branch("submission").clone(&repo.clone_url, dir.path())?;

    println!("Cloned");
    println!("Checked out submission branch!");

    let platform = {
        let mut platform_path = PathBuf::from(dir.path());
        platform_path.push(".platform");

        println!("Path: {}", platform_path.display());
        std::fs::read_to_string(platform_path)?
    };

    println!("Using Platform {}", platform);

    let clean_dockerfile = format!("../dockerfiles/dockerfiles/{}/Dockerfile", platform);
    let repo_dockerfile = {
        let mut buf = PathBuf::from(dir.path());
        buf.push("Dockerfile");
        buf
    };

    std::fs::copy(clean_dockerfile, repo_dockerfile)?;

    println!("Copied Dockerfile");

    // setup container
    let out = std::process::Command::new("docker").arg("build").arg("--rm").arg("--quiet").arg("--network=none").arg(dir.path()).output()?;


    if !out.status.success() {
        Err(SetupError::ContainerBuildFailed)?
    }

    let id = {
        String::from_utf8(out.stdout)?.trim().to_string()
    };

    println!("Container build with Image Id {}!", id);

    let server = "localhost";
    let player = "player";

    // run test
    let result = std::process::Command::new("docker").arg("run").arg("--rm").arg(&id).arg(server).arg(player).output()?;

    let del_res = std::process::Command::new("docker").arg("rmi").arg(&id).output()?;

    if del_res.status.success() {
        println!("Deleted Container Image!");
    } else {
        eprintln!("Failed to delete Image!");
        println!("{}", String::from_utf8(del_res.stdout)?);
        eprintln!("{}", String::from_utf8(del_res.stderr)?);
    }

    if result.status.success() {
        // TODO statistics
        println!("Success");
        Ok(TestResult {})
    } else {
        // TODO negative Test result
        println!("Failure!");
        println!("{}", String::from_utf8(result.stdout)?);
        eprintln!("{}", String::from_utf8(result.stderr)?);
        Ok(TestResult {})
    }
}