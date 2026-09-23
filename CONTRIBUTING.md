# Contributing

The project lives at [github.com/wroom-tv/LiClone](https://github.com/wroom-tv/LiClone). These rules apply to every change that lands there.

## License first

You may use, read, and change LiClone for yourself or your organization. You may not publish the app, a fork, or a rebuilt installer unless Wroom agrees in writing. [LICENSE](LICENSE) is the full text.

A pull request gives Wroom the right to ship your contribution in LiClone. You keep ownership of what you wrote. Wroom may decline it.

## How a change gets in

1. Open an issue or a pull request against `main`. Describe what a person using LiClone will notice.
2. One pull request does one thing. A bug fix does not also restyle the app. A doc change does not also change mount behavior.
3. `main` stays releasable. If the check workflow is red, the change does not merge.
4. Do not push straight to `main` for a feature, a fix, or a release. A release is a tag `v1.2.3` on a commit that already matches that version.
5. Do not force-push `main`, and do not rewrite history on a branch someone else is reviewing.

## What you may edit

- `src/` for the window the person sees
- `src-tauri/src/` for what the app does on Windows
- `docs/`, `README.md`, `CHANGELOG.md`, `SECURITY.md`, and this file when the behavior or the rules change
- Installer images in `src-tauri/windows/` only to change how the installer looks

Leave these alone unless the change cannot work without them:

- `src/sealed/` — shared controls. A visual tweak belongs in `src/App.css` or the screen that uses them.
- `LICENSE` — only Wroom changes the license text
- Lockfiles, except when a dependency was added on purpose
- Generated installer output under `src-tauri/target/`

## What every code change includes

- The same version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`
- A `CHANGELOG.md` entry under that version, written as something a person will notice. Not a list of file names.
- No passwords, tokens, rclone config, or private mount paths
- Wording in the app and in the docs that speaks to the person using LiClone

## Before you open the pull request

```bash
npx tsc --noEmit
cargo check --manifest-path src-tauri/Cargo.toml
```

The GitHub check runs those, plus a version match, and it rejects a code change that does not update `CHANGELOG.md`.

## A useful bug report

Use the bug template. Include the LiClone version, the rclone version from the header, what you clicked, what you expected, and what happened. Leave secrets out.
