# Pisanix Website 

The Pisanix website is built using [Docusaurus 2](https://docusaurus.io/), a modern static website generator.


## New Documents

1. Create a new document under directory `docs`.
2. Add same name file under `i18n/docusaurus-plugin-content-docs/current` if it is translated into English.

NOTE: Currently two collections: Features and UseCases.

## Build steps

1. Using `npm run build` to make all the static files available at `build`.
2. Copy files under `build` to another repo `pisanix.io` and overwrite all existing files.
3. Commit all changes in `pisanix.io` then push them to branch `gh-pages`.
4. Refresh website `https://www.pisanix.io` and get the latest version.
