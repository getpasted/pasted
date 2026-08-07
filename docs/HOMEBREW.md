# Homebrew distribution

Pasted's tagged desktop release publishes a `pasted.rb` Cask alongside its signed and notarized universal DMG. The Cask installs `Pasted.app` and exposes its bundled `pasted` command through Homebrew's binary directory.

## Initial tap

Use a separate public repository named `JJJ/homebrew-tap`. It contains distribution metadata only; application source and release credentials remain in this repository and GitHub Environment secrets.

Users can then install Pasted with either:

```sh
brew install --cask JJJ/tap/pasted
```

or:

```sh
brew tap JJJ/tap
brew install --cask pasted
```

Give the tap repository this structure:

```text
Casks/pasted.rb
.github/workflows/update-pasted.yml
README.md
```

The updater should run on a schedule and by manual dispatch. It downloads the Cask from the latest public Pasted release, commits only when its contents changed, and pushes with the tap repository's built-in `GITHUB_TOKEN`:

```yaml
name: Update Pasted

on:
  workflow_dispatch:
  schedule:
    - cron: '23 */6 * * *'

permissions:
  contents: write

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - name: Fetch latest Cask
        run: |
          mkdir -p Casks
          curl --fail --silent --show-error --location \
            https://github.com/JJJ/Pasted/releases/latest/download/pasted.rb \
            --output Casks/pasted.rb
      - name: Commit an updated Cask
        run: |
          if git diff --quiet -- Casks/pasted.rb; then
            exit 0
          fi
          git config user.name github-actions[bot]
          git config user.email 41898282+github-actions[bot]@users.noreply.github.com
          git add Casks/pasted.rb
          git commit -m "Update Pasted Cask"
          git push
```

This pull model intentionally requires no personal access token, deploy key, Apple credential, or cross-repository secret. A newly published GitHub Release becomes available through the tap within six hours; manually running the updater makes it immediate.

## Official Homebrew Cask

After Pasted has a stable public release history and visible usage, submit the same Cask to `Homebrew/homebrew-cask`. Acceptance would shorten installation to:

```sh
brew install --cask pasted
```

Keep `JJJ/homebrew-tap` available until the official Cask is established, then point existing users toward the official package.
