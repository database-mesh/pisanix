# Contributing to Pisanix

Thanks for your interest in contributing to Pisanix! This document outlines some of the conventions on building, running, and testing Pisanix, the development workflow, commit message formatting, contact points and other resources.

If you need any help or mentoring getting started, understanding the codebase, or making a PR (or anything else really), please ask on [Slack](https://databasemesh.slack.com/). If you don't know where to start, please click on the contributor icon below to get you on the right contributing path.

You can choose from the following two projects to contribute to the one you are interested in.

* [Pisa-Proxy](https://github.com/database-mesh/pisanix/blob/master/pisa-proxy/CONTRIBUTING.md)
* [Pisa-Controller](https://github.com/database-mesh/pisanix/blob/master/pisa-controller/CONTRIBUTING.md)

# First Pisanix Pull Request

## Prerequisites

To build Pisanix from scratch you will need to install the following tools:

* Git
* Rust 1.16 install with [rustup](https://rustup.rs/)
* [Golang 1.16](https://golang.org/dl/)
* make

## Pull Requests

### Submit a PR
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
      git rebase upstream/master
      git checkout -b futures-0.1.0-dev
      ```
5. If you have added code that should be tested, add unit tests.
6. Verify and ensure that the test suites passes, make test.
7. Make sure your code passes both linters, make lint.
8. Submit and push your code to the remote repo.
      ```
      git add <file>
      git commit -m 'commit log'
      git push origin futures-0.1.0-dev
      ```
9.  Change the status to “Ready for review”.

### PR Template

```
<!--
Thank you for contributing to Pisanix!

If you haven't already, please read Pisanix's [CONTRIBUTING](../CONTRIBUTING.md) document.

If you're unsure about anything, just ask; somebody should be along to answer within a day or two.

PR Title Format:
1. module [, module2, module3]: what's changed
2. *: what's changed
-->

### What is changed and how it works?

Close #xxx <!-- Associate issue that describes the problem the PR tries to solve. -->

What's Changed:

### Related changes

- PR to update `pisanix/docs/cn`:
- PR to update `pisanix/protocol`:
- Need to cherry-pick to the release branch

### Check List <!--REMOVE the items that are not applicable-->

Tests <!-- At least one of them must be included. -->

- Unit test
- Integration test
- Manual test (add detailed scripts or steps below)
- No code

Side effects

- Performance regression
    - Consumes more CPU
    - Consumes more MEM
- Breaking backward compatibility

### Release note <!-- bugfixes or new feature need a release note -->

release-note
Please add a release note.
If you don't think this PR needs a release note then fill it with None.
If this PR will be picked to release branch, then a release note is probably required.


```

### PR Commit Message

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

## Issues
Pisanix uses [GitHub issues](https://github.com/database-mesh/pisanix/issues) to track bugs. Please include necessary information and instructions to reproduce your issue.

## Code of Conduct
Please refer to the [CNCF Code of Conduct](https://github.com/cncf/foundation/blob/master/code-of-conduct.md), which describes the expectations for interactions within the community. 
