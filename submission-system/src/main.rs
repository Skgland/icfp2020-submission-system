use tempdir::TempDir;
use std::path::{PathBuf};

use serde::Serialize;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{Display, Debug};
use serde::export::Formatter;
use git2::build::RepoBuilder;
use actix_web::{web, App, HttpServer, HttpResponse};
use std::net::SocketAddr;
use listenfd::ListenFd;
use std::sync::RwLock;


#[derive(Serialize, Deserialize)]
struct ConfigFile {
    repos: Vec<RepoSettings>,
    debug: bool,
}

#[derive(Serialize, Deserialize)]
struct RepoSettings {
    match_url: String,
    clone_url: String,
    deploy_token: String,
    deploy_user: String,
}

#[derive(Debug)]
enum TestResult {
    Success,
    RunError {
        run: Output
    },
    TestError {
        test: Output
    },
    RunTestError {
        run: Output,
        test: Output,
    },
}

#[derive(Debug)]
struct Output {
    stdout: String,
    stderr: String,
}

impl Display for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Stderr:\n")?;
        f.write_str(&self.stderr)?;
        f.write_str("\nStderr:\n")?;
        f.write_str(&self.stderr)?;
        f.write_str("\n")
    }
}

impl Display for TestResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Debug)]
struct TestLogEntry {
    repository: String,
    branch: String,
    result: TestLogResult,
}

#[derive(Debug)]
enum TestLogResult {
    Success,
    SetupError(SetupError),
    TestError {
        run_error_log: Option<Output>,
        test_error_log: Option<Output>,
    },

    InProgress,
}

impl Display for TestLogResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => {
                f.write_str("Success")
            }
            TestLogResult::SetupError(error) => {
                f.write_str("Setup Error:\n")?;
                Display::fmt(error, f)
            }
            TestLogResult::TestError { run_error_log, test_error_log } => {
                f.write_str("Test Error:\n")?;
                if let  Some(rel) = run_error_log {
                    Display::fmt(rel,f)?;
                    f.write_str("\n")?
                }
                if let Some(tel) = test_error_log {
                    Display::fmt(tel, f)?
                }
                Ok(())
            }
            TestLogResult::InProgress => {
                f.write_str("In Progress")
            }
        }
    }
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut listenfd = ListenFd::from_env();


    let config_content = std::fs::read_to_string("repositories.ron")?;

    let config: ConfigFile = ron::de::from_str(&config_content)?;


    let conf_data = web::Data::new(config);
    let result_data = web::Data::new(RwLock::new(Vec::<TestLogEntry>::new()));


    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(conf_data.clone())
            .app_data(result_data.clone())
            .service(
                web::resource("/submission").route(web::post().to(submission_handler))
            ).service(web::resource("/board").route(web::get().to(submision_lookup)))
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0)? {
        println!("Starting Server using TCPListener from listenfd.");
        server.listen(l)?
    } else {
        let sock_addr = SocketAddr::new([0, 0, 0, 0].into(), 80);

        println!("Starting Server on {}", sock_addr);

        server.bind(sock_addr)?
    };
    server.run().await?;

    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
struct RequestData {
    object_kind: String,
    #[serde(alias = "ref")]
    reference: String,
    repository: Repo,
}

#[derive(Deserialize, Serialize, Debug)]
struct Repo {
    git_ssh_url: String,
    git_http_url: String,
}

