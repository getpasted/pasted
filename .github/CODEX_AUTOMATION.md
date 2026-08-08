# Codex issue pipeline

Pasted can turn an approved GitHub issue into a reviewable draft pull request without giving Codex unattended access to repository or organization administration.

## One-time setup

1. Add an `OPENAI_API_KEY` Actions secret to `getpasted/pasted` (or expose an organization secret to this repository).
2. In **Settings → Actions → General**, allow workflows to read and write repository contents and allow GitHub Actions to create pull requests.
3. Connect `getpasted/pasted` to Codex cloud and optionally enable automatic pull-request reviews in Codex settings.

Secrets are stored in GitHub, never committed to this repository.

## Normal flow

1. Create or refine an issue until its outcome and acceptance criteria are implementation-ready.
2. Apply `codex: ready`. Only a repository writer should apply this label.
3. The workflow replaces it with `codex: working`, checks out a credential-free copy, and runs the official `openai/codex-action` with workspace-only permissions.
4. Protected automation files are rejected. A successful change is pushed to a `codex/issue-*` branch and opened as a **draft** pull request.
5. The issue moves to `codex: review`; existing pull-request CI validates the change before a human chooses whether to merge it.

Use **Actions → Codex issue implementation → Run workflow** with an issue number to retry deliberately. A failed or no-change run receives `codex: blocked`; update the issue and reapply `codex: ready` when it is safe to try again.

## Security boundary

- Public issue text is untrusted. Label application by a repository writer is the approval gate.
- The checkout does not persist GitHub credentials while Codex runs.
- Codex cannot change workflows, local actions, or `AGENTS.md` through this pipeline.
- Codex produces a draft pull request; it cannot merge its own work.
- This workflow does not wake a specific local Codex desktop task. It runs Codex in GitHub Actions, while connected Codex cloud can automatically review the resulting pull request.
