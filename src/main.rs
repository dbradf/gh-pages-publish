use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
};

use anyhow::Result;
use clap::Parser;
use cmd_lib::{run_cmd, run_fun};
use tracing::{event, Level};
use which::which;

/// Publish documentation to github pages.
#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    /// Branch to publish to.
    #[clap(long, default_value_t = String::from("gh-pages"))]
    target_branch: String,
    /// Directory containing built documentation.
    #[clap(long, parse(from_os_str))]
    docs_dir: PathBuf,
    /// Location of git binary.
    #[clap(long, parse(from_os_str))]
    git_binary: Option<PathBuf>,
    /// Location of base of repository to publish to.
    #[clap(long, parse(from_os_str), default_value = ".")]
    repo_base: PathBuf,
    /// Enable verbose logging.
    #[clap(long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    let default_git = which("git");
    if default_git.is_err() && args.git_binary.is_none() {
        eprintln!("ERROR: Could not find 'git' binary.");
        eprintln!("Please ensure 'git' is available in your PATH or provide it via --git-binary");
        exit(1);
    }

    if args.verbose {
        tracing_subscriber::fmt::init();
    }

    let git_service = GitService {
        git_binary: args.git_binary.unwrap_or_else(|| default_git.unwrap()),
    };
    std::env::set_current_dir(&args.repo_base).unwrap();
    if let Err(err) = run(&git_service, &args.target_branch, &args.docs_dir) {
        println!("ERROR: {}", err);
    }
}

#[derive(Debug)]
struct CommitMetaData {
    author: String,
    email: String,
    message: String,
}

impl CommitMetaData {
    pub fn from_git_history(commit_str: &str) -> Self {
        let parts: Vec<&str> = commit_str.split(':').collect();
        Self {
            author: parts[0].to_string(),
            email: parts[1].to_string(),
            message: parts[2..].join(":"),
        }
    }

    pub fn author_string(&self) -> String {
        format!("{} <{}>", self.author, self.email)
    }
}

struct GitService {
    git_binary: PathBuf,
}

impl GitService {
    pub fn active_branch(&self) -> Result<String> {
        let git_binary = &self.git_binary;
        Ok(run_fun!(
            ${git_binary} rev-parse --abbrev-ref "HEAD"
        )?)
    }

    pub fn get_last_commit(&self) -> Result<CommitMetaData> {
        let git_binary = &self.git_binary;
        let format = r"%an:%ae:%s";
        let output = run_fun!(
            ${git_binary} log -n 1 --pretty=format:"${format}"
        )?;

        Ok(CommitMetaData::from_git_history(&output))
    }

    pub fn switch_branch(&self, branch: &str) -> Result<()> {
        let git_binary = &self.git_binary;
        run_fun!(
            ${git_binary} checkout ${branch}
        )?;
        Ok(())
    }

    pub fn changes_exist(&self) -> Result<bool> {
        let git_binary = &self.git_binary;
        let output = run_fun!(
            ${git_binary} status --short
        )?;

        Ok(!output.trim().is_empty())
    }

    pub fn add(&self, filespec: &str) -> Result<()> {
        let git_binary = &self.git_binary;
        run_fun!(
            ${git_binary} add ${filespec}
        )?;
        Ok(())
    }

    pub fn commit(&self, commit_metadata: &CommitMetaData) -> Result<()> {
        let git_binary = &self.git_binary;
        let user = &commit_metadata.author;
        let email = &commit_metadata.email;
        let message = &commit_metadata.message;
        let author = &commit_metadata.author_string();
        run_cmd!(
            ${git_binary} -c user.name="${user}" -c user.email="${email}" commit -m "${message}" --author "${author}"
        )?;
        Ok(())
    }

    pub fn push_branch(&self, branch: &str, commit_metadata: &CommitMetaData) -> Result<()> {
        let git_binary = &self.git_binary;
        let user = &commit_metadata.author;
        let email = &commit_metadata.email;
        run_fun!(
            ${git_binary} -c user.name="${user}" -c user.email="${email}" push origin ${branch}
        )?;

        Ok(())
    }
}

fn run(git_service: &GitService, target_branch: &str, build_dir: &Path) -> Result<()> {
    let active_branch = git_service.active_branch()?;
    let result = publish_branch(git_service, target_branch, build_dir);
    if result.is_err() {
        event!(
            Level::WARN,
            "Encountered error during execution: {:?}",
            err = result
        );
    }
    event!(Level::INFO, "Switching back to previously active branch");
    git_service.switch_branch(&active_branch)?;

    result
}

fn publish_branch(git_service: &GitService, target_branch: &str, build_dir: &Path) -> Result<()> {
    // get commit-data
    event!(Level::INFO, "Details details about last commit");
    let last_commit = git_service.get_last_commit()?;
    // switch branch
    event!(Level::INFO, "Switching to target branch");
    git_service.switch_branch(target_branch)?;

    // move docs to root
    event!(Level::INFO, "Moving files to repo root");
    let target = PathBuf::from(".").canonicalize()?;
    let files = fs::read_dir(build_dir)?;
    for entry in files {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let mut target_path = target.clone();
        target_path.push(file_name);

        if target_path.exists() {
            if target_path.is_dir() {
                fs::remove_dir_all(&target_path)?;
            } else {
                fs::remove_file(&target_path)?;
            }
        }

        fs::rename(path, target_path)?;
    }
    // if any changes
    event!(Level::INFO, "Checking if any changes exist");
    if git_service.changes_exist()? {
        //   create commit
        event!(Level::INFO, "Adding changed files to git");
        git_service.add(".")?;
        event!(Level::INFO, "Committing changed files to git");
        git_service.commit(&last_commit)?;

        //   push branch
        event!(Level::INFO, "Publishing branch to origin");
        git_service.push_branch(target_branch, &last_commit)?;
    }

    Ok(())
}
