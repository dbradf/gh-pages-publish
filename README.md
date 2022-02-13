# gh-pages-publish

A tool to publish documentation to github pages.

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/dbradf/gh-pages-publish)](https://github.com/dbradf/gh-pages-publish/releases/latest)


## Usage

```bash
gh-pages-publish --docs-dir path/to/docs
```

```
$ gh-pages-publish --help
gh-pages-publish 0.1.0
Publish documentation to github pages

USAGE:
    gh-pages-publish [OPTIONS] --docs-dir <DOCS_DIR>

OPTIONS:
        --docs-dir <DOCS_DIR>              Directory containing built documentation
        --git-binary <GIT_BINARY>          Location of git binary
    -h, --help                             Print help information
        --repo-base <REPO_BASE>            Location of base of repository to publish to [default: .]
        --target-branch <TARGET_BRANCH>    Branch to publish to [default: gh-pages]
    -V, --version                          Print version information
```
