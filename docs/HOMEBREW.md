# Homebrew distribution

Pasted's tagged desktop release publishes a `pasted.rb` Cask alongside its signed and notarized universal DMG. The Cask installs `Pasted.app` and exposes its bundled `pasted` command through Homebrew's binary directory.

## Initial tap

Use the separate public repository `getpasted/homebrew-tap`. It contains distribution metadata only; application source and release credentials remain in this repository and GitHub Environment secrets.

Users can then install Pasted with either:

```sh
brew install --cask getpasted/tap/pasted
```

or:

```sh
brew tap getpasted/tap
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
      - name: Fetch latest published Cask
        id: fetch
        run: |
          cask_path="$RUNNER_TEMP/pasted.rb"
          if ! curl --fail --silent --show-error --location \
            https://github.com/getpasted/pasted/releases/latest/download/pasted.rb \
            --output "$cask_path"; then
            echo '::notice::Pasted has no public release Cask yet; nothing to update.'
            echo 'available=false' >> "$GITHUB_OUTPUT"
            exit 0
          fi
          ruby -c "$cask_path"
          mkdir -p Casks
          cp "$cask_path" Casks/pasted.rb
          echo 'available=true' >> "$GITHUB_OUTPUT"
      - name: Commit an updated Cask
        if: steps.fetch.outputs.available == 'true'
        run: |
          if [[ -z "$(git status --porcelain -- Casks/pasted.rb)" ]]; then
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

Keep `getpasted/homebrew-tap` available until the official Cask is established, then point existing users toward the official package.