async fn submision_lookup(results: web::Data<RwLock<Vec<TestLogEntry>>>) -> HttpResponse {
    let guard = results.read().unwrap();

    let results: String = guard.iter().map(|entry| format!("
            <tr>
                <td>{}</td>
                <td>{}</td>
                <td><pre>{}</pre></td>
            </tr>", &entry.repository, &entry.branch, &entry.result)).collect();

    HttpResponse::Ok().body(format!("\
<html>
    <head>
        <meta charset='utf-8' />
    </head>
    <body>
        <h1> Test Results </h1>
        <table>
        <tr><th>Repo</th><th>Branch</th><th>Result</th></tr>
        {}
        </table>
    </body>
</html>
", results))
}

async fn submission_handler(form: web::Json<RequestData>, conf: web::Data<ConfigFile>, results: web::Data<RwLock<Vec<TestLogEntry>>>) -> Result<HttpResponse, actix_web::error::Error> {
    println!("{:?}", form);

    for rep in conf.repos.iter() {
        let branch = form.reference.replace("refs/heads/", "");

        if branch != "submission" && branch != "master" && !branch.starts_with("submissions/") {
            return Ok(HttpResponse::Ok().body("Skipping none master|submission branch"));
        }

        if form.repository.git_http_url == rep.match_url {
            let clone_url = rep.clone_url.replace("{username}", &rep.deploy_user).replace("{password}", &rep.deploy_token);
            let branche_clone = branch.clone();
            let match_clone = rep.match_url.clone();
            actix_rt::Arbiter::current().exec_fn(move || test_wrapper(&match_clone, &clone_url, &branche_clone, results.clone()));

            return Ok(HttpResponse::Ok().body("Running Test!"));
        }
    }

    Ok(HttpResponse::Ok().body(format!("Unknown Repository {}", form.repository.git_http_url)))
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
    ContainerBuildFailed(Output),
}

impl_from_for!(git2::Error => SetupError as GitError);
impl_from_for!(std::io::Error => SetupError as IOError);
impl_from_for!(ron::Error => SetupError as RonError);
impl_from_for!(std::string::FromUtf8Error => SetupError as Utf8Erro);


impl Error for SetupError {}

impl Display for SetupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::GitError(git_err) => {
                Display::fmt(git_err,f)
            },
            SetupError::IOError(io_err) => {
                Display::fmt(io_err,f)
            },
            SetupError::RonError(ron_err) => {
                Display::fmt(ron_err, f)
            },
            SetupError::Utf8Erro(utf8_error) => {
                Display::fmt(utf8_error, f)
            },
            SetupError::ContainerBuildFailed(cbf) => {
                Display::fmt(cbf, f)
            },
        }
    }
}

fn test_wrapper(match_url: &str, clone_url: &str, branch: &str, results: web::Data<RwLock<Vec<TestLogEntry>>>)
{
    let index = {
        let mut guard = results.write().unwrap();
        let len = guard.len();
        guard.push(TestLogEntry {
            repository: match_url.into(),
            branch: branch.into(),
            result: TestLogResult::InProgress,
        });
        len
    };

    match test(clone_url, branch) {
        Ok(result) => {
            results.write().unwrap().get_mut(index).map(|e| e.result = match result {
                TestResult::Success => {
                    TestLogResult::Success
                }
                TestResult::TestError {test} => {
                    TestLogResult::TestError{test_error_log: Some(test), run_error_log: None}
                }

                TestResult::RunError { run} => {
                    TestLogResult::TestError { test_error_log: None, run_error_log: Some(run) }}
                TestResult::RunTestError { run, test } => {
                    TestLogResult::TestError { test_error_log: Some(test), run_error_log: Some(run) }}
            });
        }
        Err(error) => {
            results.write().unwrap().get_mut(index).map(
                |e| e.result = TestLogResult::SetupError(error)
            );
        }
    }
}

fn test(clone_url: &str, branch: &str) -> Result<TestResult, SetupError> {
    let dir = TempDir::new("submission")?;

    let mut reo_builder = RepoBuilder::new();

    let _repo = reo_builder.branch(branch).clone(clone_url, dir.path())?;

    println!("Cloned");
    println!("Checked out {} branch!", branch);

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
        return Err(SetupError::ContainerBuildFailed(Output{stdout: String::from_utf8(out.stdout)?, stderr: String::from_utf8(out.stderr)? }))
    }

    let id = {
        String::from_utf8(out.stdout)?.trim().to_string()
    };

    println!("Container build with Image Id {}!", id);

    let server = "localhost";
    let player = "player";

    // run run.sh
    let result = std::process::Command::new("docker").arg("run").arg("--rm").arg(&id).arg(server).arg(player).output()?;

    // run test.sh
    let test_result = std::process::Command::new("docker").arg("run").arg("--rm").arg("--entrypoint").arg("./test.sh").arg(&id).output()?;

    let del_res = std::process::Command::new("docker").arg("rmi").arg(&id).output()?;

    if del_res.status.success() {
        println!("Deleted Container Image!");
    } else {
        eprintln!("Failed to delete Image!");
        println!("{}", String::from_utf8(del_res.stdout)?);
        eprintln!("{}", String::from_utf8(del_res.stderr)?);
    }

    match (result.status.success(), test_result.status.success()) {
        (true, true) => {
            println!("Success");
            Ok(TestResult::Success)
        }
        (false, false) => {
            println!("Run and Test failed!");
            Ok(TestResult::RunTestError {
                run: Output { stdout: String::from_utf8(result.stdout)?, stderr: String::from_utf8(result.stderr)? },
                test: Output { stdout: String::from_utf8(test_result.stdout)?, stderr: String::from_utf8(test_result.stderr)?},
            })
        }
        (false, _) => {
            println!("Run failed!");
            Ok(TestResult::RunError{
                run: Output { stdout: String::from_utf8(result.stdout)?, stderr: String::from_utf8(result.stderr)? },
            })
        }
        (_, false) => {
            println!("Test failed!");
            Ok(TestResult::TestError{
                test: Output { stdout: String::from_utf8(test_result.stdout)?, stderr: String::from_utf8(test_result.stderr)? },
            })
        }
    }
}