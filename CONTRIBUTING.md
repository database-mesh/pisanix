# Contributing to Pisanix

Thanks for your interest in contributing to Pisanix! This document outlines some of the conventions on building, running, and testing Pisa-Proxy, the development workflow, commit message formatting, contact points and other resources.

If you need any help or mentoring getting started, understanding the codebase, or making a PR (or anything else really), please ask on [Slack](https://databasemesh.slack.com/). If you don't know where to start, please click on the contributor icon below to get you on the right contributing path.

## First Pisanix Pull Request

### Prerequisites
To build Pisanix from scratch you will need to install the following tools:
* Git
* Rust install with [rustup](https://rustup.rs/)
* golang

### Pull Requests

#### Submit a PR
1. Fork the Pisanix repo and set remote repo.
      ```
      # git clone https://github.com/wbtlb/pisanix.git
      # cd pisanix

      # git remote add upstream https://github.com/database-mesh/pisanix.git

      # git remote -v
      origin	https://github.com/wbtlb/pisanix.git (fetch)
      origin	https://github.com/wbtlb/pisanix.git (push)
      upstream	https://github.com/database-mesh/pisanix.git (fetch)
      upstream	https://github.com/database-mesh/pisanix.git (push)
      ```
2. Open a regular issue for binding the pull request.
3. Submit a Draft Pull Requests, tag your work in progress.
4. Create your own branch and develop with it.Before developing, it's recommend to pull from remote repo to keep your repo latest. Now,you could develop at new branch.
      ```
      git checkout master
      git fetch upstream
      git rebase upstrea/master
      git checkout -b futures-0.1.0-dev
      ```
5. If you have added code that should be tested, add unit tests.
6. Verify and ensure that the test suites passes, make test.
7. Make sure your code passes both linters, make lint.
8. Submmit and push your code to the remote repo.
      ```
      git add <file>
      git commit -m 'commit log'
      git push origin futures-0.1.0-dev
      ```
9.  Change the status to “Ready for review”.

#### PR Template

#### PR Commit Message

Format: `<type>(<scope>): <subject>`

`<scope>` is optional

```
fix(functions): fix group by string bug
^--^  ^------------^
|     |
|     +-> Summary in present tense.
|
+-------> Type: chore, docs, feat, fix, refactor, style, or test.
```

More types:

* `feat`: new feature for the user.
* `fix`: bug fix for the user.
* `docs`: changes to the documentation.
* `style`: formatting, missing semi colons, etc; no production code change.
* `refactor`: refactoring production code, eg. renaming a variable.
* `test`: adding missing tests, refactoring tests; no production code change.
* `chore`: updating grunt tasks etc; no production code change.

### Issues
Pisanix uses [GitHub issues](https://github.com/database-mesh/pisanix/issues) to track bugs. Please include necessary information and instructions to reproduce your issue.
### Documentation

### Code of Conduct

### Roadmap
